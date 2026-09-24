use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use md5::Digest;
use serde_json::{json, Value};

use crate::models::{
    HomeworkItem, HomeworkSource, ParsedQuestion, ParsedSection, QuestionOption, UserSession,
};
use crate::workflow::oss::OssFormConfig;

pub struct CloudClient;

impl CloudClient {
    pub fn session_file_path() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let dir = PathBuf::from(app_data).join("fepc");
            let _ = fs::create_dir_all(&dir);
            let p = dir.join("fepc_session.json");
            if !p.exists() {
                let old_p = PathBuf::from("fepc_session.json");
                if old_p.exists() {
                    let _ = fs::copy(&old_p, &p);
                }
            }
            return p;
        }
        PathBuf::from("fepc_session.json")
    }

    #[inline(never)]
    pub fn load_saved_session() -> Option<UserSession> {
        let path = Self::session_file_path();
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path) {
                return serde_json::from_str::<UserSession>(&content).ok();
            }
        None
    }

    pub fn save_session(session: &UserSession) {
        let path = Self::session_file_path();
        if let Ok(json_str) = serde_json::to_string_pretty(session) {
            let _ = fs::write(&path, json_str);
        }
    }

    pub fn clear_session() {
        let path = Self::session_file_path();
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    /// 生成或获取现有设备码
    pub fn get_or_generate_device_code(existing: Option<&str>) -> String {
        if let Some(code) = existing
            && !code.trim().is_empty() {
                return code.trim().to_string();
            }
        let res = cloud::cloud::cloud_generate_device_code_json();
        if let Some(code) = res.get("result").and_then(|r| r.get("device_code")).and_then(|v| v.as_str()) {
            return code.to_string();
        }
        "BF1CB2218D9A88B9|0C072A134F008B9F".to_string()
    }

    /// 执行官方 ETS100 云端登录
    pub fn login(phone: &str, password: &str, device_code: &str) -> Result<UserSession, String> {
        let res = cloud::cloud::cloud_login_json(phone, password, device_code);
        if res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let result = res.get("result").ok_or("未返回结果数据")?;
            let token = result.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let parent_id = result.get("parent_account_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let user_name = result.get("user_name").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let session = UserSession {
                phone: phone.to_string(),
                token,
                device_code: device_code.to_string(),
                parent_account_id: parent_id,
                user_name,
                is_logged_in: true,
            };
            Self::save_session(&session);
            Ok(session)
        } else {
            let error = res.get("error").cloned().unwrap_or(json!({}));
            let code = error.get("code").and_then(|v| v.as_str()).unwrap_or("unknown");
            let msg = error.get("message").and_then(|v| v.as_str()).unwrap_or("登录失败");
            let api_code = error.get("api_code").and_then(|v| v.as_i64()).unwrap_or(0);

            // 若返回 30014 (设备需要换绑)，优先尝试免密验证码 "0" 快速绑定 (对齐 ets_get_answer 和 fe-pro)
            if api_code == 30014
                && let Ok(sess) = Self::bind_device(phone, password, device_code, "0") {
                    return Ok(sess);
                }

            // 识别 30008 (sms_verification_required)、30014、以及提示包含设备绑定的情况
            if code == "device_bind_required"
                || code == "sms_verification_required"
                || api_code == 30008
                || api_code == 30014
                || msg.contains("新设备需要绑定")
                || msg.contains("设备需要绑定")
            {
                Err("device_bind_required".to_string())
            } else {
                Err(format!("{}: {}", code, msg))
            }
        }
    }

    /// 发送换绑验证码
    pub fn send_rebind_code(phone: &str) -> Result<(), String> {
        let res = cloud::cloud::cloud_send_device_binding_code_json(phone);
        if res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let msg = res.get("error").and_then(|e| e.get("message")).and_then(|v| v.as_str()).unwrap_or("发送验证码失败");
            Err(msg.to_string())
        }
    }

    /// 验证码换绑并登录
    pub fn bind_device(phone: &str, password: &str, device_code: &str, code: &str) -> Result<UserSession, String> {
        let res = cloud::cloud::cloud_bind_device_json(phone, password, device_code, code);
        if res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let result = res.get("result").ok_or("换绑未返回结果")?;
            let token = result.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let parent_id = result.get("parent_account_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let user_name = result.get("user_name").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let session = UserSession {
                phone: phone.to_string(),
                token,
                device_code: device_code.to_string(),
                parent_account_id: parent_id,
                user_name,
                is_logged_in: true,
            };
            Self::save_session(&session);
            Ok(session)
        } else {
            let msg = res.get("error").and_then(|e| e.get("message")).and_then(|v| v.as_str()).unwrap_or("设备换绑失败");
            Err(msg.to_string())
        }
    }

    /// 获取云端作业列表
    pub fn get_homework_list(session: &UserSession, status: &str) -> Result<Vec<HomeworkItem>, String> {
        let res = cloud::cloud::cloud_get_homework_list_json(
            &session.token,
            &session.parent_account_id,
            status,
        );

        if !res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let msg = res.get("error").and_then(|e| e.get("message")).and_then(|v| v.as_str()).unwrap_or("获取作业列表失败");
            return Err(msg.to_string());
        }

        let mut items = Vec::new();
        if let Some(list) = res.get("result").and_then(|r| r.get("homeworks")).and_then(|v| v.as_array()) {
            for hw in list {
                let id = hw.get("id").map(|v| match v {
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    _ => "".to_string(),
                }).unwrap_or_default();

                let name = hw.get("name").and_then(|v| v.as_str()).unwrap_or("未命名作业").to_string();
                let deadline = hw.get("deadline").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let status_str = hw.get("status_name").and_then(|v| v.as_str()).unwrap_or(status).to_string();
                let total_point = hw.get("total_point").and_then(|v| v.as_f64()).unwrap_or(30.0);
                let awarded_point = hw.get("real_score").and_then(|v| v.as_f64());
                let question_count = hw.get("question_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                let set_type = hw.get("set_type").and_then(|v| v.as_i64()).unwrap_or(0);
                let pk = hw.get("pk").and_then(|v| v.as_i64()).unwrap_or(0);
                let resource_id = hw.get("resource_id").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let mut resource_urls = Vec::new();
                if let Some(contents) = hw.get("contents").and_then(|v| v.as_array()) {
                    for c in contents {
                        if let Some(dl_url) = c.get("download_url").and_then(|v| v.as_str())
                            && !dl_url.is_empty() {
                                resource_urls.push(dl_url.to_string());
                            }
                    }
                }

                items.push(HomeworkItem {
                    id,
                    name,
                    status: status_str,
                    deadline,
                    total_point,
                    awarded_point,
                    question_count,
                    source: HomeworkSource::Cloud,
                    local_dir: None,
                    resource_urls,
                    sections: Vec::new(),
                    set_type,
                    pk,
                    resource_id,
                });
            }
        }
        Ok(items)
    }

    /// 通过 4 级回退链获取并解析词汇PK/在线作业题目
    pub fn fetch_and_parse_pk_homework(session: &UserSession, homework: &HomeworkItem) -> Result<Vec<ParsedSection>, String> {
        let token = &session.token;
        let parent_id = &session.parent_account_id;
        if token.is_empty() {
            return Err("未登录 ETS 账号，无法请求云端在线题目".to_string());
        }

        let mut pk_json: Option<Value> = None;

        // 路径 1: pk-info
        if let Ok(raw_pk) = Self::get_pk_info(token, &homework.id, parent_id)
            && Self::has_questions(&raw_pk) {
                pk_json = Some(raw_pk);
            }

        // 路径 2: homework/struct (带 resource_id)
        if pk_json.is_none()
            && let Ok(raw_struct) = Self::get_homework_struct(token, &homework.id, &homework.resource_id, parent_id)
            && Self::has_questions(&raw_struct) {
                pk_json = Some(raw_struct);
            }

        // 路径 3: homework/struct (不带 resource_id)
        if pk_json.is_none() && !homework.resource_id.is_empty()
            && let Ok(raw_struct_no_res) = Self::get_homework_struct(token, &homework.id, "", parent_id)
            && Self::has_questions(&raw_struct_no_res) {
                pk_json = Some(raw_struct_no_res);
            }

        // 路径 4: score-detail + word/struct (历史已完成对战作业题目恢复)
        if pk_json.is_none()
            && let Ok(detail_body) = Self::get_score_detail(token, &homework.id, &homework.resource_id, parent_id)
            && let Some(details_arr) = detail_body.get("details").and_then(Value::as_array) {
                let mut word_id_set = std::collections::HashSet::new();
                let mut question_id_set = std::collections::HashSet::new();
                for d in details_arr {
                    if let Some(w_arr) = d.get("word").and_then(Value::as_array) {
                        for entry in w_arr {
                            if let Some(wid) = entry.get("word_id").and_then(Value::as_i64)
                                && wid > 0 {
                                    word_id_set.insert(wid);
                                }
                            if let Some(qid) = entry.get("question_id").and_then(Value::as_i64)
                                && qid > 0 {
                                    question_id_set.insert(qid);
                                }
                        }
                    }
                }

                if !word_id_set.is_empty() {
                    let word_ids: Vec<i64> = word_id_set.into_iter().collect();
                    if let Ok(word_struct_body) = Self::get_word_struct(token, &word_ids)
                        && let Some(data_arr) = word_struct_body.get("data").and_then(Value::as_array) {
                            let mut filtered_arr = Vec::new();
                            for item in data_arr {
                                if let Some(q_list) = item.get("question").and_then(Value::as_array) {
                                    let mut matching_qs = Vec::new();
                                    for q in q_list {
                                        let qid = q.get("id").and_then(Value::as_i64).unwrap_or(0);
                                        if question_id_set.is_empty() || question_id_set.contains(&qid) {
                                            matching_qs.push(q.clone());
                                        }
                                    }
                                    if !matching_qs.is_empty() {
                                        let mut item_copy = item.clone();
                                        if let Some(obj) = item_copy.as_object_mut() {
                                            obj.insert("question".to_string(), Value::Array(matching_qs));
                                        }
                                        filtered_arr.push(item_copy);
                                    }
                                }
                            }
                            if !filtered_arr.is_empty() {
                                pk_json = Some(json!({ "data": filtered_arr }));
                            }
                        }
                }
            }

        let mut json_val = pk_json.ok_or_else(|| "未能从云端解析到任何题目 (4级回退均为空)".to_string())?;
        if let Some(obj) = json_val.as_object_mut() {
            obj.insert("structure_type".to_string(), json!("word_pk"));
        }

        let group_name = if homework.set_type == 6 || homework.pk == 1 {
            "词汇PK".to_string()
        } else {
            "在线作业".to_string()
        };

        let raw_questions = resolver::api::parse_task(&json_val, None, None);
        let mut questions = Vec::new();
        for (i, raw_q) in raw_questions.into_iter().enumerate() {
            let no = raw_q.get("no").map(|v| match v {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                _ => (i + 1).to_string(),
            }).unwrap_or_else(|| (i + 1).to_string());

            let cat = raw_q.get("type").or_else(|| raw_q.get("category")).and_then(|v| v.as_str()).unwrap_or("word.choice").to_string();
            let prompt = raw_q
                .get("question")
                .or_else(|| raw_q.get("text"))
                .or_else(|| raw_q.get("prompt"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut answers = Vec::new();
            if let Some(ans_val) = raw_q.get("answers").or_else(|| raw_q.get("answer")) {
                match ans_val {
                    Value::Array(arr) => {
                        for a in arr {
                            if let Some(s) = a.as_str() {
                                let s_trim = s.trim();
                                if !s_trim.is_empty() { answers.push(s_trim.to_string()); }
                            }
                        }
                    }
                    Value::String(s) => {
                        let s_trim = s.trim();
                        if !s_trim.is_empty() { answers.push(s_trim.to_string()); }
                    }
                    _ => {}
                }
            }

            let mut options = Vec::new();
            if let Some(opt_arr) = raw_q.get("options").and_then(|v| v.as_array()) {
                for (oi, o_val) in opt_arr.iter().enumerate() {
                    let full_str = o_val.as_str().unwrap_or("");
                    let (label, text) = if let Some((l, t)) = full_str.split_once('.') {
                        (l.trim().to_string(), t.trim().to_string())
                    } else {
                        (((b'A' + (oi as u8).min(25)) as char).to_string(), full_str.to_string())
                    };
                    options.push(QuestionOption { label, text, image_path: None });
                }
            }

            questions.push(ParsedQuestion {
                index: i,
                number: no,
                category: cat,
                prompt_text: prompt,
                answers,
                important_points: Vec::new(),
                audio_file: None,
                options,
            });
        }

        if questions.is_empty() {
            return Err("词汇/PK解析题目数量为0".to_string());
        }

        Ok(vec![ParsedSection {
            section_type: "word_pk".to_string(),
            title: group_name,
            questions,
        }])
    }

    /// 下载并解析云端试卷
    pub fn download_and_parse_homework(session: &UserSession, homework: &HomeworkItem) -> Result<HomeworkItem, String> {
        // 若为词汇PK或无常规ZIP资源下载链接，直接走 4 级回退链在线获取
        if homework.is_pk_or_vocab() {
            let sections = Self::fetch_and_parse_pk_homework(session, homework)?;
            let total_q = sections.iter().map(|s| s.questions.len()).sum();
            return Ok(HomeworkItem {
                id: homework.id.clone(),
                name: homework.name.clone(),
                status: "已解析完成".to_string(),
                deadline: homework.deadline.clone(),
                total_point: homework.total_point,
                awarded_point: homework.awarded_point,
                question_count: total_q,
                source: HomeworkSource::Cloud,
                local_dir: None,
                resource_urls: homework.resource_urls.clone(),
                sections,
                set_type: homework.set_type,
                pk: homework.pk,
                resource_id: homework.resource_id.clone(),
            });
        }

        let temp_dir = std::env::temp_dir().join("fepc_cache").join(&homework.id);
        let _ = fs::create_dir_all(&temp_dir);

        let mut sections = Vec::new();
        let mut question_counter = 0usize;

        for (idx, url) in homework.resource_urls.iter().enumerate() {

            let zip_dest = temp_dir.join(format!("part_{idx}.zip"));
            let zip_dest_str = zip_dest.to_string_lossy().to_string();

            // 下载 ZIP 文件
            let dl_res = cloud::cloud::cloud_download_file_json(url, &zip_dest_str);
            if !dl_res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }

            // 计算解压密码
            let pwd_res = cloud::cloud::cloud_zip_password_json(&zip_dest_str);
            let password = pwd_res.get("result")
                .and_then(|r| r.get("password"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // 解压 ZIP
            let part_dir = temp_dir.join(format!("part_{idx}"));
            let _ = fs::create_dir_all(&part_dir);

            if let Ok(file) = File::open(&zip_dest) {
                let mut archive = match zip::ZipArchive::new(file) {
                    Ok(a) => a,
                    Err(_) => continue,
                };

                for i in 0..archive.len() {
                    let zip_res = if password.is_empty() {
                        archive.by_index(i)
                    } else {
                        archive.by_index_decrypt(i, password.as_bytes())
                    };
                    if let Ok(mut zip_file) = zip_res {
                        let out_path = match zip_file.enclosed_name() {
                            Some(p) => part_dir.join(p),
                            None => continue,
                        };
                        if zip_file.is_dir() {
                            let _ = fs::create_dir_all(&out_path);
                        } else {
                            if let Some(p) = out_path.parent() {
                                let _ = fs::create_dir_all(p);
                            }
                            if let Ok(mut outfile) = File::create(&out_path) {
                                let _ = std::io::copy(&mut zip_file, &mut outfile);
                            }
                        }
                    }
                }
            }

            // 解压完毕后即刻清理大型 ZIP 压缩包原件，避免磁盘空间浪费
            let _ = fs::remove_file(&zip_dest);

            // 解析 content.json
            let content_file = part_dir.join("content.json");
            if content_file.exists()
                && let Ok(c_str) = fs::read_to_string(&content_file)
                && let Ok(content_val) = serde_json::from_str::<Value>(&c_str) {
                    let content2_val = part_dir.join("content2.json").exists().then(|| {
                            fs::read_to_string(part_dir.join("content2.json"))
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                        }).flatten();

                        let info_val = part_dir.join("info.json").exists().then(|| {
                            fs::read_to_string(part_dir.join("info.json"))
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                        }).flatten();

                        let structure_type = content_val.get("structure_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知题型")
                            .to_string();

                        let raw_questions = resolver::api::parse_task(
                            &content_val,
                            content2_val.as_ref(),
                            info_val.as_ref(),
                        );

                        let mut questions = Vec::new();
                        for raw_q in raw_questions {
                            question_counter += 1;
                            let no = raw_q.get("no").map(|v| match v {
                                Value::Number(n) => n.to_string(),
                                Value::String(s) => s.clone(),
                                _ => question_counter.to_string(),
                            }).unwrap_or_else(|| question_counter.to_string());

                            let cat = raw_q.get("category").and_then(|v| v.as_str()).unwrap_or(&structure_type).to_string();
                            let prompt = raw_q
                                .get("question")
                                .or_else(|| raw_q.get("text"))
                                .or_else(|| raw_q.get("prompt"))
                                .or_else(|| raw_q.get("ask"))
                                .or_else(|| raw_q.get("content"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let mut answers = Vec::new();
                            if let Some(ans_val) = raw_q.get("answers").or_else(|| raw_q.get("answer")) {
                                match ans_val {
                                    Value::Array(arr) => {
                                        for a in arr {
                                            if let Some(s) = a.as_str() {
                                                let s_trim = s.trim();
                                                if !s_trim.is_empty() {
                                                    answers.push(s_trim.to_string());
                                                }
                                            }
                                        }
                                    }
                                    Value::String(s) => {
                                        let s_trim = s.trim();
                                        if !s_trim.is_empty() {
                                            answers.push(s_trim.to_string());
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            let mut important_points = Vec::new();
                            if let Some(Value::Array(arr)) = raw_q.get("important") {
                                for item in arr {
                                    if let Some(s) = item.as_str()
                                        && !s.is_empty() {
                                            important_points.push(s.to_string());
                                        }
                                }
                            }

                            let mut options = Vec::new();
                            if let Some(opts) = raw_q.get("options").and_then(|v| v.as_array()) {
                                for opt in opts {
                                    match opt {
                                        Value::String(s) => {
                                            let trimmed = s.trim();
                                            let (label, text) = if let Some((l, r)) = trimmed.split_once('.') {
                                                if l.chars().count() == 1 {
                                                    (l.trim().to_string(), r.trim().to_string())
                                                } else {
                                                    ("".to_string(), trimmed.to_string())
                                                }
                                            } else {
                                                ("".to_string(), trimmed.to_string())
                                            };
                                            options.push(QuestionOption { label, text, image_path: None });
                                        }
                                        Value::Object(obj) => {
                                            let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let img = obj.get("image").or_else(|| obj.get("image_path")).and_then(|v| v.as_str()).map(|s| s.to_string());
                                            options.push(QuestionOption { label, text, image_path: img });
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            let audio_file = raw_q.get("audio").and_then(|v| v.as_str()).map(|s| s.to_string());

                            questions.push(ParsedQuestion {
                                index: question_counter - 1,
                                number: no,
                                category: cat,
                                prompt_text: prompt,
                                answers,
                                important_points,
                                audio_file,
                                options,
                            });
                        }

                        if !questions.is_empty() {
                            let friendly_title = match structure_type.as_str() {
                                "collector.choose" => "听后选择".to_string(),
                                "collector.read" => "短文朗读".to_string(),
                                "collector.picture" => "看图说话".to_string(),
                                "collector.fill" => "听后填空".to_string(),
                                "collector.role" | "collector.role_play" => "角色扮演".to_string(),
                                "collector.dialogue" => "听后回答 (简答)".to_string(),
                                "collector.repeat" | "collector.repeat_essay" | "collector.repeat_dialogue" => "短文复述".to_string(),
                                "collector.3q5a" => "三问五答".to_string(),
                                "collector.word" => "单词辨音".to_string(),
                                "collector.essay" => "情景表达".to_string(),
                                other => other.to_string(),
                            };

                            sections.push(ParsedSection {
                                section_type: structure_type,
                                title: friendly_title,
                                questions,
                            });
                        }
                    }
                }

        if sections.is_empty() {
            // 兜底：若 ZIP 下载未能解析出有效题目，尝试 4 级回退链
            if let Ok(pk_sections) = Self::fetch_and_parse_pk_homework(session, homework) {
                sections = pk_sections;
            }
        }

        if sections.is_empty() {
            return Err("未能解析出有效的试卷题目".to_string());
        }

        let total_q = sections.iter().map(|s| s.questions.len()).sum();

        Ok(HomeworkItem {
            id: homework.id.clone(),
            name: homework.name.clone(),
            status: "已下载完成".to_string(),
            deadline: homework.deadline.clone(),
            total_point: homework.total_point,
            awarded_point: homework.awarded_point,
            question_count: total_q,
            source: HomeworkSource::Cloud,
            local_dir: Some(temp_dir.to_string_lossy().to_string()),
            resource_urls: homework.resource_urls.clone(),
            sections,
            set_type: homework.set_type,
            pk: homework.pk,
            resource_id: homework.resource_id.clone(),
        })
    }

    /// 底层调用 ETS100 官方签名接口
    pub fn call_ets_api(endpoint: &str, route: &str, params: Value) -> Result<Value, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let body_json = serde_json::to_string(&json!([{"r": route, "params": params}]))
            .map_err(|e| format!("序列化请求失败: {e}"))?;
        let body_b64 = BASE64_STANDARD.encode(body_json.as_bytes());

        let sign = crate::themida_tiger_red!({
            let sign_input = format!("grlx{}{}{}", timestamp, body_b64, "555ffbe95ccf4e9535a110170b445ab8");
            let mut hasher = md5::Md5::new();
            hasher.update(sign_input.as_bytes());
            format!("{:x}", hasher.finalize())
        });

        let payload = json!({
            "body": body_b64,
            "head": {
                "version": "1.0",
                "sign": sign,
                "pid": "grlx",
                "time": timestamp
            }
        });
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| format!("序列化签名载荷失败: {e}"))?;

        let url = format!("https://api.ets100.com{endpoint}");
        let resp_res = ureq::post(&url)
            .timeout(Duration::from_secs(20))
            .set("User-Agent", "libcurl-agent/1.0")
            .set("Content-Type", "application/x-www-form-urlencoded")
            .set("Accept", "*/*")
            .send_string(&payload_str);

        let resp_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取 ETS 响应失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_text = resp.into_string().unwrap_or_default();
                return Err(format!("ETS HTTP 错误 ({code}): {err_text}"));
            }
            Err(e) => return Err(format!("ETS 请求网络失败: {e}")),
        };

        let mut val: Value = serde_json::from_str(&resp_str)
            .map_err(|e| format!("ETS 响应不是有效 JSON: {e}"))?;

        if let Some(arr) = val.as_array_mut()
            && !arr.is_empty() {
                val = arr.remove(0);
            }

        let code = val.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
        if code != 0 && code != 200 {
            let msg = val.get("msg").and_then(|v| v.as_str()).unwrap_or("未知错误");
            return Err(format!("ETS API 错误 ({code}): {msg}"));
        }

        if let Some(body_val) = val.get_mut("body") {
            Self::normalize_body(body_val);
        }

        Ok(val)
    }

    fn normalize_body(body_val: &mut Value) {
        if let Some(body_str) = body_val.as_str() {
            let body_trimmed = body_str.trim();
            let mut decoded_opt = None;
            if let Ok(decoded_bytes) = BASE64_STANDARD.decode(body_trimmed.as_bytes())
                && let Ok(decoded_json) = serde_json::from_slice::<Value>(&decoded_bytes) {
                    decoded_opt = Some(decoded_json);
                }
            if decoded_opt.is_none()
                && let Ok(direct_json) = serde_json::from_str::<Value>(body_trimmed) {
                    decoded_opt = Some(direct_json);
                }
            if let Some(mut parsed) = decoded_opt {
                if let Value::Array(arr) = parsed {
                    parsed = json!({ "data": arr });
                }
                *body_val = parsed;
            }
        } else if let Value::Array(arr) = body_val.clone() {
            *body_val = json!({ "data": arr });
        }
    }

    pub fn extract_body(val: &Value) -> Value {
        if let Some(body) = val.get("body") {
            body.clone()
        } else {
            val.clone()
        }
    }

    pub fn has_questions(val: &Value) -> bool {
        if let Value::Object(map) = val {
            for key in &["word_question", "word_questions", "questions", "data", "words", "word"] {
                if let Some(arr) = map.get(*key).and_then(Value::as_array)
                    && !arr.is_empty() {
                        return true;
                    }
            }
        }
        false
    }

    /// 获取词汇对战 PK 作业信息 (getPkInfo)
    /// POST /g/homework/pk-info
    pub fn get_pk_info(token: &str, homework_id: &str, parent_account_id: &str) -> Result<Value, String> {
        let hw_id_val = homework_id.parse::<i64>().map(|n| json!(n)).unwrap_or_else(|_| json!(homework_id));
        let mut params = json!({
            "sn": "test",
            "token": token,
            "homework_id": hw_id_val,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        if !parent_account_id.trim().is_empty() {
            params.as_object_mut().unwrap().insert("parent_account_id".to_string(), json!(parent_account_id));
        }
        let res = Self::call_ets_api("/g/homework/pk-info", "g/homework/pk-info", params)?;
        Ok(Self::extract_body(&res))
    }

    /// 获取作业/试卷结构 (getHomeworkStruct)
    /// POST /g/homework/struct
    pub fn get_homework_struct(token: &str, homework_id: &str, resource_id: &str, parent_account_id: &str) -> Result<Value, String> {
        let mut params = json!({
            "sn": "test",
            "token": token,
            "id": homework_id,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        if !resource_id.trim().is_empty() {
            params.as_object_mut().unwrap().insert("resource_id".to_string(), json!(resource_id));
        }
        if !parent_account_id.trim().is_empty() {
            params.as_object_mut().unwrap().insert("parent_account_id".to_string(), json!(parent_account_id));
        }
        let res = Self::call_ets_api("/g/homework/struct", "g/homework/struct", params)?;
        Ok(Self::extract_body(&res))
    }

    /// 获取历史作业得分与单词详情 (逆向历史作业) (getScoreDetail)
    /// POST /g/homework/score-detail
    pub fn get_score_detail(token: &str, homework_id: &str, resource_id: &str, parent_account_id: &str) -> Result<Value, String> {
        let hw_id_val = homework_id.parse::<i64>().map(|n| json!(n)).unwrap_or_else(|_| json!(homework_id));
        let mut params = json!({
            "sn": "test",
            "token": token,
            "homework_id": hw_id_val,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        if !resource_id.trim().is_empty() {
            params.as_object_mut().unwrap().insert("resource_id".to_string(), json!(resource_id));
        }
        if !parent_account_id.trim().is_empty() {
            params.as_object_mut().unwrap().insert("parent_account_id".to_string(), json!(parent_account_id));
        }
        let res = Self::call_ets_api("/g/homework/score-detail", "g/homework/score-detail", params)?;
        Ok(Self::extract_body(&res))
    }

    /// 批量获取单词与题目定义结构 (getWordStruct)
    /// POST /g/word/struct
    pub fn get_word_struct(token: &str, word_ids: &[i64]) -> Result<Value, String> {
        let params = json!({
            "sn": "test",
            "token": token,
            "word_ids": word_ids,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/g/word/struct", "g/word/struct", params)?;
        Ok(Self::extract_body(&res))
    }

    /// 阶段 0: 检查作业并开启做题流水线 (checkstart)
    pub fn check_start(token: &str, homework_id: &str) -> Result<Value, String> {
        let hw_id_num = homework_id.parse::<i64>().unwrap_or(0);
        let params = json!({
            "sn": "test",
            "token": token,
            "homework_id": hw_id_num,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/g/homework/checkstart", "g/homework/checkstart", params)?;
        Ok(res.get("body").cloned().unwrap_or(res))
    }

    /// 阶段 1: 批量注册/查询题目实体资源 (ppr-res)
    pub fn ppr_resources(token: &str, batch: &[Value]) -> Result<Value, String> {
        let params = json!({
            "sn": "test",
            "token": token,
            "batch": batch,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/g/set/ppr-res", "g/set/ppr-res", params)?;
        Ok(res.get("body").cloned().unwrap_or(res))
    }

    /// 阶段 2: 预申请文件上传唯一 ID (pre-upload)
    pub fn pre_upload(token: &str) -> Result<String, String> {
        let params = json!({
            "sn": "test",
            "token": token,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/m/file/pre-upload", "m/file/pre-upload", params)?;
        let body = res.get("body").unwrap_or(&res);
        if let Some(id) = body.get("id").and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }) {
            Ok(id)
        } else {
            Err("pre-upload 响应中未包含文件 ID".to_string())
        }
    }

    /// 阶段 3: 获取阿里云 OSS 上传策略凭据 (oss-config)
    pub fn get_oss_config(
        token: &str,
        dir_kind: &str,
        tag: &str,
        file_name: &str,
    ) -> Result<OssFormConfig, String> {
        let params = json!({
            "sn": "test",
            "token": token,
            "dir": dir_kind,
            "tag": tag,
            "name": file_name,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/system/oss-config", "system/oss-config", params)?;
        let body = res.get("body").unwrap_or(&res);

        let host = body.get("host").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let dir = body.get("dir").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let access_id = body.get("accessid").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let policy = body.get("policy").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let signature = body.get("signature").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let callback = body.get("callback").and_then(|v| v.as_str()).unwrap_or("").to_string();

        if access_id.is_empty() || policy.is_empty() {
            return Err("获取到的 OSS 上传凭据不完整".to_string());
        }

        Ok(OssFormConfig {
            host,
            dir,
            access_id,
            policy,
            signature,
            callback,
        })
    }

    /// 阶段 4-6: 同步单题评分、音频与 XML (sync-v2)
    pub fn sync_audio_v2(token: &str, mut params: Value) -> Result<Value, String> {
        if let Some(obj) = params.as_object_mut() {
            obj.insert("sn".to_string(), json!("test"));
            obj.insert("token".to_string(), json!(token));
            obj.insert("system".to_string(), json!("4"));
            obj.insert("global_client_version".to_string(), json!("5.4.5"));
            obj.insert("sign_response".to_string(), json!(1));
        }
        let res = Self::call_ets_api("/m/audio/sync-v2", "m/audio/sync-v2", params)?;
        Ok(res.get("body").cloned().unwrap_or(res))
    }

    /// 阶段 7: 提交作业总用时 (set-use-time)
    pub fn set_use_time(
        token: &str,
        homework_id: &str,
        resource_id: &str,
        total_seconds: u32,
    ) -> Result<Value, String> {
        let hw_id_num = homework_id.parse::<i64>().unwrap_or(0);
        let params = json!({
            "sn": "test",
            "token": token,
            "homework_id": hw_id_num,
            "resource_id": resource_id,
            "use_time": total_seconds,
            "reset": 0,
            "end": 1,
            "correct": 0,
            "version": 2,
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/m/homework/set-use-time", "m/homework/set-use-time", params)?;
        Ok(res.get("body").cloned().unwrap_or(res))
    }

    /// 阶段 8: 提交最终作业得分终审报告 (report-score)
    pub fn report_score(
        token: &str,
        homework_id: &str,
        resource_id: &str,
        parent_id: &str,
        score_rate: i64,
        total_seconds: u32,
    ) -> Result<Value, String> {
        let hw_id_num = homework_id.parse::<i64>().unwrap_or(0);
        let params = json!({
            "sn": "test",
            "token": token,
            "homework_id": hw_id_num,
            "resource_id": resource_id,
            "homework_score_rate": score_rate,
            "homework_complete_rate": 1_000_000,
            "homework_use_time": total_seconds,
            "is_confirm_commit": 1,
            "parent_account_id": parent_id,
            "seq_id": "",
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/g/homework/report-score", "g/homework/report-score", params)?;
        Ok(res.get("body").cloned().unwrap_or(res))
    }

    /// 阶段 0 (PK/词汇专项): 作业得分重置/掉落 (score-drop)
    pub fn score_drop(token: &str, homework_id: &str) -> Result<Value, String> {
        let hw_id_num = homework_id.parse::<i64>().unwrap_or(0);
        let params = json!({
            "sn": "test",
            "token": token,
            "homework_id": hw_id_num,
            "resource_id": "",
            "seq_id": "",
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        });
        let res = Self::call_ets_api("/g/homework/score-drop", "g/homework/score-drop", params)?;
        Ok(Self::extract_body(&res))
    }

    /// 阶段 1 (PK专项): 开启词汇对战会话 (start-word-pk)
    pub fn start_word_pk(token: &str, mut params: Value) -> Result<Value, String> {
        if let Some(obj) = params.as_object_mut() {
            obj.insert("sn".to_string(), json!("test"));
            obj.insert("token".to_string(), json!(token));
            obj.insert("version".to_string(), json!("1"));
            obj.insert("system".to_string(), json!("4"));
            obj.insert("global_client_version".to_string(), json!("5.4.5"));
            obj.insert("sign_response".to_string(), json!(1));
        }
        let res = Self::call_ets_api("/g/pk/start-word-pk", "g/pk/start-word-pk", params)?;
        Ok(Self::extract_body(&res))
    }

    /// 阶段 2 (PK专项): 提交词汇竞技对战成绩 (submit-word)
    pub fn submit_word_pk(token: &str, mut params: Value) -> Result<Value, String> {
        if let Some(obj) = params.as_object_mut() {
            obj.insert("sn".to_string(), json!("test"));
            obj.insert("token".to_string(), json!(token));
            obj.insert("version".to_string(), json!("3"));
            obj.insert("system".to_string(), json!("4"));
            obj.insert("global_client_version".to_string(), json!("5.4.5"));
            obj.insert("sign_response".to_string(), json!(1));
        }
        let res = Self::call_ets_api("/g/pk/submit-word", "g/pk/submit-word", params)?;
        Ok(Self::extract_body(&res))
    }

    /// 阶段 2 (词汇专项 set_type == 6): 同步词汇成绩与选择 (score-sync)
    pub fn score_sync(token: &str, mut params: Value) -> Result<Value, String> {
        if let Some(obj) = params.as_object_mut() {
            obj.insert("sn".to_string(), json!("test"));
            obj.insert("token".to_string(), json!(token));
            obj.insert("version".to_string(), json!("3"));
            obj.insert("system".to_string(), json!("4"));
            obj.insert("global_client_version".to_string(), json!("5.4.5"));
            obj.insert("sign_response".to_string(), json!(1));
        }
        let res = Self::call_ets_api("/g/homework/score-sync", "g/homework/score-sync", params)?;
        Ok(Self::extract_body(&res))
    }
}
