// =========================================================================================
// ⚠️ 作业列表排版规范：
// 1. 卡片底部的操作按钮组严禁使用 `ui.wrap` 配合 `ui.accent_button` / `ui.button`，
//    否则上游 FluentButton 会因 `raw_w.min(avail_w)` 导致行尾按钮被挤压为几十像素，文字截断！
// 2. 统一使用 `responsive_flow` + `accent_action_btn` / `action_btn` / `subtle_action_btn`。
// =========================================================================================

use eframe::egui::{self, Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::cloud_client::CloudClient;
use crate::models::{HomeworkCategory, HomeworkItem, HomeworkSource};
use crate::state::{AnswerViewMode, AppState, NavTab};
use crate::utils::exporter::Exporter;
use crate::views::{
    accent_action_btn, action_btn, choice_btn, render_header, responsive_flow, subtle_action_btn,
};

fn switch_category(state: &mut AppState, cat: HomeworkCategory) {
    state.selected_category = cat;
    if cat != HomeworkCategory::Local {
        refresh_current_category(state);
    }
}

fn refresh_current_category(state: &mut AppState) {
    if !state.session.is_logged_in {
        state.notification = Some(("请先在【云端账号】页面登录 ETS 账号".to_string(), InfoBarKind::Warning));
        return;
    }

    match state.selected_category {
        HomeworkCategory::Current => {
            match CloudClient::get_homework_list(&state.session, "1") {
                Ok(items) => {
                    state.homework_items = items;
                    state.notification = Some((
                        format!("成功获取到 {} 份当前日常作业", state.homework_items.len()),
                        InfoBarKind::Success,
                    ));
                }
                Err(e) => {
                    state.notification = Some((format!("获取当前作业失败: {}", e), InfoBarKind::Error));
                }
            }
        }
        HomeworkCategory::History => {
            match CloudClient::get_homework_list(&state.session, "2") {
                Ok(items) => {
                    state.homework_items = items;
                    state.notification = Some((
                        format!("成功获取到 {} 份历史归档作业", state.homework_items.len()),
                        InfoBarKind::Success,
                    ));
                }
                Err(e) => {
                    state.notification = Some((format!("获取历史作业失败: {}", e), InfoBarKind::Error));
                }
            }
        }
        HomeworkCategory::VocabPk => {
            let mut all = Vec::new();
            if let Ok(curr) = CloudClient::get_homework_list(&state.session, "1") {
                all.extend(curr);
            }
            if let Ok(hist) = CloudClient::get_homework_list(&state.session, "2") {
                all.extend(hist);
            }
            let mut seen = std::collections::HashSet::new();
            all.retain(|h| seen.insert(h.id.clone()));
            all.retain(|h| h.is_pk_or_vocab());
            let pk_count = all.len();
            state.homework_items = all;
            state.notification = Some((
                format!("成功检索并汇集 {} 份在线词汇PK竞技任务", pk_count),
                InfoBarKind::Success,
            ));
        }
        HomeworkCategory::Local => {}
    }
}

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(ui, "作业与试卷", "查看云端与本地同步的听力口语试卷答案，支持日常作业、历史记录与词汇PK对战");

    // 状态统计与分类筛选操作栏 (采用 ui.wrap 动态自适应流式排版：全屏宽屏展开为一行，窄屏自动折行无溢出)
    ui.card("作业控制台", "分类筛选与快速操作", |ui| {
        responsive_flow(ui, |ui| {
            ui.label(RichText::new("分类:").strong());
            egui::ComboBox::from_id_salt("homework_category_dropdown")
                .selected_text(match state.selected_category {
                    HomeworkCategory::Current => "📘 当前作业",
                    HomeworkCategory::History => "🕒 历史作业",
                    HomeworkCategory::VocabPk => "🔥 词汇PK [HOT]",
                    HomeworkCategory::Local => "💻 本地试卷",
                })
                .width(165.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(state.selected_category == HomeworkCategory::Current, "📘 当前作业 — 正在进行的日常任务").clicked() {
                        switch_category(state, HomeworkCategory::Current);
                    }
                    if ui.selectable_label(state.selected_category == HomeworkCategory::History, "🕒 历史作业 — 往期完成与逾期归档").clicked() {
                        switch_category(state, HomeworkCategory::History);
                    }
                    if ui.selectable_label(state.selected_category == HomeworkCategory::VocabPk, "🔥 词汇PK [HOT] — 在线词汇竞赛对战").clicked() {
                        switch_category(state, HomeworkCategory::VocabPk);
                    }
                    if ui.selectable_label(state.selected_category == HomeworkCategory::Local, "💻 本地试卷 — 本地扫描导入的试卷").clicked() {
                        switch_category(state, HomeworkCategory::Local);
                    }
                });

            ui.separator();

            ui.add(egui::TextEdit::singleline(&mut state.search_filter).hint_text("🔍 搜索试卷或 ID...").desired_width(180.0));

            ui.separator();

            ui.label(RichText::new("状态:").color(Color32::from_gray(180)));

            for (key, label) in [("all", "全部"), ("pending", "未完成"), ("completed", "已完成")] {
                let is_sel = state.selected_status_filter == key;
                if choice_btn(ui, label, is_sel, state.theme_config.accent).clicked() {
                    state.selected_status_filter = key.to_string();
                }
            }

            ui.separator();

            if action_btn(ui, "🔄 刷新列表").clicked() {
                refresh_current_category(state);
            }

            if action_btn(ui, "📁 扫描本地").clicked() {
                state.current_tab = NavTab::LocalScan;
            }
        });
    });

    ui.add_space(12.0);

    // 过滤题目
    let filter = state.search_filter.trim().to_lowercase();
    let display_items: Vec<&HomeworkItem> = state
        .homework_items
        .iter()
        .chain(state.local_items.iter())
        .filter(|hw| {
            let matches_category = match state.selected_category {
                HomeworkCategory::Current => {
                    if hw.source == HomeworkSource::Cloud {
                        !hw.is_pk_or_vocab()
                    } else {
                        // 未登录云端账号且云端列表为空时，本地作业在当前分类下可见
                        !state.session.is_logged_in && state.homework_items.is_empty()
                    }
                }
                HomeworkCategory::History => hw.source == HomeworkSource::Cloud && !hw.is_pk_or_vocab(),
                HomeworkCategory::VocabPk => hw.source == HomeworkSource::Cloud && hw.is_pk_or_vocab(),
                HomeworkCategory::Local => hw.source == HomeworkSource::Local,
            };
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
            matches_category && matches_text && matches_status
        })
        .collect();

    let category_title = match state.selected_category {
        HomeworkCategory::Current => {
            if !state.session.is_logged_in && state.homework_items.is_empty() {
                "当前试卷列表 (离线/本地模式)"
            } else {
                "当前日常作业列表"
            }
        }
        HomeworkCategory::History => "历史作业归档",
        HomeworkCategory::VocabPk => "词汇PK对战任务 (HOT)",
        HomeworkCategory::Local => "本地已扫描试卷",
    };

    responsive_flow(ui, |ui| {
        ui.heading(RichText::new(format!("{} (共 {} 份)", category_title, display_items.len())).strong().size(18.0));
        if state.selected_category == HomeworkCategory::VocabPk {
            ui.label(RichText::new("🔥 4级回退链路保护中").size(12.0).color(Color32::from_rgb(255, 120, 0)));
        }
    });
    ui.add_space(6.0);

    if display_items.is_empty() {
        ui.card("暂无匹配作业", "未找到符合当前筛选条件的试卷", |ui| {
            ui.label("当前分类或筛选条件下暂无作业。您可以点击上方的【🔄 刷新列表】拉取最新列表，或切换不同的分类标签。");
            if !state.session.is_logged_in {
                ui.add_space(4.0);
                responsive_flow(ui, |ui| {
                    if accent_action_btn(ui, "🔑 立即登录云端账号", None).clicked() {
                        state.current_tab = NavTab::CloudAccount;
                    }
                });
            }
        });
        return;
    }

    // 自然卡片排版 (操作按钮使用 horizontal_wrapped 防止任何屏幕溢出)
    for hw in display_items {
        let is_cloud = hw.source == HomeworkSource::Cloud;
        let is_pk = hw.is_pk_or_vocab();

        let source_badge = if is_pk {
            "🔥 词汇PK"
        } else if is_cloud {
            "☁️ 云端"
        } else {
            "💻 本地"
        };

        let subtitle = format!(
            "{}  |  状态: {}  |  满分: {} 分  |  题目数: {} 题",
            source_badge, hw.status, hw.total_point, hw.question_count
        );

        let card_header = if is_pk {
            format!("⚔️ {}", hw.name)
        } else {
            hw.name.clone()
        };

        ui.card(&card_header, &subtitle, |ui| {
            if is_pk {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("竞技类型: 词汇对战 / 在线专项练习").size(12.5).color(Color32::from_rgb(255, 160, 50)));
                    if !hw.resource_id.is_empty() {
                        ui.label(RichText::new(format!("资源ID: {}", hw.resource_id)).size(11.5).color(Color32::from_gray(160)));
                    }
                });
                ui.add_space(2.0);
            }

            responsive_flow(ui, |ui| {
                if accent_action_btn(ui, "📖 查看答案", None).clicked() {
                    state.selected_homework = Some(hw.clone());
                    state.answer_view_mode = AnswerViewMode::Detailed;
                    // 如果是云端作业且未下载解析，触发下载 / 回退解析
                    if is_cloud && !hw.is_parsed() {
                        match CloudClient::download_and_parse_homework(&state.session, hw) {
                            Ok(parsed_hw) => {
                                state.selected_homework = Some(parsed_hw);
                            }
                            Err(err) => {
                                state.notification = Some((format!("解析试卷失败: {}", err), InfoBarKind::Error));
                            }
                        }
                    }
                }

                if action_btn(ui, "⚡ 极简答案").clicked() {
                    state.selected_homework = Some(hw.clone());
                    state.answer_view_mode = AnswerViewMode::Concise;
                    if is_cloud && !hw.is_parsed()
                        && let Ok(parsed_hw) = CloudClient::download_and_parse_homework(&state.session, hw) {
                            state.selected_homework = Some(parsed_hw);
                        }
                }

                if action_btn(ui, "🚀 自动做题").clicked() {
                    state.auto_selected_hw_id = Some(hw.id.clone());
                    state.current_tab = NavTab::AutoComplete;
                }

                if subtle_action_btn(ui, "📋 复制全部答案").clicked() {
                    let hw_to_copy = if is_cloud && !hw.is_parsed() {
                        CloudClient::download_and_parse_homework(&state.session, hw).unwrap_or_else(|_| hw.clone())
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
