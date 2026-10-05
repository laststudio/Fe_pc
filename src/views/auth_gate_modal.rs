use eframe::egui::{self, Color32, Margin, RichText, Stroke};
use crate::state::{AppState, NavTab};
use crate::views::{accent_action_btn, action_btn, responsive_flow};

/// WinUI 风格全局权限拦截与会员特权引导模态对话框
pub fn render_auth_gate_modal(ctx: &egui::Context, state: &mut AppState) {
    if !state.auth_gate_modal_open {
        return;
    }

    let is_dark = state.theme_config.theme.is_dark();
    let screen_w = ctx.content_rect().width();
    let modal_w = (screen_w - 48.0).clamp(320.0, 440.0);

    let card_bg = if is_dark {
        Color32::from_rgb(32, 32, 36)
    } else {
        Color32::from_rgb(255, 255, 255)
    };

    let border_color = if is_dark {
        Color32::from_rgb(60, 60, 68)
    } else {
        Color32::from_rgb(220, 220, 228)
    };

    let shadow_color = if is_dark {
        Color32::from_black_alpha(150)
    } else {
        Color32::from_black_alpha(60)
    };

    let frame = egui::Frame::new()
        .fill(card_bg)
        .stroke(Stroke::new(1.0, border_color))
        .corner_radius(10.0)
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 32,
            spread: 0,
            color: shadow_color,
        });

    let backdrop_color = if is_dark {
        Color32::from_black_alpha(160)
    } else {
        Color32::from_black_alpha(100)
    };

    let mut action_dismiss = false;
    let mut action_switch_login = false;
    let mut action_open_website = false;

    let requires_login = state.auth_gate_modal_requires_login;
    let feat_name = state.auth_gate_modal_feature_name.clone();
    let title = state.auth_gate_modal_title.clone();
    let message = state.auth_gate_modal_message.clone();
    let required_tier = state.auth_gate_modal_required_tier.clone();

    let modal = egui::Modal::new(egui::Id::new("fepc_auth_gate_modal"))
        .backdrop_color(backdrop_color)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(modal_w);

            // 1. 顶部 Header (图标 + 标题 + 关闭按钮)
            egui::Frame::new()
                .inner_margin(Margin { left: 20, right: 16, top: 18, bottom: 8 })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_text, icon_color, header_title) = if requires_login {
                            ("🔐", Color32::from_rgb(0, 150, 255), if title.is_empty() { "需要登录 LA 账号".to_string() } else { title.clone() })
                        } else {
                            ("👑", Color32::from_rgb(255, 200, 50), if title.is_empty() { "会员特权解锁".to_string() } else { title.clone() })
                        };

                        ui.label(RichText::new(icon_text).size(22.0).color(icon_color));
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(header_title)
                                    .size(17.0)
                                    .strong()
                                    .color(if requires_login {
                                        if is_dark { Color32::WHITE } else { Color32::BLACK }
                                    } else {
                                        Color32::from_rgb(255, 195, 45)
                                    })
                            );
                            let subtitle = if requires_login {
                                "登录 LA Studio 账号以激活并使用该功能"
                            } else {
                                "此功能属于尊享特权，升级会员即可立即解锁"
                            };
                            ui.label(RichText::new(subtitle).size(11.5).color(Color32::from_gray(150)));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            let close_btn = ui.add(
                                egui::Button::new(RichText::new("\u{00D7}").size(18.0).color(Color32::from_gray(160)))
                                    .frame(false)
                            );
                            if close_btn.clicked() {
                                action_dismiss = true;
                            }
                        });
                    });
                });

            ui.add_space(4.0);

            // 分割细线
            ui.painter().line_segment(
                [
                    ui.cursor().left_top(),
                    egui::pos2(ui.cursor().left() + modal_w, ui.cursor().top()),
                ],
                Stroke::new(1.0, border_color),
            );

            // 2. 中间内容卡片
            egui::Frame::new()
                .inner_margin(Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    // 功能标识标签
                    responsive_flow(ui, |ui| {
                        ui.label(RichText::new("受限功能:").size(12.5).color(Color32::from_gray(160)));
                        ui.label(
                            RichText::new(format!("【 {} 】", if feat_name.is_empty() { "高级功能" } else { &feat_name }))
                                .size(13.0)
                                .strong()
                                .color(if requires_login { Color32::from_rgb(0, 180, 255) } else { Color32::from_rgb(255, 180, 40) })
                        );
                    });

                    ui.add_space(8.0);

                    // 提示说明面板 (柔和底色高亮)
                    let callout_bg = if is_dark {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                    } else {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                    };

                    egui::Frame::new()
                        .fill(callout_bg)
                        .corner_radius(6.0)
                        .inner_margin(Margin::symmetric(14, 10))
                        .stroke(Stroke::new(1.0, if is_dark { Color32::from_rgb(50, 50, 56) } else { Color32::from_rgb(230, 230, 235) }))
                        .show(ui, |ui| {
                            let prompt_text = if !message.is_empty() {
                                message.clone()
                            } else if requires_login {
                                "该功能需要登录 LA 账号后方可使用。".to_string()
                            } else {
                                let tier_display = if required_tier.is_empty() { "Pro 会员" } else { &required_tier };
                                format!("此功能需要开通 {tier_display} 方可使用。")
                            };

                            ui.label(
                                RichText::new(prompt_text)
                                    .size(13.5)
                                    .strong()
                                    .color(if is_dark { Color32::WHITE } else { Color32::from_rgb(30, 30, 30) })
                            );

                            ui.add_space(4.0);
                            let tip = if requires_login {
                                "💡 提示：登录后即可在云端安全代登 ETS 账号、自动同步作业列表并漫游声音资产。"
                            } else {
                                "💡 提示：开通会员可立即解锁全自动答题流水线、高拟真 AI 声音克隆与极速多端漫游。"
                            };
                            ui.label(RichText::new(tip).size(12.0).color(Color32::from_gray(160)));
                        });

                    ui.add_space(16.0);

                    // 3. 底部操作按钮栏 (自适应折行，杜绝压缩截断)
                    responsive_flow(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if requires_login {
                                if accent_action_btn(ui, "🔐 立即登录 / 切换至账号页", Some(Color32::from_rgb(0, 120, 215))).clicked() {
                                    action_switch_login = true;
                                }
                                ui.add_space(4.0);
                                if action_btn(ui, "稍后再说").clicked() {
                                    action_dismiss = true;
                                }
                            } else {
                                if accent_action_btn(ui, "🚀 前往官网开通会员", Some(Color32::from_rgb(215, 140, 0))).clicked() {
                                    action_open_website = true;
                                }
                                ui.add_space(4.0);
                                if action_btn(ui, "知道了").clicked() {
                                    action_dismiss = true;
                                }
                            }
                        });
                    });
                });
        });

    let escape = modal.is_top_modal
        && !modal.any_popup_open
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

    if modal.should_close() || escape || action_dismiss {
        state.close_auth_gate_modal();
    } else if action_switch_login {
        state.close_auth_gate_modal();
        state.selected_homework = None;
        state.current_tab = NavTab::CloudAccount;
        state.la_active_tab = 0;
    } else if action_open_website {
        state.close_auth_gate_modal();
        let _ = open::that("https://lastudio.cc");
    }
}
