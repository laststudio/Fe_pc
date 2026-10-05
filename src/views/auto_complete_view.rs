// =========================================================================================
// ⚠️ 自动做题流水线视图排版规范：
// 1. 严格禁止使用不可折行的 `ui.row`。参数配置卡片、预设按钮、时长显示、音色选择全部必须使用 `responsive_flow`。
// 2. 按钮统一使用 `choice_btn` / `action_btn` / `accent_action_btn`，杜绝 FluentButton 测宽压缩。
// 3. 拟真用时与阶段标题等长文本确保自动换行，禁止将卡片撑出视口。
// =========================================================================================

use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui::{Color32, RichText, ScrollArea};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::auto_complete::{AutoCompleteWorkflow, AutoProgressMsg};
use crate::models::HomeworkItem;
use crate::state::{AppState, NavTab};
use crate::views::{
    accent_action_btn, action_btn, choice_btn, compact_slider, render_header, responsive_flow, truncate_middle,
};

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(
        ui,
        "⚡ 全自动做题流水线",
        "AI 语音合成、科大讯飞标准评分 XML 生成与官方 8 阶段做题协议簇 (Scheme B)",
    );

    // 轮询后台消息通道
    let mut finished = false;
    if let Some(ref rx) = state.auto_rx {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                AutoProgressMsg::Step { current, total, title } => {
                    state.auto_progress = Some((current, total, title.clone()));
                    state.auto_logs.push(format!("[{}/{}] {}", current, total, title));
                }
                AutoProgressMsg::Log(line) => {
                    state.auto_logs.push(line);
                }
                AutoProgressMsg::QuestionDone { number, category, score, max_score, status, audio_path, .. } => {
                    state.auto_questions_done.push((number, category, score, max_score, status, audio_path));
                }
                AutoProgressMsg::Complete { success, final_score, message } => {
                    finished = true;
                    state.auto_logs.push(format!("流水线执行结束: {} (最终得分: {:.1} 分)", message, final_score));
                    if success {
                        state.notification = Some((format!("作业自动完成成功！最终得分: {:.1} 分", final_score), InfoBarKind::Success));
                    } else {
                        state.notification = Some((format!("流水线未完全成功: {}", message), InfoBarKind::Warning));
                    }
                }
            }
        }
    }
    if finished {
        state.auto_running = false;
        state.auto_rx = None;
    }

    // 1. 强制要求登录 LA Studio 账号方可使用全自动做题
    if state.la_session.is_none() {
        ui.card("🔒 会员专享功能受限", "全自动做题流水线需登录 LA Studio 账号", |ui| {
            ui.label(
                RichText::new("全自动做题流水线（AI 语音合成、科大讯飞标准评分生成与官方 8 阶段做题协议簇）为 LA Studio 会员专享高级功能。")
                    .color(Color32::from_rgb(255, 200, 50))
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label("系统需要连接至 LA Studio API 接口，安全拉取绑定的 ETS100 云端学生会话与做题令牌后方可运行。");
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                RichText::new("💡 温馨提示：基础题目与答案解析功能无需 LA 账号，完全开放使用！")
                    .color(Color32::from_rgb(76, 175, 80))
                    .strong(),
            );
            ui.label("• 您可以随时在【作业与试卷】中浏览题目结构、查看试卷解析与极简答案速查；");
            ui.label("• 您也可以在【本地试卷扫描】中直读电脑本地已下载的试卷数据并导出为 Markdown/文本；");
            ui.label("• 您也可以在【云端账号管理】中切换到官方直连模式，直接使用 ETS 手机号密码查题。");
            ui.add_space(12.0);

            responsive_flow(ui, |ui| {
                if accent_action_btn(ui, "🔑 前往登录 LA Studio 账号", None).clicked() {
                    state.current_tab = NavTab::CloudAccount;
                    state.la_active_tab = 0;
                }
                if action_btn(ui, "📚 返回作业与试卷列表 (基础解析)").clicked() {
                    state.current_tab = NavTab::HomeworkList;
                }
                if action_btn(ui, "📁 前往本地试卷扫描 (离线题库)").clicked() {
                    state.current_tab = NavTab::LocalScan;
                }
            });
        });
        return;
    }

    // 2. 检查是否已通过 LA API 获取云端 ETS 做题凭据
    let la_sess = state.la_session.clone().unwrap();
    if state.la_cloud_session.is_none() || state.session.token.trim().is_empty() {
        ui.card("⚠️ 尚未同步云端 ETS100 做题凭据", "已登录 LA 账号，但尚未拉取云端学生会话", |ui| {
            ui.label(
                RichText::new(format!(
                    "👤 当前 LA 账号: {} [{}]",
                    la_sess.user.username,
                    la_sess.user.tier_display_name()
                ))
                .strong()
                .size(15.0)
                .color(Color32::from_rgb(0, 180, 255)),
            );
            ui.add_space(4.0);
            ui.label("全自动答题需要通过 LA API 中继服务获取您在云端绑定的 ETS100 做题会话。");
            ui.add_space(8.0);

            responsive_flow(ui, |ui| {
                if accent_action_btn(ui, "⚡ 立即通过 LA API 获取云端做题凭据", None).clicked() {
                    match crate::la_client::LaClient::get_cloud_session(&la_sess.access_token, "") {
                        Ok(cloud) => {
                            state.session = crate::models::UserSession {
                                phone: cloud.account_display.clone(),
                                token: cloud.token.clone(),
                                device_code: cloud.device_code.clone(),
                                parent_account_id: cloud.parent_account_id.clone(),
                                user_name: cloud.account_display.clone(),
                                is_logged_in: true,
                            };
                            state.la_cloud_session = Some(cloud);
                            state.notification = Some(("云端 ETS100 做题凭据获取成功！".to_string(), InfoBarKind::Success));
                            if let Ok(hw_list) = crate::cloud_client::CloudClient::get_homework_list(&state.session, "1") {
                                state.homework_items = hw_list;
                            }
                        }
                        Err(e) => {
                            state.notification = Some((
                                format!("获取云端做题凭据失败: {e}。若未绑定 ETS 账号，请先在【账号中心】绑定。"),
                                InfoBarKind::Error,
                            ));
                        }
                    }
                }

                if action_btn(ui, "⚙ 前往账号中心关联/绑定 ETS 账号").clicked() {
                    state.current_tab = NavTab::CloudAccount;
                    state.la_active_tab = 0;
                }
            });
        });
        return;
    }

    // 3. LA 账号及云端 ETS 会话有效，展示授权状态条
    let student_display = state
        .la_cloud_session
        .as_ref()
        .map(|c| c.account_display.clone())
        .unwrap_or_else(|| "已激活".to_string());

    ui.card("🟢 LA Studio 云端授权有效", "已接入云端中继代理，可随时启动全自动做题流水线", |ui| {
        responsive_flow(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "👤 LA 会员: {} [{}]",
                    la_sess.user.username,
                    la_sess.user.tier_display_name()
                ))
                .strong()
                .color(Color32::from_rgb(0, 180, 255)),
            );
            ui.label(" | ");
            ui.label(
                RichText::new(format!("🎓 云端学生: {student_display}"))
                    .strong()
                    .color(Color32::from_rgb(76, 175, 80)),
            );

            if action_btn(ui, "🔄 刷新云端凭据").clicked() {
                match crate::la_client::LaClient::get_cloud_session(&la_sess.access_token, "") {
                    Ok(cloud) => {
                        state.session = crate::models::UserSession {
                            phone: cloud.account_display.clone(),
                            token: cloud.token.clone(),
                            device_code: cloud.device_code.clone(),
                            parent_account_id: cloud.parent_account_id.clone(),
                            user_name: cloud.account_display.clone(),
                            is_logged_in: true,
                        };
                        state.la_cloud_session = Some(cloud);
                        state.notification = Some(("云端做题凭据已成功刷新！".to_string(), InfoBarKind::Success));
                    }
                    Err(e) => {
                        state.notification = Some((format!("刷新凭据失败: {e}"), InfoBarKind::Error));
                    }
                }
            }
        });
    });
    ui.add_space(8.0);

    // 整合所有可选作业
    let all_homeworks: Vec<HomeworkItem> = state
        .homework_items
        .iter()
        .cloned()
        .chain(state.local_items.iter().cloned())
        .collect();

    if all_homeworks.is_empty() {
        ui.card("未找到可做作业", "请先在【作业列表】拉取云端作业或在【本地扫描】中检测本地试卷", |ui| {
            responsive_flow(ui, |ui| {
                if accent_action_btn(ui, "📚 前往作业列表", None).clicked() {
                    state.current_tab = NavTab::HomeworkList;
                }
            });
        });
        return;
    }

    // 默认选中第一个作业
    if state.auto_selected_hw_id.is_none()
        && let Some(first) = all_homeworks.first() {
            state.auto_selected_hw_id = Some(first.id.clone());
            if state.auto_config.use_time_seconds == 0 || state.auto_config.use_time_seconds == 180 {
                state.auto_config.use_time_seconds = crate::workflow::scoring::calculate_recommended_use_time(first);
            }
        }

    let active_hw = state
        .auto_selected_hw_id
        .as_ref()
        .and_then(|id| all_homeworks.iter().find(|h| &h.id == id));

    if let Some(hw) = active_hw
        && (state.auto_config.use_time_seconds == 0 || state.auto_config.use_time_seconds == 180)
    {
        state.auto_config.use_time_seconds = crate::workflow::scoring::calculate_recommended_use_time(hw);
    }

    let selected_hw_name = active_hw
        .map(|h| h.name.clone())
        .unwrap_or_else(|| "未选择作业".to_string());

    let max_possible_score = active_hw.map(|h| h.total_point).unwrap_or(30.0);

    if let Some(hw) = active_hw
        && let Err(compat_err) = crate::auto_complete::check_homework_compatibility(hw) {
            ui.card("⚠️ 题型兼容性预警", "该作业可能包含全自动流水线暂不支持的题型", |ui| {
                ui.label(RichText::new(&compat_err).color(Color32::from_rgb(255, 100, 100)).strong());
            });
            ui.add_space(4.0);
    }

    ui.card("流水线参数配置", "设置目标分数、语音合成音色、用时规划与答题模式", |ui| {
        ui.label(
            RichText::new(format!("当前目标作业: {} (满分: {:.1} 分)", selected_hw_name, max_possible_score))
                .strong()
                .size(15.0)
                .color(Color32::from_rgb(0, 180, 255)),
        );

        // 进度展示与音波/呼吸动效
        if let Some((cur, tot, title)) = &state.auto_progress {
            ui.separator();
            let progress_val = (*cur as f32) / (*tot as f32).max(1.0);
            let smooth_progress = ui.ctx().animate_value_with_time(
                ui.id().with("auto_progress_smooth"),
                progress_val,
                0.35,
            );

            let t = ui.input(|i| i.time as f32);
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner = if state.auto_running {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
                spinner_frames[((t * 12.0) as usize) % spinner_frames.len()]
            } else {
                "✔"
            };

            let pulse = if state.auto_running {
                ((t * 4.0).sin() * 0.25 + 0.75).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let stage_color = Color32::from_rgb(
                (0.0 * pulse) as u8,
                (220.0 * pulse) as u8,
                (130.0 * pulse) as u8,
            );

            responsive_flow(ui, |ui| {
                ui.label(
                    RichText::new(format!("{spinner} 当前阶段: [{cur}/{tot}] - {title}"))
                        .color(stage_color)
                        .strong(),
                );

                if state.auto_running {
                    render_soundwave_visualizer(ui, t);
                }
            });

            ui.progress(smooth_progress);
        }

        ui.separator();

        // 模式切换
        responsive_flow(ui, |ui| {
            ui.label(RichText::new("做题工作流:").strong());
            let is_auto = state.auto_config.mode != "manual";
            if choice_btn(ui, "🚀 一键全自动模式", is_auto, state.theme_config.accent).clicked() {
                state.auto_config.mode = "auto".to_string();
            }
            if choice_btn(ui, "✍ 手动逐题控分模式", !is_auto, state.theme_config.accent).clicked() {
                state.auto_config.mode = "manual".to_string();
            }
        });

        ui.separator();

        // 根据模式展示目标分或逐题控分
        if state.auto_config.mode != "manual" {
            let slider_max = if state.auto_config.allow_over_max {
                (max_possible_score * 1.5).max(100.0) as f32
            } else {
                max_possible_score as f32
            };
            let safety_ceiling = max_possible_score * 0.95;

            responsive_flow(ui, |ui| {
                ui.label(format!("目标总分 (0.0 ~ {:.1} 分):", slider_max));
                let mut score_f32 = state.auto_config.target_total as f32;
                if compact_slider(ui, egui::Slider::new(&mut score_f32, 10.0..=slider_max).show_value(false)).changed() {
                    state.auto_config.target_total = (score_f32 * 10.0).round() as f64 / 10.0;
                }
                ui.label(
                    RichText::new(format!("{:.1} 分", state.auto_config.target_total))
                        .color(Color32::from_rgb(0, 180, 255))
                        .strong(),
                );
                if state.auto_config.target_total > safety_ceiling && !state.auto_config.allow_over_max {
                    ui.label(RichText::new("⚠️ 接近满分，建议 ≤ 95% 规避平台风控").color(Color32::from_rgb(255, 165, 0)).size(11.5));
                }
            });

            // 加权拟真随机分配抽样
            responsive_flow(ui, |ui| {
                ui.label(RichText::new("拟真加权扣分方案:").size(12.0));
                if action_btn(ui, "🎲 换一批分配方案 (重新抽样)").clicked() {
                    state.auto_config.score_reroll_nonce = state.auto_config.score_reroll_nonce.wrapping_add(1);
                    state.notification = Some(("已使用新随机种子重新生成各题扣分方案！".to_string(), InfoBarKind::Success));
                }
            });
        } else {
            // 手动逐题模式
            ui.label(RichText::new("✍ 手动逐题自定义控分:").strong().color(Color32::from_rgb(0, 180, 255)));
            responsive_flow(ui, |ui| {
                ui.label("快捷批量预设:");
                if action_btn(ui, "95% 安全比例").clicked()
                    && let Some(hw) = active_hw {
                    let q_list: Vec<_> = hw.sections.iter().flat_map(|s| &s.questions).cloned().collect();
                    let allocs = crate::workflow::scoring::plan_question_allocations(&q_list, hw.total_point, 0, false);
                    for (idx, alloc) in allocs.iter().enumerate() {
                        let max_q = alloc.max_score;
                        state.auto_config.per_question_scores.insert(idx, crate::workflow::scoring::round_points(max_q * 0.95));
                    }
                }
                if action_btn(ui, "90% 良好比例").clicked()
                    && let Some(hw) = active_hw {
                    let q_list: Vec<_> = hw.sections.iter().flat_map(|s| &s.questions).cloned().collect();
                    let allocs = crate::workflow::scoring::plan_question_allocations(&q_list, hw.total_point, 0, false);
                    for (idx, alloc) in allocs.iter().enumerate() {
                        let max_q = alloc.max_score;
                        state.auto_config.per_question_scores.insert(idx, crate::workflow::scoring::round_points(max_q * 0.90));
                    }
                }
                if action_btn(ui, "85% 稳健比例").clicked()
                    && let Some(hw) = active_hw {
                    let q_list: Vec<_> = hw.sections.iter().flat_map(|s| &s.questions).cloned().collect();
                    let allocs = crate::workflow::scoring::plan_question_allocations(&q_list, hw.total_point, 0, false);
                    for (idx, alloc) in allocs.iter().enumerate() {
                        let max_q = alloc.max_score;
                        state.auto_config.per_question_scores.insert(idx, crate::workflow::scoring::round_points(max_q * 0.85));
                    }
                }
                if action_btn(ui, "100% 满分").clicked()
                    && let Some(hw) = active_hw {
                    let q_list: Vec<_> = hw.sections.iter().flat_map(|s| &s.questions).cloned().collect();
                    let allocs = crate::workflow::scoring::plan_question_allocations(&q_list, hw.total_point, 0, false);
                    for (idx, alloc) in allocs.iter().enumerate() {
                        let max_q = alloc.max_score;
                        state.auto_config.per_question_scores.insert(idx, max_q);
                    }
                }
            });

            // 逐题微调滚动列表
            if let Some(hw) = active_hw {
                let questions: Vec<_> = hw.sections.iter().flat_map(|s| &s.questions).cloned().collect();
                if !questions.is_empty() {
                    let allocs = crate::workflow::scoring::plan_question_allocations(&questions, hw.total_point, 0, false);
                    ScrollArea::vertical()
                        .id_salt("manual_questions_scroll")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (idx, q) in questions.iter().enumerate() {
                                let max_q = allocs.get(idx).map(|a| a.max_score).unwrap_or(2.0);
                                let cur_val = state.auto_config.per_question_scores.entry(idx).or_insert_with(|| crate::workflow::scoring::round_points(max_q * 0.95));
                                responsive_flow(ui, |ui| {
                                    ui.label(RichText::new(format!("Q{} ({}) 满分:{:.1}", idx + 1, q.category, max_q)).size(11.5));
                                    let mut v = *cur_val as f32;
                                    if compact_slider(ui, egui::Slider::new(&mut v, 0.0..=max_q as f32).show_value(false)).changed() {
                                        *cur_val = (v * 10.0).round() as f64 / 10.0;
                                    }
                                    ui.label(format!("{:.1}分", *cur_val));
                                    if action_btn(ui, "📁 指定录音").clicked()
                                        && let Some(path) = rfd::FileDialog::new().add_filter("音频", &["wav", "mp3", "m4a"]).pick_file() {
                                        state.auto_config.per_question_audio.insert(idx, path.to_string_lossy().to_string());
                                    }
                                    if let Some(p) = state.auto_config.per_question_audio.get(&idx) {
                                        let name = std::path::Path::new(p).file_name().and_then(|n| n.to_str()).unwrap_or("音频");
                                        ui.label(RichText::new(format!("已指定: {}", name)).size(10.5).color(Color32::from_rgb(0, 180, 255)));
                                    }
                                });
                            }
                        });
                }
            }
        }

        ui.separator();

        // 答题用时规划
        responsive_flow(ui, |ui| {
            ui.label("答题用时来源:");
            let is_smart = state.auto_config.use_time_source != "manual";
            if choice_btn(ui, "智能耗时计算 (推荐)", is_smart, state.theme_config.accent).clicked() {
                state.auto_config.use_time_source = "smart".to_string();
            }
            if choice_btn(ui, "自定义耗时", !is_smart, state.theme_config.accent).clicked() {
                state.auto_config.use_time_source = "manual".to_string();
            }
        });

        if state.auto_config.use_time_source == "smart" {
            if let Some(hw) = active_hw {
                let (smart_secs, breakdown) = crate::workflow::scoring::calculate_smart_use_time(hw, state.auto_config.time_reroll_nonce);
                responsive_flow(ui, |ui| {
                    let m = smart_secs / 60;
                    let s = smart_secs % 60;
                    ui.label(RichText::new(format!(
                        "⏱ 拟真预计用时: {} 分 {} 秒 (听题 {}s + 准备 {}s + 作答 {}s + 缓冲 {}s)",
                        m, s, breakdown.listen_seconds, breakdown.prep_seconds, breakdown.answer_seconds, breakdown.overhead_seconds
                    )).color(Color32::from_rgb(76, 175, 80)).strong());

                    if action_btn(ui, "🎲 换个尾数").clicked() {
                        state.auto_config.time_reroll_nonce = state.auto_config.time_reroll_nonce.wrapping_add(1);
                    }
                });
            }
        } else {
            responsive_flow(ui, |ui| {
                ui.label("自定义耗时:");
                let mut time_val = state.auto_config.use_time_seconds as f32;
                if compact_slider(ui, egui::Slider::new(&mut time_val, 30.0..=900.0).show_value(false)).changed() {
                    state.auto_config.use_time_seconds = time_val as u32;
                }
                let m = state.auto_config.use_time_seconds / 60;
                let s = state.auto_config.use_time_seconds % 60;
                ui.label(format!("{} 分 {} 秒 (共 {} 秒)", m, s, state.auto_config.use_time_seconds));
            });
        }

        ui.separator();

        // 语音合成与声音克隆矩阵配置
        let active_p = state.tts_settings.active_provider.clone();
        responsive_flow(ui, |ui| {
            ui.label(RichText::new("当前活动引擎:").strong());
            match active_p.as_str() {
                "edge" => {
                    ui.label(RichText::new("🌐 微软 Edge 官方语音 (免配置 · 完全免费 · 超拟真自然音色)").strong().color(Color32::from_rgb(76, 175, 80)));
                }
                "fish_audio" => {
                    ui.label(RichText::new("🐟 Fish Audio (s2.1-pro-free 免 Key 免费模型 · 零样本极速克隆)").strong().color(Color32::from_rgb(0, 200, 255)));
                }
                "siliconflow" => {
                    ui.label(RichText::new("⚡ 硅基流动 (CosyVoice2-0.5B / Fish-Speech · 低成本极速克隆)").strong().color(Color32::from_rgb(255, 170, 0)));
                }
                "mimo" => {
                    ui.label(RichText::new("📱 小米 MiMo (v2.5 原生声音克隆协议)").strong().color(Color32::from_rgb(255, 120, 50)));
                }
                "minimax" => {
                    ui.label(RichText::new("🎙 MiniMax 海螺 (speech-2.8-turbo/hd · 极致拟真呼吸感)").strong().color(Color32::from_rgb(200, 100, 255)));
                }
                "cloudflare" => {
                    ui.label(RichText::new("☁ Cloudflare Workers AI (每日免费 10000 神经元)").strong().color(Color32::from_rgb(255, 150, 50)));
                }
                _ => {
                    ui.label(RichText::new("🛠 自定义 HTTP 接口 (兼容 OpenAI 规范)").strong().color(Color32::from_gray(180)));
                }
            }

            if action_btn(ui, "⚙ 矩阵详细配置与测速诊断").clicked() {
                state.current_tab = NavTab::Settings;
            }
        });

        // 快捷切换提供商矩阵栏
        responsive_flow(ui, |ui| {
            ui.label("快捷切换引擎:");
            let providers = [
                ("edge", "🌐 微软 Edge (免费)"),
                ("fish_audio", "🐟 Fish Audio (免Key)"),
                ("siliconflow", "⚡ 硅基流动"),
                ("mimo", "📱 小米 MiMo"),
                ("minimax", "🎙 MiniMax 海螺"),
                ("cloudflare", "☁ Cloudflare"),
                ("custom_http", "🛠 自定义 HTTP"),
            ];

            for (p_id, p_label) in providers {
                let is_sel = state.tts_settings.active_provider == p_id;
                if choice_btn(ui, p_label, is_sel, state.theme_config.accent).clicked() {
                    state.tts_settings.active_provider = p_id.to_string();
                    let _ = state.tts_settings.save();
                    state.auto_config.tts_mode = match p_id {
                        "edge" => "edge".to_string(),
                        "fish_audio" | "siliconflow" | "mimo" => "clone".to_string(),
                        _ => "http".to_string(),
                    };
                }
            }
        });

        // 根据当前引擎展示上下文感知的子面板
        match state.tts_settings.active_provider.as_str() {
            "edge" => {
                responsive_flow(ui, |ui| {
                    ui.label("精选自然音色:");
                    let voices = [
                        ("en-US-AvaNeural", "Ava (新一代自然美音女声·推荐)"),
                        ("en-US-AndrewNeural", "Andrew (新一代自然美音男声·推荐)"),
                        ("en-US-JennyNeural", "Jenny (标准美音女声)"),
                        ("en-US-GuyNeural", "Guy (标准美音男声)"),
                        ("en-US-AriaNeural", "Aria (自然抑扬女声)"),
                        ("en-US-BrianNeural", "Brian (标准国际男声)"),
                        ("zh-CN-XiaoxiaoNeural", "晓晓 (中文普通话女声)"),
                        ("zh-CN-YunxiNeural", "云希 (中文阳光男声)"),
                    ];

                    for (v_id, v_label) in voices {
                        let is_selected = state.auto_config.voice_name == v_id || state.tts_settings.edge.voice == v_id;
                        if choice_btn(ui, v_label, is_selected, state.theme_config.accent).clicked() {
                            state.auto_config.voice_name = v_id.to_string();
                            state.tts_settings.edge.voice = v_id.to_string();
                            let _ = state.tts_settings.save();
                        }
                    }
                });
            }
            "fish_audio" | "siliconflow" | "mimo" => {
                responsive_flow(ui, |ui| {
                    ui.label("本地参考录音:");
                    if action_btn(ui, "📁 选取录音文件 (.wav/.mp3)").clicked()
                        && let Some(path) = rfd::FileDialog::new().add_filter("音频文件", &["wav", "mp3", "m4a"]).pick_file() {
                        state.auto_config.clone_reference_path = path.to_string_lossy().to_string();
                    }
                    if !state.auto_config.clone_reference_path.is_empty() {
                        let short_p = truncate_middle(&state.auto_config.clone_reference_path, 36);
                        ui.label(RichText::new(short_p).size(11.0).color(Color32::from_rgb(0, 180, 255)))
                            .on_hover_text(&state.auto_config.clone_reference_path);
                        if action_btn(ui, "✕ 清除").clicked() {
                            state.auto_config.clone_reference_path.clear();
                        }
                    } else if state.tts_settings.active_provider == "fish_audio" {
                        ui.label(RichText::new("💡 未选择录音时，Fish Audio 将以官方预设人声朗读（无需配置 Key）").size(11.0).color(Color32::from_rgb(0, 200, 255)));
                    } else {
                        ui.label(RichText::new("⚠️ 未选择录音时，将自动平滑回退至微软 Edge 官方音色").size(11.0).color(Color32::from_gray(160)));
                    }
                });

                if let Some(la_sess) = &state.la_session {
                    responsive_flow(ui, |ui| {
                        ui.label(RichText::new("LA 云端声音资产漫游:").size(12.0));
                        let sync_label = if state.la_voices_loading { "🔄 同步中..." } else { "🔄 同步云端声音" };
                        if action_btn(ui, sync_label).clicked() {
                            let token = la_sess.access_token.clone();
                            match crate::la_client::LaClient.get_user_voices(&token) {
                                Ok(voices) => {
                                    state.la_voices = voices;
                                    state.notification = Some((format!("成功同步 {} 个云端声音！", state.la_voices.len()), InfoBarKind::Success));
                                }
                                Err(e) => {
                                    state.notification = Some((format!("同步云端声音失败: {e}"), InfoBarKind::Warning));
                                }
                            }
                        }
                    });

                    if !state.la_voices.is_empty() {
                        responsive_flow(ui, |ui| {
                            ui.label("选择云端音色:");
                            for v in &state.la_voices {
                                let is_active = state.auto_config.clone_voice_id == v.id;
                                let label = format!("{} ({}s)", v.name, v.duration.unwrap_or(10.0) as u32);
                                if choice_btn(ui, &label, is_active, state.theme_config.accent).clicked() {
                                    state.auto_config.clone_voice_id = v.id.clone();
                                    let cache_dir = std::env::temp_dir().join("fepc_voices");
                                    let dest = cache_dir.join(format!("{}.wav", v.id));
                                    if crate::la_client::LaClient.download_voice_audio(&v.id, &la_sess.access_token, &dest).is_ok() {
                                        state.auto_config.clone_reference_path = dest.to_string_lossy().to_string();
                                        state.notification = Some((format!("已加载云端声音: {}", v.name), InfoBarKind::Success));
                                    }
                                }
                            }
                        });
                    }
                }
            }
            "minimax" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new(format!(
                        "MiniMax 模型: {} (已配 Key: {})",
                        state.tts_settings.minimax.model,
                        if state.tts_settings.minimax.api_key.trim().is_empty() { "未填·请在设置中配置" } else { "已就绪" }
                    )).size(11.5).color(Color32::from_rgb(200, 100, 255)));
                });
            }
            "cloudflare" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new(format!(
                        "Cloudflare 模型: {} (Account ID: {})",
                        state.tts_settings.cloudflare.model,
                        if state.tts_settings.cloudflare.account_id.trim().is_empty() { "未配置" } else { "已就绪" }
                    )).size(11.5).color(Color32::from_rgb(255, 150, 50)));
                });
            }
            _ => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new(format!(
                        "自定义 HTTP 端点: {} (模型: {})",
                        if state.tts_settings.custom_http.endpoint_url.trim().is_empty() { "未配置" } else { state.tts_settings.custom_http.endpoint_url.as_str() },
                        state.tts_settings.custom_http.model
                    )).size(11.5).color(Color32::from_gray(180)));
                });
            }
        }

        responsive_flow(ui, |ui| {
            ui.add(fluent_egui::winui::ToggleSwitch::new(&mut state.auto_config.no_upload));
            ui.label(RichText::new("本地调试运行模式，不向官方服务器回传数据 (--no-upload)").size(12.5));
        });

        responsive_flow(ui, |ui| {
            ui.add(fluent_egui::winui::ToggleSwitch::new(&mut state.auto_config.allow_over_max));
            ui.label(RichText::new("允许超过作业满分 (测试模式)").size(12.5));
        });

        ui.separator();

        responsive_flow(ui, |ui| {
            let is_killed = state.software_policy.as_ref().is_some_and(|p| p.is_kill_switch_on);
            let check = state.check_feature("auto_complete");
            let v_check = if state.tts_settings.active_provider != "edge" {
                state.check_feature("voice_clone")
            } else {
                crate::la_client::FeatureCheckResult {
                    allowed: true,
                    reason: String::new(),
                    required_tier: String::new(),
                    requires_login: false,
                    requires_membership: false,
                }
            };

            let can_start = !is_killed
                && state.auto_selected_hw_id.is_some()
                && !state.auto_running
                && check.allowed
                && v_check.allowed
                && state.la_session.is_some()
                && !state.session.token.trim().is_empty();

            let btn_label = if !check.allowed {
                if check.requires_login {
                    "🔐 登录后使用自动做题流水线"
                } else {
                    "👑 会员自动做题流水线"
                }
            } else if !v_check.allowed {
                "👑 会员语音克隆流水线"
            } else {
                "🚀 开始全自动做题流水线"
            };

            let btn_clicked = if can_start {
                accent_action_btn(ui, btn_label, None).clicked()
            } else {
                action_btn(ui, btn_label).clicked()
            };

            if is_killed {
                ui.label(RichText::new("🛑 服务已被紧急停用 (Kill Switch 激活)，流水线已禁用").color(Color32::from_rgb(255, 100, 100)).strong());
            }

            if btn_clicked {
                if is_killed {
                    state.notification = Some(("客户端服务已被紧急停用 (Kill Switch 激活)，无法启动自动化流水线".to_string(), InfoBarKind::Error));
                } else if !check.allowed {
                    state.trigger_auth_gate_modal("auto_complete", "作业自动完成流水线");
                } else if !v_check.allowed {
                    state.trigger_auth_gate_modal("voice_clone", "商业级语音克隆");
                } else if can_start
                    && let Some(hw) = all_homeworks.iter().find(|h| Some(&h.id) == state.auto_selected_hw_id.as_ref()) {
                        let hw_clone = hw.clone();
                        let session_clone = state.session.clone();
                        let config_clone = state.auto_config.clone();

                        state.auto_running = true;
                        state.auto_progress = Some((0, 8, "正在初始化流水线...".to_string()));
                        state.auto_logs.clear();
                        state.auto_questions_done.clear();
                        state.auto_logs.push(format!("开始执行作业 [{}] 自动化流水线...", hw_clone.name));

                        let (tx, rx): (Sender<AutoProgressMsg>, Receiver<AutoProgressMsg>) = channel();
                        state.auto_rx = Some(rx);

                        std::thread::spawn(move || {
                            AutoCompleteWorkflow::run(session_clone, hw_clone, config_clone, tx);
                        });

                        state.notification = Some(("自动化答题流水线已成功在后台启动！".to_string(), InfoBarKind::Success));
                    }
            }

            if state.auto_running {
                ui.label("⏳ 任务正在执行中，请勿关闭程序...");
            }
        });
    });

    ui.add_space(8.0);

    // 题目实时完成状态与得分明细表格
    if !state.auto_questions_done.is_empty() {
        ui.heading("题目评分与上传明细:");
        ui.card("实时题目评分与录音试听看板", "展示逐题生成的打分、状态与本地音频回放", |ui| {
            ScrollArea::vertical()
                .id_salt("auto_questions_table")
                .max_height(160.0)
                .show(ui, |ui| {
                    for (num, cat, score, max, status, maybe_audio) in &state.auto_questions_done {
                        responsive_flow(ui, |ui| {
                            ui.label(RichText::new(format!("第 {} 题", num)).strong());
                            ui.label(format!("题型: {cat}"));
                            ui.label(
                                RichText::new(format!("得分: {:.2} / {:.2}", score, max))
                                    .color(Color32::from_rgb(76, 175, 80))
                                    .strong(),
                            );
                            ui.label(format!("状态: {status}"));
                            if let Some(audio_path) = maybe_audio
                                && action_btn(ui, "▶ 试听").clicked() {
                                let _ = open::that(audio_path);
                            }
                        });
                    }
                });
        });
        ui.add_space(8.0);
    }

    // 快捷作业选择列表
    ui.heading("选择要完成的作业:");
    ScrollArea::vertical()
        .id_salt("auto_homework_picker")
        .max_height(180.0)
        .show(ui, |ui| {
            for hw in &all_homeworks {
                let is_selected = state.auto_selected_hw_id.as_ref() == Some(&hw.id);
                let btn_text = if is_selected {
                    format!("👉 [已选] {}", hw.name)
                } else {
                    format!("选择: {}", hw.name)
                };

                responsive_flow(ui, |ui| {
                    if choice_btn(ui, &btn_text, is_selected, state.theme_config.accent).clicked() {
                        let changed = state.auto_selected_hw_id.as_deref() != Some(&hw.id);
                        state.auto_selected_hw_id = Some(hw.id.clone());
                        if changed || state.auto_config.use_time_seconds == 0 || state.auto_config.use_time_seconds == 180 {
                            state.auto_config.use_time_seconds = crate::workflow::scoring::calculate_recommended_use_time(hw);
                        }
                    }
                    ui.label(format!("(题数: {} | 满分: {})", hw.question_count, hw.total_point));
                });
            }
        });

    ui.add_space(8.0);

    // 运行日志输出控制台
    ui.heading("流水线控制台输出 (Live Logs):");
    ui.card("运行日志终端", "实时显示 8 阶段完成进度与协议交互信息", |ui| {
        ScrollArea::vertical()
            .id_salt("auto_live_logs")
            .max_height(180.0)
            .stick_to_bottom(true)
            .show(ui, |ui_raw| {
                if state.auto_logs.is_empty() {
                    ui_raw.label(RichText::new("等待启动任务...").color(Color32::from_gray(140)));
                } else {
                    for line in &state.auto_logs {
                        let color = if line.contains("🎉") || line.contains("成功") || line.contains("完成") {
                            Color32::from_rgb(76, 175, 80)
                        } else if line.contains("✓") {
                            Color32::from_rgb(0, 200, 255)
                        } else if line.contains("阶段") || line.contains("开始") {
                            Color32::from_rgb(255, 200, 50)
                        } else if line.contains("失败") || line.contains("❌") {
                            Color32::from_rgb(255, 82, 82)
                        } else {
                            Color32::from_gray(210)
                        };
                        ui_raw.label(RichText::new(line).size(13.0).color(color));
                    }
                }
            });
    });
}

