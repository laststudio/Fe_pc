use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui::{Color32, RichText, ScrollArea};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::auto_complete::{AutoCompleteWorkflow, AutoProgressMsg};
use crate::models::HomeworkItem;
use crate::state::{AppState, NavTab};
use crate::views::render_header;

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(ui, "⚡ 自动做题流水线", "AI 语音合成、智能评分与全自动作业完成回传 (Scheme B)");

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

    // 整合所有可选作业
    let all_homeworks: Vec<HomeworkItem> = state
        .homework_items
        .iter()
        .cloned()
        .chain(state.local_items.iter().cloned())
        .collect();

    if all_homeworks.is_empty() {
        ui.card("未找到可做作业", "请先在【作业列表】刷新云端作业或在【本地扫描】中检测本地试卷", |ui| {
            if ui.accent_button("📚 前往作业列表").clicked() {
                state.current_tab = NavTab::HomeworkList;
            }
        });
        return;
    }

    // 作业选择与参数配置
    if state.auto_selected_hw_id.is_none() {
        if let Some(first) = all_homeworks.first() {
            state.auto_selected_hw_id = Some(first.id.clone());
        }
    }

    ui.card("流水线参数配置", "设置目标分数、语音合成音色与上报行为", |ui| {
        // 当前选中的作业
        let selected_hw_name = state
            .auto_selected_hw_id
            .as_ref()
            .and_then(|id| all_homeworks.iter().find(|h| &h.id == id))
            .map(|h| h.name.clone())
            .unwrap_or_else(|| "未选择作业 (请从下方列表中指定)".to_string());

        ui.label(RichText::new(format!("当前目标作业: {}", selected_hw_name)).strong().size(15.0).color(Color32::from_rgb(0, 180, 255)));

        // 进度展示
        if let Some((cur, tot, title)) = &state.auto_progress {
            ui.separator();
            let progress_val = (*cur as f32) / (*tot as f32).max(1.0);
            ui.label(RichText::new(format!("当前进度: 阶段 {}/{} - {}", cur, tot, title)).color(Color32::from_rgb(0, 220, 130)).strong());
            ui.progress(progress_val);
        }

        ui.separator();

        ui.row(|ui| {
            ui.label("目标分数 (0 ~ 30 分):");
            let mut score_f32 = state.auto_config.target_total as f32;
            if ui.slider(&mut score_f32, 10.0..=30.0).changed() {
                state.auto_config.target_total = (score_f32 * 10.0).round() as f64 / 10.0;
            }
            ui.label(format!("{} 分", state.auto_config.target_total));
        });

        ui.row(|ui| {
            ui.label("TTS 语音音色:");
            let voices = [
                ("en-US-JennyNeural", "Jenny (女声·标准美音)"),
                ("en-US-GuyNeural", "Guy (男声·标准美音)"),
                ("en-US-AriaNeural", "Aria (女声·自然抑扬)"),
                ("zh-CN-YunxiNeural", "云希 (中文/中英双语)"),
            ];

            for (v_id, v_label) in voices {
                let is_selected = state.auto_config.voice_name == v_id;
                if ui.button(if is_selected { format!("✔ {}", v_label) } else { v_label.to_string() }).clicked() {
                    state.auto_config.voice_name = v_id.to_string();
                }
            }
        });

        ui.row(|ui| {
            ui.toggle(&mut state.auto_config.no_upload, "仅本地测试运行，不向服务器回传答题数据 (--no-upload)");
        });

        ui.separator();

        ui.row(|ui| {
            let can_start = state.auto_selected_hw_id.is_some() && !state.auto_running;
            let btn_clicked = if can_start {
                ui.accent_button("🚀 开始自动完成作业").clicked()
            } else {
                ui.button("🚀 开始自动完成作业").clicked()
            };

            if btn_clicked && can_start {
                if let Some(hw) = all_homeworks.iter().find(|h| Some(&h.id) == state.auto_selected_hw_id.as_ref()) {
                    let hw_clone = hw.clone();
                    let session_clone = state.session.clone();
                    let config_clone = state.auto_config.clone();

                    state.auto_running = true;
                    state.auto_progress = Some((0, 8, "正在初始化流水线...".to_string()));
                    state.auto_logs.clear();
                    state.auto_logs.push("开始执行自动化工作流...".to_string());

                    // 创建后台通道
                    let (tx, rx): (Sender<AutoProgressMsg>, Receiver<AutoProgressMsg>) = channel();
                    state.auto_rx = Some(rx);

                    std::thread::spawn(move || {
                        AutoCompleteWorkflow::run(session_clone, hw_clone, config_clone, tx);
                    });

                    state.notification = Some(("自动化答题流水线已成功在后台启动！".to_string(), InfoBarKind::Success));
                }
            }

            if state.auto_running {
                ui.label("⏳ 任务正在运行中，请耐心等待...");
            }
        });
    });

    ui.add_space(8.0);

    // 快捷作业选择列表
    ui.heading("选择要完成的作业:");
    ScrollArea::vertical()
        .max_height(220.0)
        .show(&mut **ui, |ui_raw| {
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
    ui.card("运行日志终端", "实时显示 8 阶段完成进度与调试信息", |ui| {
        ScrollArea::vertical()
            .max_height(180.0)
            .stick_to_bottom(true)
            .show(&mut **ui, |ui_raw| {
                if state.auto_logs.is_empty() {
                    ui_raw.label(RichText::new("等待启动任务...").color(Color32::from_gray(140)));
                } else {
                    for line in &state.auto_logs {
                        let color = if line.contains("🎉") || line.contains("成功") || line.contains("完成") {
                            Color32::from_rgb(76, 175, 80)
                        } else if line.contains("✓") {
                            Color32::from_rgb(0, 200, 255)
                        } else if line.contains("阶段") || line.contains("[8/8]") || line.contains("开始") {
                            Color32::from_rgb(255, 200, 50)
                        } else if line.contains("失败") || line.contains("未完全成功") {
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
