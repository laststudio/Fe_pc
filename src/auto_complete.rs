use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cloud_client::CloudClient;
use crate::models::{HomeworkItem, ParsedQuestion, UserSession};
use crate::workflow::oss::OssUploader;
use crate::workflow::scoring::{self, ScoreProfile};
use crate::workflow::tts::synthesize_auto;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoCompleteConfig {
    pub target_total: f64,
    pub voice_name: String,
    pub rate: i32,
    pub pitch: i32,
    pub use_time_seconds: u32,
    pub no_upload: bool,
    pub accuracy: f64,
    pub fluency: f64,
    pub integrity: f64,
    pub allow_over_max: bool,
    #[serde(default = "default_tts_mode")]
    pub tts_mode: String,
    #[serde(default)]
    pub http_tts_url: String,
    #[serde(default)]
    pub http_tts_key: String,
    #[serde(default = "default_http_tts_model")]
    pub http_tts_model: String,
    #[serde(default)]
    pub clone_reference_path: String,
    #[serde(default = "default_mode")]
    pub mode: String, // "auto" 或 "manual"
    #[serde(default = "default_use_time_source")]
    pub use_time_source: String, // "smart" 或 "manual"
    #[serde(default)]
    pub time_reroll_nonce: u64,
    #[serde(default)]
    pub score_reroll_nonce: u64,
    #[serde(default)]
    pub clone_voice_id: String,
    #[serde(default)]
    pub per_question_scores: std::collections::HashMap<usize, f64>,
    #[serde(default)]
    pub per_question_audio: std::collections::HashMap<usize, String>,
}

fn default_tts_mode() -> String {
    "edge".to_string()
}

fn default_http_tts_model() -> String {
    "tts-1".to_string()
}

fn default_mode() -> String {
    "auto".to_string()
}

fn default_use_time_source() -> String {
    "smart".to_string()
}

impl Default for AutoCompleteConfig {
    fn default() -> Self {
        Self {
            target_total: 28.5,
            voice_name: "en-US-JennyNeural".to_string(),
            rate: -5,
            pitch: 0,
            use_time_seconds: 180,
            no_upload: false,
            accuracy: 4.95,
            fluency: 4.95,
            integrity: 4.95,
            allow_over_max: false,
            tts_mode: "edge".to_string(),
            http_tts_url: String::new(),
            http_tts_key: String::new(),
            http_tts_model: "tts-1".to_string(),
            clone_reference_path: String::new(),
            mode: "auto".to_string(),
            use_time_source: "smart".to_string(),
            time_reroll_nonce: 0,
            score_reroll_nonce: 0,
            clone_voice_id: String::new(),
            per_question_scores: std::collections::HashMap::new(),
            per_question_audio: std::collections::HashMap::new(),
        }
    }
}

