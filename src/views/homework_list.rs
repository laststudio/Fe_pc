use eframe::egui::RichText;
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::cloud_client::CloudClient;
use crate::models::{HomeworkItem, HomeworkSource};
use crate::state::{AnswerViewMode, AppState, NavTab};
use crate::utils::exporter::Exporter;
use crate::views::render_header;

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(ui, "作业与试卷", "查看云端与本地同步的听力口语试卷答案");

    // 状态统计与操作栏
    ui.card("作业控制台", "筛选与快速操作", |ui| {
        ui.row(|ui| {
            ui.text_box_hint(&mut state.search_filter, "🔍 搜索试卷名称或 ID...");

            for (key, label) in [("all", "全部"), ("pending", "未完成"), ("completed", "已完成")] {
                let is_sel = state.selected_status_filter == key;
                if ui.button(if is_sel { format!("● {}", label) } else { label.to_string() }).clicked() {
                    state.selected_status_filter = key.to_string();
                }
            }
            
            if ui.button("🔄 刷新云端列表").clicked() {
                if state.session.is_logged_in {
                    match CloudClient::get_homework_list(&state.session, "1") {
                        Ok(items) => {
                            state.homework_items = items;
                            state.notification = Some((
                                format!("成功获取到 {} 份云端作业", state.homework_items.len()),
                                InfoBarKind::Success,
                            ));
                        }
                        Err(e) => {
                            state.notification = Some((format!("获取作业失败: {}", e), InfoBarKind::Error));
                        }
                    }
                } else {
                    state.notification = Some(("请先在【云端账号】页面登录 ETS 账号".to_string(), InfoBarKind::Warning));
                }
            }

            if ui.button("📁 扫描本地试卷").clicked() {
                state.current_tab = NavTab::LocalScan;
            }
        });
    });

    ui.add_space(12.0);

    // 过滤题目
    let filter = state.search_filter.trim().to_lowercase();
    let display_items: Vec<HomeworkItem> = state
        .homework_items
        .iter()
        .cloned()
        .chain(state.local_items.iter().cloned())
        .filter(|hw| {
            let matches_text = if filter.is_empty() {
                true
            } else {
                hw.name.to_lowercase().contains(&filter) || hw.id.contains(&filter)
            };
            let matches_status = match state.selected_status_filter.as_str() {
                "pending" => !hw.status.contains("已完成"),
                "completed" => hw.status.contains("已完成"),
                _ => true,
            };
            matches_text && matches_status
        })
        .collect();

    ui.heading(RichText::new(format!("试卷列表 (共 {} 份)", display_items.len())).strong().size(18.0));
    ui.add_space(6.0);

    if display_items.is_empty() {
        ui.card("暂无试卷", "未找到任何作业", |ui| {
            ui.label("当前列表中暂无作业。您可以点击上方的【刷新云端列表】拉取最新作业，或进入【本地试卷扫描】直读电脑中下载的作业。");
            if !state.session.is_logged_in {
                if ui.accent_button("🔑 立即登录云端账号").clicked() {
                    state.current_tab = NavTab::CloudAccount;
                }
            }
        });
        return;
    }

    // 自然卡片排版，无内层 ScrollArea 限制
    for hw in display_items {
        let is_cloud = hw.source == HomeworkSource::Cloud;
        let source_badge = if is_cloud { "☁️ 云端" } else { "💻 本地" };
        let subtitle = format!(
            "{}  |  状态: {}  |  满分: {} 分  |  题目数: {} 题",
            source_badge, hw.status, hw.total_point, hw.question_count
        );

        ui.card(&hw.name, &subtitle, |ui| {
            ui.row(|ui| {
                if ui.accent_button("📖 查看答案").clicked() {
                    state.selected_homework = Some(hw.clone());
                    state.answer_view_mode = AnswerViewMode::Detailed;
                    // 如果是云端作业且未下载解析，触发下载
                    if is_cloud && !hw.is_parsed() {
                        match CloudClient::download_and_parse_homework(&state.session, &hw) {
                            Ok(parsed_hw) => {
                                state.selected_homework = Some(parsed_hw);
                            }
                            Err(err) => {
                                state.notification = Some((format!("解析云端试卷失败: {}", err), InfoBarKind::Error));
                            }
                        }
                    }
                }

                if ui.button("⚡ 极简答案").clicked() {
                    state.selected_homework = Some(hw.clone());
                    state.answer_view_mode = AnswerViewMode::Concise;
                    if is_cloud && !hw.is_parsed() {
                        if let Ok(parsed_hw) = CloudClient::download_and_parse_homework(&state.session, &hw) {
                            state.selected_homework = Some(parsed_hw);
                        }
                    }
                }

                if ui.button("🚀 自动做题").clicked() {
                    state.auto_selected_hw_id = Some(hw.id.clone());
                    state.current_tab = NavTab::AutoComplete;
                }

                if ui.subtle_button("📋 复制全部答案").clicked() {
                    let hw_to_copy = if is_cloud && !hw.is_parsed() {
                        CloudClient::download_and_parse_homework(&state.session, &hw).unwrap_or_else(|_| hw.clone())
                    } else {
                        hw.clone()
                    };
                    let text = Exporter::format_concise_answers(&hw_to_copy);
                    if Exporter::copy_text_to_clipboard(&text).is_ok() {
                        state.notification = Some(("已复制极简答案到剪贴板".to_string(), InfoBarKind::Success));
                    }
                }
            });
        });
        ui.add_space(8.0);
    }
}