fn render_soundwave_visualizer(ui: &mut eframe::egui::Ui, time: f32) {
    let (rect, _) = ui.allocate_exact_size(eframe::egui::vec2(100.0, 18.0), eframe::egui::Sense::hover());
    let painter = ui.painter();
    let num_bars = 9;
    let bar_width = 3.5;
    let spacing = (rect.width() - (num_bars as f32 * bar_width)) / ((num_bars - 1) as f32);

    for i in 0..num_bars {
        let x = rect.min.x + i as f32 * (bar_width + spacing);
        let phase = i as f32 * 0.85;
        let factor = 0.25 + 0.75 * ((time * 6.5 + phase).sin().abs() * 0.65 + (time * 11.0 + phase * 1.6).cos().abs() * 0.35);
        let bar_h = (rect.height() * factor).clamp(3.0, rect.height());
        let y = rect.center().y - bar_h / 2.0;

        let color_t = (i as f32) / ((num_bars - 1) as f32);
        let r = (0.0 * (1.0 - color_t) + 140.0 * color_t) as u8;
        let g = (200.0 * (1.0 - color_t) + 120.0 * color_t) as u8;
        let b = 255;
        let bar_color = Color32::from_rgb(r, g, b);

        painter.rect_filled(
            eframe::egui::Rect::from_min_size(eframe::egui::pos2(x, y), eframe::egui::vec2(bar_width, bar_h)),
            1.5,
            bar_color,
        );
    }
}