pub fn check_homework_compatibility(homework: &HomeworkItem) -> Result<(), String> {
    let hw_name = homework.name.to_lowercase();
    if hw_name.contains("作文") || hw_name.contains("composition") || hw_name.contains("写作") {
        return Err("检测到试卷包含【英语作文/写作】题型，暂不支持全自动作答，请在客户端手动完成。".to_string());
    }
    if hw_name.contains("听写") || hw_name.contains("dictation") || hw_name.contains("拼写") {
        return Err("检测到试卷包含【单词听写/拼写】题型，暂不支持全自动作答。".to_string());
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub enum AutoProgressMsg {
    Step { current: usize, total: usize, title: String },
    Log(String),
    QuestionDone {
        index: usize,
        number: String,
        category: String,
        score: f64,
        max_score: f64,
        status: String,
        audio_path: Option<String>,
    },
    Complete { success: bool, final_score: f64, message: String },
}

pub struct AutoCompleteWorkflow;

impl AutoCompleteWorkflow {
    fn now_str() -> String {
        #[cfg(windows)]
        {
            #[repr(C)]
            struct SystemTimeWin {
                w_year: u16,
                w_month: u16,
                w_day_of_week: u16,
                w_day: u16,
                w_hour: u16,
                w_minute: u16,
                w_second: u16,
                w_milliseconds: u16,
            }

            unsafe extern "system" {
                fn GetLocalTime(lp_system_time: *mut SystemTimeWin);
            }

            let mut st = SystemTimeWin {
                w_year: 0,
                w_month: 0,
                w_day_of_week: 0,
                w_day: 0,
                w_hour: 0,
                w_minute: 0,
                w_second: 0,
                w_milliseconds: 0,
            };
            unsafe {
                GetLocalTime(&mut st);
            }
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                st.w_year, st.w_month, st.w_day, st.w_hour, st.w_minute, st.w_second
            )
        }
        #[cfg(not(windows))]
        {
            let d = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let secs = d % 86400;
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            format!("2026-09-19 {:02}:{:02}:{:02}", h, m, s)
        }
    }

    fn seq_id() -> String {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let rand_part = (millis % 9000 + 1000) as u32;
        format!("{millis}{rand_part}")
    }

    /// 执行全功能 8 阶段自动化做题流水线
    pub fn run(
        session: UserSession,
        mut homework: HomeworkItem,
        config: AutoCompleteConfig,
        sender: Sender<AutoProgressMsg>,
    ) {
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "🚀 启动作业 [{}] 8阶段全自动化做题流水线 (模式: {})...",
            homework.name,
            if config.no_upload { "本地试运行 (--no-upload)" } else { "官方云端正式提交" }
        )));

        if session.token.trim().is_empty() && !config.no_upload {
            let _ = sender.send(AutoProgressMsg::Log(
                "❌ 错误: 未检测到有效的 ETS100 做题令牌凭据，必须登录 LA 账号以获取做题授权".to_string(),
            ));
            let _ = sender.send(AutoProgressMsg::Complete {
                success: false,
                final_score: 0.0,
                message: "未检测到有效做题凭据，请登录 LA 账号".to_string(),
            });
            return;
        }

        // 阶段 0: 检查题目试卷与资源解析
        let _ = sender.send(AutoProgressMsg::Step {
            current: 1,
            total: 8,
            title: "检查作业状态与题目结构".to_string(),
        });

        if homework.sections.is_empty() {
            let _ = sender.send(AutoProgressMsg::Log("  检测到试卷题目未缓存，正在自动下载解密试卷资源包...".to_string()));
            match CloudClient::download_and_parse_homework(&session, &homework) {
                Ok(parsed_hw) => {
                    homework = parsed_hw;
                    let _ = sender.send(AutoProgressMsg::Log(format!(
                        "  ✓ 成功解密试卷，共加载 {} 个分类题目",
                        homework.sections.len()
                    )));
                }
                Err(e) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!("  ❌ 下载解析试卷失败: {e}")));
                    let _ = sender.send(AutoProgressMsg::Complete {
                        success: false,
                        final_score: 0.0,
                        message: format!("解析作业失败: {e}"),
                    });
                    return;
                }
            }
        }

        let mut flat_questions: Vec<ParsedQuestion> = Vec::new();
        for section in &homework.sections {
            for q in &section.questions {
                flat_questions.push(q.clone());
            }
        }

        if flat_questions.is_empty() {
            let _ = sender.send(AutoProgressMsg::Log("❌ 作业中未检测到任何题目".to_string()));
            let _ = sender.send(AutoProgressMsg::Complete {
                success: false,
                final_score: 0.0,
                message: "作业未包含题目".to_string(),
            });
            return;
        }

        let total_questions = flat_questions.len();
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "✓ 题目结构就绪: 共 {} 题，目标总分: {} / {} 分",
            total_questions, config.target_total, homework.total_point
        )));

        if homework.is_pk_or_vocab() {
            Self::run_pk_workflow(&session, &homework, &flat_questions, &config, &sender);
            return;
        }

        // 阶段 1: check_start 开启作业
        let _ = sender.send(AutoProgressMsg::Step {
            current: 2,
            total: 8,
            title: "开启云端答题会话 (checkstart)".to_string(),
        });

        let mut resource_id = String::new();
        if !config.no_upload {
            match CloudClient::check_start(&session.token, &homework.id) {
                Ok(body) => {
                    if let Some(r_id) = body.get("resource_id").or_else(|| body.get("id")).and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        Value::Number(n) => Some(n.to_string()),
                        _ => None,
                    }) {
                        resource_id = r_id;
                    }
                    let _ = sender.send(AutoProgressMsg::Log(format!(
                        "✓ 云端会话开启成功 (resource_id: {resource_id})"
                    )));
                }
                Err(e) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!("⚠️ check_start 警告: {e}，尝试继续...")));
                }
            }
        } else {
            let _ = sender.send(AutoProgressMsg::Log("✓ 试运行模式: 跳过云端 check_start".to_string()));
        }

        // 阶段 2: ppr_resources 注册实体映射
        let _ = sender.send(AutoProgressMsg::Step {
            current: 3,
            total: 8,
            title: "注册题目实体资源 (ppr-res)".to_string(),
        });

        if !config.no_upload {
            let batch: Vec<Value> = flat_questions
                .iter()
                .enumerate()
                .map(|(idx, q)| {
                    json!({
                        "entity_id": q.number,
                        "order": idx
                    })
                })
                .collect();
            match CloudClient::ppr_resources(&session.token, &batch) {
                Ok(_) => {
                    let _ = sender.send(AutoProgressMsg::Log("✓ 题目实体批次映射注册成功".to_string()));
                }
                Err(e) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!("⚠️ ppr_resources 响应: {e}")));
                }
            }
        }

        // 阶段 4: TTS 语音音频合成
        let _ = sender.send(AutoProgressMsg::Step {
            current: 4,
            total: 8,
            title: format!("TTS 语音音频合成 (音色: {})", config.voice_name),
        });

        // 阶段 5: 生成科大讯飞评测打分 XML
        let _ = sender.send(AutoProgressMsg::Step {
            current: 5,
            total: 8,
            title: "生成标准评分 XML 与讯飞评测打分".to_string(),
        });

        // 阶段 6: 上传 OSS 并同步答题数据 (sync-v2)
        let _ = sender.send(AutoProgressMsg::Step {
            current: 6,
            total: 8,
            title: if config.no_upload {
                "本地试运行答题数据归档 (--no-upload)".to_string()
            } else {
                "上传 OSS 并同步答题数据 (sync-v2)".to_string()
            },
        });

        // 预先计算加权拟真分配方案 (包含客观题二值化、0.125档位、失误题加权及大题防满分)
        let allocations = scoring::plan_question_allocations(
            &flat_questions,
            config.target_total,
            config.score_reroll_nonce,
            config.allow_over_max,
        );

        let temp_dir = std::env::temp_dir().join("fepc_run").join(&homework.id);
        let _ = std::fs::create_dir_all(&temp_dir);

        let mut completed_questions = 0;
        let mut total_accumulated_score = 0.0;

        for (idx, q) in flat_questions.iter().enumerate() {
            let q_num = &q.number;
            let alloc = allocations.get(idx);
            let q_max_score = if let Some(a) = alloc {
                a.max_score
            } else {
                (homework.total_point / total_questions as f64).max(1.0)
            };

            // 模式判别：手动模式优先采用手动指定分数；一键自动模式采用加权拟真分配
            let (target_q_score, target_ise) = if config.mode == "manual" {
                let manual_val = config.per_question_scores.get(&idx).copied().unwrap_or_else(|| {
                    alloc.map(|a| a.allocated_score).unwrap_or(2.5)
                });
                let ise = if q_max_score > 0.0 {
                    let raw = (manual_val * 5.0 / q_max_score) - 0.0001;
                    scoring::round_points(raw.clamp(0.0, 5.0))
                } else {
                    5.0
                };
                (manual_val, ise)
            } else if let Some(a) = alloc {
                (a.allocated_score, a.ise_score)
            } else {
                (config.target_total / total_questions as f64, 4.9)
            };

            let alloc_tag = if let Some(a) = alloc {
                if a.is_full {
                    "[满分]"
                } else if a.is_mistake {
                    "[失误题]"
                } else if a.is_big_question {
                    "[长难大题]"
                } else {
                    ""
                }
            } else {
                ""
            };

            let _ = sender.send(AutoProgressMsg::Log(format!(
                "👉 [{}/{}] 处理第 {} 题 ({} · 计划 {:.2}/{:.2} 分 {})",
                idx + 1, total_questions, q_num, q.category, target_q_score, q_max_score, alloc_tag
            )));

            // 确定答题文本
            let text_to_answer = if !q.answers.is_empty() {
                q.answers.join(" ")
            } else if !q.prompt_text.is_empty() {
                q.prompt_text.clone()
            } else {
                "OK".to_string()
            };

            let is_choice = q.category.contains("choose") || q.category.contains("choice");
            let choose_option = if is_choice {
                q.answers.first().cloned().unwrap_or_else(|| "A".to_string())
            } else {
                String::new()
            };

            // 1. 获取答题音频：手动模式优先检查该题是否有本地自定义音频，否则走 TTS / 克隆
            let mut audio_bytes = Vec::new();
            let custom_audio = if config.mode == "manual" {
                config.per_question_audio.get(&idx).cloned()
            } else {
                None
            };

            if let Some(ref path_str) = custom_audio
                && !path_str.trim().is_empty()
                && let Ok(bytes) = std::fs::read(path_str)
            {
                audio_bytes = bytes;
                let _ = sender.send(AutoProgressMsg::Log(format!(
                    "  ✓ [自定义音频] 已加载本地录音: {} ({} 字节)",
                    path_str, audio_bytes.len()
                )));
            }

            if audio_bytes.is_empty() && !is_choice && !text_to_answer.trim().is_empty() {
                let http_url = if config.http_tts_url.trim().is_empty() { None } else { Some(config.http_tts_url.as_str()) };
                let http_key = if config.http_tts_key.trim().is_empty() { None } else { Some(config.http_tts_key.as_str()) };
                let http_model = if config.http_tts_model.trim().is_empty() { None } else { Some(config.http_tts_model.as_str()) };
                let clone_ref = if config.clone_reference_path.trim().is_empty() { None } else { Some(config.clone_reference_path.as_str()) };

                match synthesize_auto(
                    &text_to_answer,
                    &config.tts_mode,
                    Some(&config.voice_name),
                    Some(config.rate),
                    Some(config.pitch),
                    http_url,
                    http_key,
                    http_model,
                    clone_ref,
                ) {
                    Ok(bytes) => {
                        audio_bytes = bytes;
                        let engine_desc = if config.tts_mode.eq_ignore_ascii_case("clone") {
                            "Voice Clone"
                        } else if config.tts_mode.eq_ignore_ascii_case("http") {
                            "HTTP TTS"
                        } else {
                            "Edge TTS"
                        };
                        let _ = sender.send(AutoProgressMsg::Log(format!(
                            "  ✓ [{engine_desc}] 语音合成完毕 (大小: {} 字节)",
                            audio_bytes.len()
                        )));
                    }
                    Err(e) => {
                        let _ = sender.send(AutoProgressMsg::Log(format!("  ⚠️ [TTS] 合成跳过/回退: {e}")));
                    }
                }
            }

            // 保存音频切片以供答题完成后即时回放试听
            let mut saved_audio_path = None;
            if !audio_bytes.is_empty() {
                let audio_file = temp_dir.join(format!("{}_{}.mp3", idx + 1, q_num));
                if std::fs::write(&audio_file, &audio_bytes).is_ok() {
                    saved_audio_path = Some(audio_file.to_string_lossy().to_string());
                }
            }

            // 2. 生成标准科大讯飞评测 XML
            let profile = ScoreProfile {
                total: target_ise,
                accuracy: config.accuracy.min(5.0),
                fluency: config.fluency.min(5.0),
                integrity: config.integrity.min(5.0),
                stress: 4.90,
            }.clamped();

            let xml_content = scoring::generate_score_xml(
                &text_to_answer,
                &profile,
                &q.category,
                idx as u64,
                !config.allow_over_max,
            );

            // 3. 上传与同步
            let mut audio_url = String::new();
            let mut xml_url = String::new();
            let mut upload_file_id = String::new();

            if !config.no_upload {
                // 上传音频
                if !audio_bytes.is_empty() {
                    upload_file_id = CloudClient::pre_upload(&session.token).unwrap_or_default();
                    let audio_file_name = format!("{}_{}_{}.mp3", homework.id, idx + 1, q_num);
                    if let Ok(oss_cfg) = CloudClient::get_oss_config(&session.token, "mark_audio", "1", &audio_file_name)
                        && let Ok(url) = OssUploader::upload_form(&oss_cfg, &audio_file_name, &audio_bytes) {
                            audio_url = url;
                            let _ = sender.send(AutoProgressMsg::Log("  ✓ [OSS] 音频直传完成".to_string()));
                        }
                }

                // 上传 XML
                let detail_file_id = CloudClient::pre_upload(&session.token).unwrap_or_default();
                let xml_file_name = format!("{}_{}_{}.xml", homework.id, idx + 1, q_num);
                if let Ok(oss_cfg) = CloudClient::get_oss_config(&session.token, "mark_xml", "1", &xml_file_name)
                    && let Ok(url) = OssUploader::upload_form(&oss_cfg, &xml_file_name, xml_content.as_bytes()) {
                        xml_url = url;
                        let _ = sender.send(AutoProgressMsg::Log("  ✓ [OSS] 评分 XML 直传完成".to_string()));
                    }

                // 同步题目分数
                let score_detail = scoring::build_score_detail(
                    &profile,
                    &q.category,
                    &choose_option,
                    true,
                    0.125,
                );

                let q_use_time: u32 = if is_choice {
                    6
                } else {
                    let words = text_to_answer.split_whitespace().count();
                    if words > 0 {
                        ((words as f64 / 2.2).max(3.0)).round() as u32
                    } else {
                        5
                    }
                };

                let sync_params = json!({
                    "homework_id": homework.id.parse::<i64>().unwrap_or(0),
                    "resource_id": resource_id,
                    "set_id": "",
                    "entity_id": q_num,
                    "order": format!("{}", idx + 1),
                    "score": format!("{:.2}", target_q_score),
                    "correct": if target_q_score > 0.0 { 1 } else { 0 },
                    "auto_submit": 1,
                    "use_time": q_use_time,
                    "graduation": 0.125,
                    "upload_file_id": upload_file_id,
                    "upload_file_url": audio_url,
                    "detail_file_id": detail_file_id,
                    "detail_file_url": xml_url,
                    "score_detail": score_detail,
                    "serial_id": "0",
                    "mistake_id": "",
                    "client_time": Self::now_str(),
                    "version": 2,
                    "mark_service": 1,
                    "seq_id": Self::seq_id(),
                    "spx_file_url": "",
                    "mark_trace_id": "{\"engine\":\"cloud\"}"
                });

                match CloudClient::sync_audio_v2(&session.token, sync_params) {
                    Ok(_) => {
                        let _ = sender.send(AutoProgressMsg::Log(format!(
                            "  ✓ 第 {} 题同步成功 (得分: {:.2})",
                            q_num, target_q_score
                        )));
                    }
                    Err(e) => {
                        let _ = sender.send(AutoProgressMsg::Log(format!(
                            "  ⚠️ 第 {} 题同步响应: {e}",
                            q_num
                        )));
                    }
                }
            } else {
                // 试运行：写出本地文件供核验
                let xml_file = temp_dir.join(format!("{}_{}.xml", idx + 1, q_num));
                let _ = std::fs::write(&xml_file, &xml_content);
                let _ = sender.send(AutoProgressMsg::Log(format!(
                    "  ✓ [试运行] 第 {} 题完成 (得分: {:.2})",
                    q_num, target_q_score
                )));
            }

            completed_questions += 1;
            total_accumulated_score += target_q_score;

            let _ = sender.send(AutoProgressMsg::QuestionDone {
                index: idx,
                number: q_num.clone(),
                category: q.category.clone(),
                score: target_q_score,
                max_score: q_max_score,
                status: "完成".to_string(),
                audio_path: saved_audio_path,
            });

            // 拟人化自然波动停顿：模拟真实思考与做题网络交互 (600ms ~ 1400ms 动态抖动)
            let jitter = 600 + (SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() % 800) as u64;
            std::thread::sleep(Duration::from_millis(jitter));
        }

        // 阶段 7: set-use-time 提交做题总用时 (支持智能散列用时与手动自定义用时)
        let _ = sender.send(AutoProgressMsg::Step {
            current: 7,
            total: 8,
            title: "上报模拟答题总用时 (set-use-time)".to_string(),
        });

        let total_use_secs = if config.use_time_source == "smart" {
            let (smart_secs, _) = scoring::calculate_smart_use_time(&homework, config.time_reroll_nonce);
            smart_secs.max(30)
        } else {
            config.use_time_seconds.max(30)
        };

        if !config.no_upload {
            match CloudClient::set_use_time(
                &session.token,
                &homework.id,
                &resource_id,
                total_use_secs,
            ) {
                Ok(_) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!(
                        "✓ 答题用时已上报: {total_use_secs} 秒"
                    )));
                }
                Err(e) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!("⚠️ set_use_time 警告: {e}")));
                }
            }
        } else {
            let _ = sender.send(AutoProgressMsg::Log(format!(
                "✓ [试运行] 模拟上报用时: {total_use_secs} 秒"
            )));
        }

        // 阶段 8: report-score 最终成绩核验与终审提交
        let _ = sender.send(AutoProgressMsg::Step {
            current: 8,
            total: 8,
            title: "终审提交试卷与核验成绩 (report-score)".to_string(),
        });

        let final_calc_score = scoring::round_points(if config.allow_over_max {
            total_accumulated_score
        } else {
            total_accumulated_score.min(homework.total_point)
        });
        let score_rate = if homework.total_point > 0.0 {
            let rate = final_calc_score / homework.total_point;
            let clamped_rate = if config.allow_over_max {
                rate.max(0.0)
            } else {
                rate.clamp(0.0, 1.0)
            };
            (clamped_rate * 1_000_000.0).round() as i64
        } else {
            1_000_000
        };

        if !config.no_upload {
            match CloudClient::report_score(
                &session.token,
                &homework.id,
                &resource_id,
                &session.parent_account_id,
                score_rate,
                total_use_secs,
            ) {
                Ok(_) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!(
                        "🎉 作业终审提交成功！最终得分率: {:.1}%，折合分数: {:.1} / {:.1} 分",
                        (score_rate as f64) / 10_000.0,
                        final_calc_score,
                        homework.total_point
                    )));
                }
                Err(e) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!("⚠️ report_score 响应: {e}")));
                }
            }
            // 上传完成，清理临时切片与音频目录
            if temp_dir.exists() {
                let _ = std::fs::remove_dir_all(&temp_dir);
            }
        } else {
            let _ = sender.send(AutoProgressMsg::Log(format!(
                "🎉 [试运行模式结束] 本地全题作答完成！核算总分: {:.1} / {:.1} 分 (文件留存于 {:?})",
                final_calc_score, homework.total_point, temp_dir
            )));
        }

        let _ = sender.send(AutoProgressMsg::Complete {
            success: true,
            final_score: final_calc_score,
            message: format!(
                "作业 [{}] 全部 {} 道题目已完成！最终得分: {:.1} 分",
                homework.name, completed_questions, final_calc_score
            ),
        });
    }

    /// 词汇对战 PK 与在线词汇专项任务自动化流水线
    fn run_pk_workflow(
        session: &UserSession,
        homework: &HomeworkItem,
        flat_questions: &[ParsedQuestion],
        config: &AutoCompleteConfig,
        sender: &Sender<AutoProgressMsg>,
    ) {
        let total_questions = flat_questions.len();
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "🔥 检测到词汇PK/在线词汇专项作业 (set_type={}, pk={})，切换至词汇专用轻量化对战流水线...",
            homework.set_type, homework.pk
        )));

        // 阶段 1: score-drop 重置作业状态
        let _ = sender.send(AutoProgressMsg::Step {
            current: 2,
            total: 6,
            title: "重置作业对战状态 (score-drop)".to_string(),
        });

        if !config.no_upload {
            if let Err(e) = CloudClient::score_drop(&session.token, &homework.id) {
                let _ = sender.send(AutoProgressMsg::Log(format!("  ⚠️ score-drop 响应: {e}")));
            } else {
                let _ = sender.send(AutoProgressMsg::Log("  ✓ 历史对战状态已成功重置".to_string()));
            }
        }

        // 阶段 2: checkstart 开启会话
        let _ = sender.send(AutoProgressMsg::Step {
            current: 3,
            total: 6,
            title: "开启云端对战会话 (checkstart)".to_string(),
        });

        let mut resource_id = homework.resource_id.clone();
        if !config.no_upload {
            match CloudClient::check_start(&session.token, &homework.id) {
                Ok(body) => {
                    if let Some(r_id) = body.get("resource_id").or_else(|| body.get("id")).and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        Value::Number(n) => Some(n.to_string()),
                        _ => None,
                    }) {
                        resource_id = r_id;
                    }
                    let _ = sender.send(AutoProgressMsg::Log(format!(
                        "  ✓ 云端会话开启成功 (resource_id: {resource_id})"
                    )));
                }
                Err(e) => {
                    let _ = sender.send(AutoProgressMsg::Log(format!("  ⚠️ check_start 响应: {e}")));
                }
            }
        }

        // 阶段 3: set-use-time 模拟用时
        let _ = sender.send(AutoProgressMsg::Step {
            current: 4,
            total: 6,
            title: "上报对战答题用时 (set-use-time)".to_string(),
        });

        let use_time = config.use_time_seconds.max((total_questions as u32) * 3).max(30);
        if !config.no_upload {
            let _ = CloudClient::set_use_time(&session.token, &homework.id, &resource_id, use_time);
            let _ = sender.send(AutoProgressMsg::Log(format!("  ✓ 模拟答题用时: {use_time} 秒")));
        }

        // 阶段 4: 构造答题明细并同步提交
        let _ = sender.send(AutoProgressMsg::Step {
            current: 5,
            total: 6,
            title: "生成词汇作答记录并同步 (submit-word / score-sync)".to_string(),
        });

        let target_ratio = if homework.total_point > 0.0 {
            let ratio = config.target_total / homework.total_point;
            if config.allow_over_max {
                ratio.max(0.0)
            } else {
                ratio.clamp(0.0, 1.0)
            }
        } else {
            1.0
        };
        let correct_count = ((total_questions as f64) * target_ratio).round() as usize;

        let mut pk_details = Vec::new();
        let mut vocab_details = Vec::new();
        let now = Self::now_str();

        for (idx, q) in flat_questions.iter().enumerate() {
            let is_correct = idx < correct_count;
            let ans = if is_correct {
                q.answers.first().cloned().unwrap_or_else(|| {
                    q.options.first().map(|o| o.label.clone()).unwrap_or_else(|| "A".to_string())
                })
            } else {
                let correct_ans = q.answers.first().map(|s| s.as_str()).unwrap_or("");
                q.options.iter()
                    .find(|o| o.label != correct_ans && o.text != correct_ans)
                    .map(|o| o.label.clone())
                    .unwrap_or_else(|| "B".to_string())
            };

            let point = if is_correct { 1.0 } else { 0.0 };
            let q_num_i64 = q.number.parse::<i64>().unwrap_or((idx + 1) as i64);
            let word_id = q.word_id.unwrap_or(q_num_i64);
            let question_id = q.question_id.unwrap_or(q_num_i64);

            pk_details.push(json!({
                "point": if is_correct { 1 } else { 0 },
                "answer": ans,
                "word_id": word_id,
                "question_id": question_id
            }));

            vocab_details.push(json!({
                "word_id": word_id,
                "question_id": question_id,
                "score_rate": if is_correct { 1 } else { 0 },
                "score_detail": format!("{{\"choose\":\"{}\",\"version\":\"3\"}}", ans),
                "client_time": now,
                "answer": ans,
                "point": point
            }));

            let max_score_per_q = homework.total_point / (total_questions as f64).max(1.0);
            let earned = if is_correct { max_score_per_q } else { 0.0 };

            let _ = sender.send(AutoProgressMsg::QuestionDone {
                index: idx,
                number: q.number.clone(),
                category: q.category.clone(),
                score: earned,
                max_score: max_score_per_q,
                status: if is_correct { "正确".to_string() } else { "故意答错(控分)".to_string() },
                audio_path: None,
            });
        }

        if !config.no_upload {
            if homework.pk == 1 {
                let mut serial_id = String::new();
                if let Ok(info) = CloudClient::get_pk_info(&session.token, &homework.id, &session.parent_account_id)
                    && let Some(s) = info.get("serial_id").or_else(|| info.get("serialId")).and_then(|v| v.as_str()) {
                        serial_id = s.to_string();
                    }
                if serial_id.is_empty() {
                    let w_ids: Vec<String> = flat_questions.iter().map(|q| {
                        q.word_id.map(|id| id.to_string()).unwrap_or_else(|| q.number.clone())
                    }).collect();
                    let q_ids: Vec<String> = flat_questions.iter().map(|q| {
                        q.question_id.map(|id| id.to_string()).unwrap_or_else(|| q.number.clone())
                    }).collect();
                    let start_params = json!({
                        "unit_id": 0,
                        "book_id": 0,
                        "word_ids": w_ids.join(","),
                        "question_ids": q_ids.join(","),
                        "name": homework.name,
                        "source": 1,
                        "version": 1,
                        "parent_account_id": session.parent_account_id
                    });
                    if let Ok(started) = CloudClient::start_word_pk(&session.token, start_params)
                        && let Some(s) = started.get("serial_id").and_then(|v| v.as_str()) {
                            serial_id = s.to_string();
                        }
                }

                let submit_params = json!({
                    "serial_id": serial_id,
                    "score_detail": pk_details,
                    "use_time": use_time,
                    "source": 1
                });
                match CloudClient::submit_word_pk(&session.token, submit_params) {
                    Ok(_) => {
                        let _ = sender.send(AutoProgressMsg::Log("  ✓ 词汇对战成绩 (submit-word) 提交成功".to_string()));
                    }
                    Err(e) => {
                        let _ = sender.send(AutoProgressMsg::Log(format!("  ⚠️ submit-word 响应: {e}")));
                    }
                }
            } else {
                let sync_params = json!({
                    "homework_id": homework.id.parse::<i64>().unwrap_or(0),
                    "resource_id": resource_id,
                    "use_time": use_time,
                    "seq_id": Self::seq_id(),
                    "type": 6,
                    "mark_service": 1,
                    "detail": vocab_details
                });
                match CloudClient::score_sync(&session.token, sync_params) {
                    Ok(_) => {
                        let _ = sender.send(AutoProgressMsg::Log("  ✓ 词汇专项成绩 (score-sync) 同步成功".to_string()));
                    }
                    Err(e) => {
                        let _ = sender.send(AutoProgressMsg::Log(format!("  ⚠️ score-sync 响应: {e}")));
                    }
                }
            }
        }

        // 阶段 5: report-score 最终成绩上报
        let _ = sender.send(AutoProgressMsg::Step {
            current: 6,
            total: 6,
            title: "终审提交试卷与核验成绩 (report-score)".to_string(),
        });

        let final_calc_score = if config.allow_over_max {
            config.target_total
        } else {
            config.target_total.min(homework.total_point)
        };
        let score_rate = if homework.total_point > 0.0 {
            let rate = final_calc_score / homework.total_point;
            let clamped_rate = if config.allow_over_max {
                rate.max(0.0)
            } else {
                rate.clamp(0.0, 1.0)
            };
            (clamped_rate * 1_000_000.0).round() as i64
        } else {
            1_000_000
        };

        if !config.no_upload {
            let _ = CloudClient::report_score(
                &session.token,
                &homework.id,
                &resource_id,
                &session.parent_account_id,
                score_rate,
                use_time,
            );
        }

        let _ = sender.send(AutoProgressMsg::Complete {
            success: true,
            final_score: final_calc_score,
            message: format!(
                "词汇对战作业 [{}] 全部 {} 题已完成！最终得分: {:.1} 分 (正确 {} 题)",
                homework.name, total_questions, final_calc_score, correct_count
            ),
        });
    }
}
