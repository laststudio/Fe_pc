// =========================================================================================
// ⚠️ 本地扫描视图排版规范：
// 1. 本地目录路径（dir_str）通常非常冗长，严禁直接塞入未截断的按钮中，必须使用 `truncate_middle(&dir_str, 42)`
//    并在 `.on_hover_text(&dir_str)` 中展示完整路径。
// 2. 所有操作按钮严禁使用 `ui.row` + `FluentButton`，统一使用 `responsive_flow` + `action_btn` / `accent_action_btn`。
// =========================================================================================

use eframe::egui::{Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::local::LocalScanner;
use crate::state::{AnswerViewMode, AppState, NavTab};
use crate::views::{
    accent_action_btn, action_btn, choice_btn, render_header, responsive_flow, truncate_middle,
};

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(ui, "本地试卷扫描", "直接读取 PC 端官方 e听说客户端下载在本地的作业与试卷数据");

    // 检测本地环境
    if state.local_ets_dirs.is_empty() {
        state.local_ets_dirs = LocalScanner::find_ets_local_dirs();
        if let Some(first) = state.local_ets_dirs.first() {
            state.selected_local_dir = Some(first.clone());
            state.local_items = LocalScanner::scan_homeworks(first);
        }
    }

    ui.card("本地客户端检测状态", "自动嗅探本地数据目录与已保存账号", |ui| {
        if state.local_ets_dirs.is_empty() {
            ui.label(RichText::new("⚠️ 未在 %APPDATA% 目录下检测到官方客户端数据目录 (74656D705F... 或 ETS)").color(Color32::from_rgb(255, 160, 0)));
        } else {
            ui.label(RichText::new(format!("✔ 已成功检测到 {} 个本地客户端数据目录", state.local_ets_dirs.len())).color(Color32::from_rgb(76, 175, 80)).strong());
            
            for dir in &state.local_ets_dirs {
                let is_selected = state.selected_local_dir.as_ref() == Some(dir);
                let dir_str = dir.to_string_lossy().to_string();
                let short_dir = truncate_middle(&dir_str, 42);
                let label = if is_selected {
                    format!("● 当前使用: {}", short_dir)
                } else {
                    format!("切换目录: {}", short_dir)
                };

                responsive_flow(ui, |ui| {
                    let btn_resp = choice_btn(ui, &label, is_selected, None);
                    if btn_resp.on_hover_text(&dir_str).clicked() {
                        state.selected_local_dir = Some(dir.clone());
                        state.local_items = LocalScanner::scan_homeworks(dir);
                        state.notification = Some((format!("已重新扫描本地目录: {}", dir_str), InfoBarKind::Success));
                    }
                });

                if is_selected {
                    // 读取硬件设备码与已保存账号
                    if let Some(dev_code) = LocalScanner::read_local_device_code(dir) {
                        ui.label(format!("  💻 本机硬件设备码: {}", dev_code));
                    }
                    let accounts = LocalScanner::read_local_accounts(dir);
                    if !accounts.is_empty() {
                        let acc_summaries: Vec<String> = accounts
                            .iter()
                            .map(|a| {
                                if a.name.is_empty() {
                                    a.account.clone()
                                } else {
                                    format!("{} ({})", a.name, a.account)
                                }
                            })
                            .collect();
                        ui.label(format!("  👤 已登录账号 ({} 个): {}", accounts.len(), acc_summaries.join("、 ")));
                    }
                }
            }
        }

        ui.separator();

        responsive_flow(ui, |ui| {
            if action_btn(ui, "🔄 重新扫描默认路径").clicked() {
                state.local_ets_dirs = LocalScanner::find_ets_local_dirs();
                if let Some(first) = state.local_ets_dirs.first() {
                    state.selected_local_dir = Some(first.clone());
                    state.local_items = LocalScanner::scan_homeworks(first);
                    state.notification = Some((format!("扫描完成，发现 {} 份本地试卷", state.local_items.len()), InfoBarKind::Success));
                }
            }

            if accent_action_btn(ui, "📂 选择其他自定义目录", None).clicked()
                && let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    state.selected_local_dir = Some(folder.clone());
                    state.local_items = LocalScanner::scan_homeworks(&folder);
                    state.notification = Some((format!("已扫描自定义目录，发现 {} 份作业", state.local_items.len()), InfoBarKind::Success));
                }
        });
    });

    ui.add_space(12.0);

    // 展示扫描出的本地试卷
    ui.heading(RichText::new(format!("本地试卷列表 (共 {} 份)", state.local_items.len())).strong().size(18.0));
    ui.add_space(6.0);

    if state.local_items.is_empty() {
        ui.card("未发现本地试卷", "当前选择的目录下未检索到包含 content.json 的题目文件夹", |ui| {
            ui.label("请先打开官方 e听说 PC 端下载作业，或使用上方【选择其他自定义目录】手动指定作业文件夹。");
        });
        return;
    }

    // 自然排版卡片列表，不添加受限的内部 ScrollArea，跟随主窗口统一平滑滚动
    for hw in &state.local_items {
        let hw_clone = hw.clone();
        let subtitle = format!(
            "本地 ID: {}  |  题目数: {} 题  |  题型组: {} 组",
            hw.id, hw.question_count, hw.sections.len()
        );

        ui.card(&hw.name, &subtitle, |ui| {
            responsive_flow(ui, |ui| {
                if accent_action_btn(ui, "📖 查看完整答案", None).clicked() {
                    state.selected_homework = Some(hw_clone.clone());
                    state.answer_view_mode = AnswerViewMode::Detailed;
                    state.current_tab = NavTab::HomeworkList;
                }

                if action_btn(ui, "⚡ 极简速查").clicked() {
                    state.selected_homework = Some(hw_clone.clone());
                    state.answer_view_mode = AnswerViewMode::Concise;
                    state.current_tab = NavTab::HomeworkList;
                }

                if action_btn(ui, "🚀 自动做题").clicked() {
                    state.auto_selected_hw_id = Some(hw_clone.id.clone());
                    state.current_tab = NavTab::AutoComplete;
                }
            });
        });
        ui.add_space(8.0);
    }
}
