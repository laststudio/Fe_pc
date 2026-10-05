use eframe::egui::{self, Color32, Margin, RichText, ScrollArea, Stroke};
use crate::state::AppState;
use crate::views::{accent_action_btn, action_btn, responsive_flow};

/// WinUI 3 风格全局系统公告与版本专属通知模态对话框
pub fn render_announcement_modal(ctx: &egui::Context, state: &mut AppState) {
    if !state.announcement_modal_open {
        return;
    }

    let is_dark = state.theme_config.theme.is_dark();
    let screen_w = ctx.content_rect().width();
    let modal_w = (screen_w - 48.0).clamp(380.0, 540.0);

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
    let mut action_open_url: Option<String> = None;

    let policy_opt = state.software_policy.clone();
    let title = policy_opt
        .as_ref()
        .and_then(|p| p.announcement_title.clone())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "系统公告与版本通知".to_string());

    let message = policy_opt
        .as_ref()
        .and_then(|p| {
            p.announcement_message
                .clone()
                .or_else(|| p.notice_message.clone())
        })
        .unwrap_or_else(|| "暂无详细公告内容。".to_string());

    let changelog = policy_opt
        .as_ref()
        .and_then(|p| {
            p.changelog_summary
                .clone()
                .or_else(|| p.update_message.clone())
        });

    let url_opt = policy_opt
        .as_ref()
        .and_then(|p| p.announcement_url.clone())
        .filter(|u| !u.trim().is_empty());

    let modal = egui::Modal::new(egui::Id::new("fepc_announcement_modal"))
        .backdrop_color(backdrop_color)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(modal_w);

            // 1. 顶部 Header (图标 + 标题 + 关闭按钮)
            egui::Frame::new()
                .inner_margin(Margin { left: 20, right: 16, top: 18, bottom: 8 })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📢").size(22.0));
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&title)
                                    .size(16.5)
                                    .strong()
                                    .color(if is_dark { Color32::WHITE } else { Color32::BLACK })
                            );
                            ui.label(
                                RichText::new("LA Studio 官方公告与软件更新提醒")
                                    .size(11.5)
                                    .color(Color32::from_gray(150))
                            );
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

            // 2. 中间公告正文与更新日志
            egui::Frame::new()
                .inner_margin(Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .max_height(280.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(&message)
                                    .size(13.0)
                                    .color(if is_dark { Color32::from_gray(230) } else { Color32::from_gray(40) })
                            );

                            if let Some(ref log) = changelog {
                                if !log.trim().is_empty() {
                                    ui.add_space(10.0);
                                    let log_bg = if is_dark {
                                        Color32::from_rgba_unmultiplied(20, 30, 45, 200)
                                    } else {
                                        Color32::from_rgb(240, 245, 255)
                                    };
                                    let log_border = if is_dark {
                                        Color32::from_rgb(40, 70, 110)
                                    } else {
                                        Color32::from_rgb(200, 220, 250)
                                    };

                                    egui::Frame::new()
                                        .fill(log_bg)
                                        .corner_radius(6.0)
                                        .stroke(Stroke::new(1.0, log_border))
                                        .inner_margin(Margin::symmetric(12, 10))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new("📝 版本更新日志与变动摘要:")
                                                    .size(12.5)
                                                    .strong()
                                                    .color(Color32::from_rgb(0, 160, 255))
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                RichText::new(log)
                                                    .size(12.0)
                                                    .color(if is_dark { Color32::from_gray(210) } else { Color32::from_gray(60) })
                                            );
                                        });
                                }
                            }
                        });
                });

            // 3. 底部操作栏
            egui::Frame::new()
                .inner_margin(Margin { left: 20, right: 20, top: 4, bottom: 16 })
                .show(ui, |ui| {
                    responsive_flow(ui, |ui| {
                        if let Some(ref url) = url_opt {
                            if action_btn(ui, "🌐 查看详情 / 访问官网").clicked() {
                                action_open_url = Some(url.clone());
                            }
                        }

                        if accent_action_btn(ui, "✔ 好的，我知道了", state.theme_config.accent).clicked() {
                            action_dismiss = true;
                        }
                    });
                });
        });

    if modal.should_close() || action_dismiss {
        state.announcement_modal_open = false;
    }

    if let Some(url) = action_open_url {
        let _ = open::that(&url);
    }
}
