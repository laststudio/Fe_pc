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
        // 如果未登录且本地存在 ETS.db，自动尝试读取离线作业列表
        if let Some(db_path) = crate::ets_local_db::detect_db_path() {
            if let Ok(cached) = crate::ets_local_db::read_cached_homeworks(&db_path) {
                if !cached.is_empty() {
                    let count = cached.len();
                    state.homework_items = cached;
                    state.notification = Some((
                        format!("📂 已自动加载本地离线作业列表 ({} 项)", count),
                        InfoBarKind::Success,
                    ));
                    return;
                }
            }
        }
        state.notification = Some(("请先在【云端账号】页面登录 ETS 账号".to_string(), InfoBarKind::Warning));
        return;
    }

    if state.homework_refreshing {
        return;
    }

    state.homework_refreshing = true;

    if let Some(ref la_sess) = state.la_session {
        crate::la_client::sync_auth_to_resolver(&la_sess.access_token);
    }

    let tx = state.async_worker_tx.clone();
    let session = state.session.clone();
    let cat = state.selected_category;

    std::thread::spawn(move || {
        let result = match cat {
            HomeworkCategory::Current => {
                CloudClient::get_homework_list(&session, "1")
            }
            HomeworkCategory::History => {
                CloudClient::get_homework_list(&session, "2")
            }
            HomeworkCategory::VocabPk => {
                let mut all = Vec::new();
                let mut err = None;
                match CloudClient::get_homework_list(&session, "1") {
                    Ok(curr) => all.extend(curr),
                    Err(e) => err = Some(e),
                }
                match CloudClient::get_homework_list(&session, "2") {
                    Ok(hist) => all.extend(hist),
                    Err(e) => if err.is_none() { err = Some(e); },
                }
                if all.is_empty() && let Some(e) = err {
                    Err(e)
                } else {
                    let mut seen = std::collections::HashSet::new();
                    all.retain(|h| seen.insert(h.id.clone()));
                    all.retain(|h| h.is_pk_or_vocab());
                    Ok(all)
                }
            }
            HomeworkCategory::Local => Ok(Vec::new()),
        };

        let _ = tx.send(crate::state::SecureWorkerMessage::new(
            crate::state::AsyncWorkerEvent::HomeworkRefreshResult(result),
        ));
    });
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

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(RichText::new("🔍").size(13.0).color(Color32::from_gray(160)));
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_filter)
                        .hint_text("搜索试卷或 ID...")
                        .desired_width(160.0)
                        .vertical_align(egui::Align::Center)
                        .margin(egui::Margin::symmetric(6, 4))
                );
            });

            ui.separator();

            ui.label(RichText::new("状态:").color(Color32::from_gray(180)));

            for (key, label) in [("all", "全部"), ("pending", "未完成"), ("completed", "已完成")] {
                let is_sel = state.selected_status_filter == key;
                if choice_btn(ui, label, is_sel, state.theme_config.accent).clicked() {
                    state.selected_status_filter = key.to_string();
                }
            }

            ui.separator();

            let refresh_btn_text = if state.homework_refreshing {
                "[ ⏳ 同步中... ]"
            } else {
                "🔄 刷新列表"
            };
            if action_btn(ui, refresh_btn_text).clicked() && !state.homework_refreshing {
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
    let display_items: Vec<HomeworkItem> = state
        .homework_items
        .iter()
        .chain(state.local_items.iter())
        .filter(|hw| {
            let matches_category = match state.selected_category {
                HomeworkCategory::Current => {
                    if hw.source == HomeworkSource::Cloud {
                        !hw.is_pk_or_vocab()
                    } else {
                        // 本地作业或离线缓存作业在当前分类下可见
                        !hw.is_pk_or_vocab()
                    }
                }
                HomeworkCategory::History => {
                    if hw.source == HomeworkSource::Cloud {
                        !hw.is_pk_or_vocab()
                    } else {
                        hw.status.contains("已完成")
                    }
                }
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
        .cloned()
        .collect();

    let category_title = match state.selected_category {
        HomeworkCategory::Current => {
            if !state.session.is_logged_in && state.homework_items.is_empty() {
                "当前试卷列表 (离线/本地模式)"
            } else if state.homework_items.iter().any(|h| h.status.contains("离线缓存")) {
                "当前作业列表 (本地 ETS.db 离线缓存)"
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
        if state.homework_refreshing {
            ui.card("正在同步", "正在安全同步云端试卷与任务...", |ui| {
                ui.label(RichText::new("⏳ 正在安全同步云端试卷与任务，请稍候...").color(Color32::from_rgb(0, 180, 255)));
            });
            return;
        }

        if let Some(db_path) = crate::ets_local_db::detect_db_path() {
            ui.card("📂 本地离线作业库", "检测到系统包含 ETS.db 离线作业缓存", |ui| {
                ui.label("当前分类暂无作业。系统检测到本地已安装 e听说 客户端并存在离线作业缓存。");
                ui.add_space(6.0);
                responsive_flow(ui, |ui| {
                    if accent_action_btn(ui, "📂 载入本地离线作业缓存", None).clicked() {
                        if let Ok(cached) = crate::ets_local_db::read_cached_homeworks(&db_path) {
                            let count = cached.len();
                            state.homework_items = cached;
                            state.notification = Some((
                                format!("📂 已成功加载本地离线作业列表 ({} 项)", count),
                                InfoBarKind::Success,
                            ));
                        }
                    }
                });
            });
            return;
        }

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

    // 若当前展示的列表中包含来自 ETS.db 的离线缓存作业，展示顶部醒目提示
    let has_offline = display_items.iter().any(|h| h.status.contains("离线缓存"));
    if has_offline {
        ui.card("📂 已自动加载本地离线作业列表", "检测到未连接云端或云端列表为空，已为您无缝加载本地 ETS.db 离线作业", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("⚡ 本地离线模式:").strong().color(Color32::from_rgb(0, 180, 255)));
                ui.label(RichText::new(format!("共 {} 项离线作业已就绪，已与本地数据库无缝融合，支持极速秒出答案与离线做题", display_items.len())).color(Color32::from_gray(200)));
            });
        });
        ui.add_space(6.0);
    }

    // 自然卡片排版 (操作按钮使用 horizontal_wrapped 防止任何屏幕溢出)
    for hw in &display_items {
        let is_cloud = hw.source == HomeworkSource::Cloud;
        let is_pk = hw.is_pk_or_vocab();
        let is_offline_cache = hw.status.contains("离线缓存");

        let source_badge = if is_pk {
            "🔥 词汇PK"
        } else if is_offline_cache {
            "📂 离线缓存"
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
                    state.report_feature_usage("answer_parser");
                    let mut hw_to_select = hw.clone();
                    state.answer_view_mode = AnswerViewMode::Detailed;
                    // 如果是云端作业且未下载解析，触发下载 / 回退解析
                    if is_cloud && !hw_to_select.is_parsed() {
                        match CloudClient::download_and_parse_homework(&state.session, hw) {
                            Ok(parsed_hw) => {
                                hw_to_select = parsed_hw;
                            }
                            Err(err) => {
                                state.notification = Some((format!("解析试卷失败: {}", err), InfoBarKind::Error));
                            }
                        }
                    }
                    state.selected_homework = Some(hw_to_select);
                }

                if action_btn(ui, "⚡ 极简答案").clicked() {
                    state.report_feature_usage("answer_parser");
                    let mut hw_to_select = hw.clone();
                    state.answer_view_mode = AnswerViewMode::Concise;
                    if is_cloud && !hw_to_select.is_parsed()
                        && let Ok(parsed_hw) = CloudClient::download_and_parse_homework(&state.session, hw) {
                            hw_to_select = parsed_hw;
                        }
                    state.selected_homework = Some(hw_to_select);
                }

                if action_btn(ui, "🚀 自动做题").clicked() {
                    state.auto_selected_hw_id = Some(hw.id.clone());
                    state.current_tab = NavTab::AutoComplete;
                }

                if subtle_action_btn(ui, "📋 复制全部答案").clicked() {
                    state.report_feature_usage("copy_export");
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
