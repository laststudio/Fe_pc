// =========================================================================================
// ⚠️ 试卷答案详情排版规范：
// 1. 试卷标题可能极长，严禁使用刚性不可折行的 `ui.row` 包裹标题，必须使用 `responsive_flow`。
// 2. 题目卡片底部的复制按钮统一使用 `responsive_flow` + `subtle_action_btn`。
// =========================================================================================

use eframe::egui::{self, Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::state::{AnswerViewMode, AppState, NavTab};
use crate::utils::exporter::Exporter;
use crate::views::{accent_action_btn, action_btn, choice_btn, responsive_flow, subtle_action_btn};

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    let hw = match &state.selected_homework {
        Some(h) => h.clone(),
        None => {
            ui.card("未选择试卷", "请返回试卷列表选择一份作业进行查看", |ui| {
                responsive_flow(ui, |ui| {
                    if accent_action_btn(ui, "⬅ 返回试卷列表", None).clicked() {
                        state.current_tab = NavTab::HomeworkList;
                    }
                });
            });
            return;
        }
    };

    // 异步上报试卷题目解析查看行为
    state.report_feature_usage("answer_parser");

    // 头部导航栏
    responsive_flow(ui, |ui| {
        if action_btn(ui, "⬅ 返回列表").clicked() {
            state.selected_homework = None;
            state.current_tab = NavTab::HomeworkList;
        }
        ui.heading(RichText::new(&hw.name).size(20.0).strong());
    });

    let subtitle = format!(
        "状态: {} | 满分: {} 分 | 总题数: {} 题",
        hw.status, hw.total_point, hw.total_questions()
    );
    ui.label(RichText::new(subtitle).color(Color32::from_gray(180)));

    // 操作控制面板
    ui.card("查看与导出控制", "切换答案显示形式或导出文件", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            let accent = state.theme_config.accent;

            // 视图切换
            let is_detailed = state.answer_view_mode == AnswerViewMode::Detailed;
            if choice_btn(ui, "📝 详细视图", is_detailed, accent).clicked() {
                state.answer_view_mode = AnswerViewMode::Detailed;
            }

            let is_concise = state.answer_view_mode == AnswerViewMode::Concise;
            if choice_btn(ui, "⚡ 极简速查", is_concise, accent).clicked() {
                state.answer_view_mode = AnswerViewMode::Concise;
            }

            let copy_check = state.check_feature("copy_export");
            let copy_label = if copy_check.allowed { "📋 复制全部答案" } else { "👑 会员复制全部答案" };
            if action_btn(ui, copy_label).clicked() {
                if !copy_check.allowed {
                    state.trigger_auth_gate_modal("copy_export", "答案复制与导出");
                } else {
                    state.report_feature_usage("copy_export");
                    let text = match state.answer_view_mode {
                        AnswerViewMode::Detailed => Exporter::format_full_answers(&hw),
                        AnswerViewMode::Concise => Exporter::format_concise_answers(&hw),
                    };
                    if Exporter::copy_text_to_clipboard(&text).is_ok() {
                        state.notification = Some(("已成功将答案复制到剪贴板！".to_string(), InfoBarKind::Success));
                    }
                }
            }

            let export_label = if copy_check.allowed { "💾 导出为 TXT" } else { "👑 会员导出为 TXT" };
            if action_btn(ui, export_label).clicked() {
                if !copy_check.allowed {
                    state.trigger_auth_gate_modal("copy_export", "答案复制与导出");
                } else if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("{}_答案.txt", hw.name))
                    .add_filter("文本文件", &["txt"])
                    .save_file()
                {
                    state.report_feature_usage("copy_export");
                    let text = Exporter::format_full_answers(&hw);
                    if Exporter::save_to_file(&path, &text).is_ok() {
                        state.notification = Some((format!("已成功导出至: {:?}", path), InfoBarKind::Success));
                    }
                }
            }

            let md_label = if copy_check.allowed { "📝 导出 Markdown" } else { "👑 会员导出 Markdown" };
            if action_btn(ui, md_label).clicked() {
                if !copy_check.allowed {
                    state.trigger_auth_gate_modal("copy_export", "答案复制与导出");
                } else if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("{}_答案.md", hw.name))
                    .add_filter("Markdown 文件", &["md"])
                    .save_file()
                {
                    state.report_feature_usage("copy_export");
                    let md = Exporter::format_markdown(&hw);
                    if Exporter::save_to_file(&path, &md).is_ok() {
                        state.notification = Some((format!("已成功导出至: {:?}", path), InfoBarKind::Success));
                    }
                }
            }

            if action_btn(ui, "🚀 以此卷发起自动做题").clicked() {
                state.auto_selected_hw_id = Some(hw.id.clone());
                state.current_tab = NavTab::AutoComplete;
            }
        });
    });

    ui.add_space(12.0);

    // 题目与答案内容自然排版
    match state.answer_view_mode {
        AnswerViewMode::Concise => {
            ui.card("极简答案速查清单", "标准答案精炼提取，便于核对或快速记录", |ui| {
                let concise_list = hw.get_concise_answers();
                for (idx_label, ans) in concise_list {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&idx_label).strong());
                        ui.label("➡");
                        ui.label(RichText::new(&ans).strong().color(Color32::from_rgb(0, 180, 255)));
                    });
                    ui.add_space(2.0);
                }
            });
        }
        AnswerViewMode::Detailed => {
            for section in &hw.sections {
                ui.heading(format!("📌 {}", section.title));
                ui.add_space(4.0);

                for q in &section.questions {
                    let card_title = format!("第 {} 题  [{}]", q.number, q.display_category());
                    ui.card(&card_title, "", |ui| {
                        if std::env::var("FEPC_SCROLL_TO_ROLE").is_ok() && (q.number == "6" || q.category.contains("role")) {
                            ui.scroll_to_cursor(Some(egui::Align::TOP));
                        }
                        if !q.prompt_text.is_empty() {
                            ui.label(RichText::new("【题目内容 / 原文】").strong());
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(&q.prompt_text).size(14.5));
                            });
                            ui.add_space(4.0);
                        }

                        if !q.options.is_empty() {
                            ui.label(RichText::new("【选择题选项】").strong());
                            for opt in &q.options {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(format!("  {}. {}", opt.label, opt.text)).size(14.0));
                                });
                            }
                            ui.add_space(4.0);
                        }

                        if q.answers.is_empty() {
                            ui.label(RichText::new("【标准参考答案】").strong().color(Color32::from_rgb(76, 175, 80)));
                            ui.label("暂无解析出标准答案");
                        } else if q.answers.len() == 1 {
                            ui.label(RichText::new("【标准参考答案】").strong().color(Color32::from_rgb(76, 175, 80)));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("✔").size(16.0).color(Color32::from_rgb(76, 175, 80)));
                                ui.label(RichText::new(&q.answers[0]).strong().size(15.0).color(Color32::from_rgb(0, 200, 255)));
                            });
                        } else {
                            // 口语/角色扮演等多答案题型：突出首选答案，备选等价表达折叠展示
                            ui.label(RichText::new("【推荐满分答案】").strong().color(Color32::from_rgb(76, 175, 80)));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("✔").size(16.0).color(Color32::from_rgb(76, 175, 80)));
                                ui.label(RichText::new(&q.answers[0]).strong().size(15.0).color(Color32::from_rgb(0, 200, 255)));
                            });

                            ui.add_space(3.0);
                            let alt_count = q.answers.len() - 1;
                            egui::CollapsingHeader::new(
                                RichText::new(format!("💬 查看其他 {} 种备选/等价表述 (点击展开)", alt_count))
                                    .color(Color32::from_gray(165))
                                    .size(13.0),
                            )
                            .id_salt(format!("alt_ans_{}_{}", q.index, q.number))
                            .default_open(false)
                            .show(ui, |ui| {
                                egui::Frame::new()
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 8))
                                    .corner_radius(4.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        for (i, alt) in q.answers.iter().skip(1).enumerate() {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(RichText::new(format!("{}.", i + 2)).color(Color32::from_gray(140)).size(13.0));
                                                ui.label(RichText::new(alt).color(Color32::from_gray(215)).size(13.5));
                                            });
                                        }
                                    });
                            });
                        }

                        if !q.important_points.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new("【核心得分关键词 / 词汇要点】").strong().color(Color32::from_rgb(255, 179, 0)));
                            ui.horizontal_wrapped(|ui| {
                                for kw in &q.important_points {
                                    ui.label(RichText::new(format!("🏷️ {}", kw)).strong().color(Color32::from_rgb(255, 200, 80)));
                                }
                            });
                        }

                        ui.add_space(6.0);
                        responsive_flow(ui, |ui| {
                            let copy_label = if q.answers.len() > 1 { "复制首选答案" } else { "复制本题答案" };
                            if subtle_action_btn(ui, copy_label).clicked()
                                && let Some(ans) = q.answers.first()
                                    && Exporter::copy_text_to_clipboard(ans).is_ok() {
                                        state.report_feature_usage("copy_export");
                                        state.notification = Some(("已复制首选答案到剪贴板！".to_string(), InfoBarKind::Success));
                                    }
                            if q.answers.len() > 1
                                && subtle_action_btn(ui, &format!("复制全部 {} 种表述", q.answers.len())).clicked() {
                                    let text = q.answers.join("\n");
                                    if Exporter::copy_text_to_clipboard(&text).is_ok() {
                                        state.report_feature_usage("copy_export");
                                        state.notification = Some(("已复制全部备选表述到剪贴板！".to_string(), InfoBarKind::Success));
                                    }
                                }
                        });
                    });
                    ui.add_space(6.0);
                }
            }
        }
    }
        }
