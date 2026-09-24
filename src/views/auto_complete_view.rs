use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui::{Color32, RichText, ScrollArea};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::auto_complete::{AutoCompleteWorkflow, AutoProgressMsg};
use crate::models::HomeworkItem;
use crate::state::{AppState, NavTab};
use crate::views::render_header;

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
                AutoProgressMsg::QuestionDone { number, category, score, max_score, status, .. } => {
                    state.auto_questions_done.push((number, category, score, max_score, status));
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

            ui.row(|ui| {
                if ui.accent_button("🔑 前往登录 LA Studio 账号").clicked() {
                    state.current_tab = NavTab::CloudAccount;
                    state.la_active_tab = 0;
                }
                if ui.button("📚 返回作业与试卷列表 (基础解析)").clicked() {
                    state.current_tab = NavTab::HomeworkList;
                }
                if ui.button("📁 前往本地试卷扫描 (离线题库)").clicked() {
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

            ui.row(|ui| {
                if ui.accent_button("⚡ 立即通过 LA API 获取云端做题凭据").clicked() {
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

                if ui.button("⚙ 前往账号中心关联/绑定 ETS 账号").clicked() {
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
        ui.row(|ui| {
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

            if ui.button("🔄 刷新云端凭据").clicked() {
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
            if ui.accent_button("📚 前往作业列表").clicked() {
                state.current_tab = NavTab::HomeworkList;
            }
        });
        return;
    }

    // 默认选中第一个作业
    if state.auto_selected_hw_id.is_none()
        && let Some(first) = all_homeworks.first() {
            state.auto_selected_hw_id = Some(first.id.clone());
        }

    let active_hw = state
        .auto_selected_hw_id
        .as_ref()
        .and_then(|id| all_homeworks.iter().find(|h| &h.id == id));

    let selected_hw_name = active_hw
        .map(|h| h.name.clone())
        .unwrap_or_else(|| "未选择作业".to_string());

    let max_possible_score = active_hw.map(|h| h.total_point).unwrap_or(30.0);

    ui.card("流水线参数配置", "设置目标分数、语音合成音色与上报行为", |ui| {
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

            ui.row(|ui| {
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

        ui.row(|ui| {
            ui.label(format!("目标总分 (0.0 ~ {:.1} 分):", max_possible_score));
            let mut score_f32 = state.auto_config.target_total as f32;
            if ui.slider(&mut score_f32, 10.0..=max_possible_score as f32).changed() {
                state.auto_config.target_total = (score_f32 * 10.0).round() as f64 / 10.0;
            }
            ui.label(format!("{:.1} 分", state.auto_config.target_total));
        });

        ui.row(|ui| {
            ui.label("TTS 语音音色:");
            let voices = [
                ("en-US-JennyNeural", "Jenny (女声·标准美音)"),
                ("en-US-GuyNeural", "Guy (男声·标准美音)"),
                ("en-US-AriaNeural", "Aria (女声·自然抑扬)"),
                ("zh-CN-YunxiNeural", "云希 (中文/双语)"),
            ];

            for (v_id, v_label) in voices {
                let is_selected = state.auto_config.voice_name == v_id;
                if ui.button(if is_selected { format!("✔ {}", v_label) } else { v_label.to_string() }).clicked() {
                    state.auto_config.voice_name = v_id.to_string();
                }
            }
        });

        ui.row(|ui| {
            ui.label("模拟答题耗时:");
            let mut time_val = state.auto_config.use_time_seconds as f32;
            if ui.slider(&mut time_val, 60.0..=600.0).changed() {
                state.auto_config.use_time_seconds = time_val as u32;
            }
            ui.label(format!("{} 秒", state.auto_config.use_time_seconds));
        });

        ui.row(|ui| {
            ui.toggle(&mut state.auto_config.no_upload, "本地调试运行模式，不向官方服务器回传数据 (--no-upload)");
        });

        ui.separator();

        ui.row(|ui| {
            let can_start = state.auto_selected_hw_id.is_some()
                && !state.auto_running
                && state.la_session.is_some()
                && !state.session.token.trim().is_empty();
            let btn_clicked = if can_start {
                ui.accent_button("🚀 开始全自动做题流水线").clicked()
            } else {
                ui.button("🚀 开始全自动做题流水线").clicked()
            };

            if btn_clicked && can_start
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

            if state.auto_running {
                ui.label("⏳ 任务正在执行中，请勿关闭程序...");
            }
        });
    });

    ui.add_space(8.0);

    // 题目实时完成状态与得分明细表格
    if !state.auto_questions_done.is_empty() {
        ui.heading("题目评分与上传明细:");
        ui.card("实时题目评分看板", "展示逐题生成的打分与同步结果", |ui| {
            ScrollArea::vertical()
                .id_salt("auto_questions_table")
                .max_height(140.0)
                .show(ui, |ui_raw| {
                    let mut ui = FluentUi::from_ui(ui_raw);
                    for (num, cat, score, max, status) in &state.auto_questions_done {
                        ui.row(|ui| {
                            ui.label(RichText::new(format!("第 {} 题", num)).strong());
                            ui.label(format!("题型: {cat}"));
                            ui.label(
                                RichText::new(format!("得分: {:.2} / {:.2}", score, max))
                                    .color(Color32::from_rgb(76, 175, 80))
                                    .strong(),
                            );
                            ui.label(format!("状态: {status}"));
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
        .show(ui, |ui_raw| {
            let mut ui = FluentUi::from_ui(ui_raw);
            for hw in &all_homeworks {
                let is_selected = state.auto_selected_hw_id.as_ref() == Some(&hw.id);
                let btn_text = if is_selected {
                    format!("👉 [已选] {}", hw.name)
                } else {
                    format!("选择: {}", hw.name)
                };

                ui.row(|ui| {
                    if ui.button(btn_text).clicked() {
                        state.auto_selected_hw_id = Some(hw.id.clone());
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

fn render_soundwave_visualizer(ui: &mut FluentUi, time: f32) {
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

