use eframe::egui::{self, Color32, Margin, RichText, Stroke};
use crate::state::AppState;
use crate::views::{accent_action_btn, action_btn, danger_action_btn, responsive_flow};

/// WinUI 3 风格全局最终用户许可协议 (EULA) 与免责声明模态对话框
pub fn render_eula_modal(ctx: &egui::Context, state: &mut AppState) {
    if !state.eula_modal_open {
        return;
    }

    let is_dark = state.theme_config.theme.is_dark();
    let screen_w = ctx.content_rect().width();
    let modal_w = (screen_w - 48.0).clamp(420.0, 660.0);

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
        Color32::from_black_alpha(175)
    } else {
        Color32::from_black_alpha(115)
    };

    let mut action_dismiss = false;
    let mut action_accept = false;
    let mut action_reject_exit = false;

    let is_first_launch = state.eula_is_first_launch && !state.eula_accepted;

    let modal = egui::Modal::new(egui::Id::new("fepc_eula_modal"))
        .backdrop_color(backdrop_color)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(modal_w);

            // 1. 顶部 Header (图标 + 标题 + 副标题 + 可选关闭按钮)
            egui::Frame::new()
                .inner_margin(Margin { left: 20, right: 16, top: 18, bottom: 8 })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🛡️").size(24.0));
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("用户服务协议与免责声明")
                                    .size(17.5)
                                    .strong()
                                    .color(if is_dark { Color32::WHITE } else { Color32::BLACK })
                            );
                            ui.label(
                                RichText::new("商业专有软件许可 (EULA) · 隐私政策 · 服务与安全免责声明")
                                    .size(11.5)
                                    .color(Color32::from_gray(150))
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            // 若为常驻查阅模式，提供关闭按钮；首次强制拦截模式不提供关闭按钮（必须接受或退出）
                            if !is_first_launch {
                                let close_btn = ui.add(
                                    egui::Button::new(RichText::new("\u{00D7}").size(18.0).color(Color32::from_gray(160)))
                                        .frame(false)
                                );
                                if close_btn.clicked() {
                                    action_dismiss = true;
                                }
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

            // 2. 中间协议内容与滚动阅读区域
            egui::Frame::new()
                .inner_margin(Margin::symmetric(20, 12))
                .show(ui, |ui| {
                    // 重要合规警示横幅
                    let banner_bg = if is_dark {
                        Color32::from_rgba_unmultiplied(255, 70, 70, 22)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 50, 50, 16)
                    };
                    let banner_border = if is_dark {
                        Color32::from_rgb(130, 45, 45)
                    } else {
                        Color32::from_rgb(240, 180, 180)
                    };

                    egui::Frame::new()
                        .fill(banner_bg)
                        .corner_radius(6.0)
                        .stroke(Stroke::new(1.0, banner_border))
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.horizontal_top(|ui| {
                                ui.label(RichText::new("⚠️").size(18.0));
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("重要合规提示与严正免责声明")
                                            .size(13.0)
                                            .strong()
                                            .color(Color32::from_rgb(255, 90, 90))
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        RichText::new(
                                            "本软件为商业专有软件体系（历史开源协议已彻底废除）。\n\
                                            严禁用于考试作弊、违规代写或规避教学监管！因违规使用导致的成绩作废与处分风险由用户全权自负。"
                                        )
                                        .size(11.5)
                                        .color(if is_dark { Color32::from_gray(215) } else { Color32::from_gray(50) })
                                    );
                                });
                            });
                        });

                    ui.add_space(10.0);

                    // 协议条文垂直可滚动区域 (最大高度 320px)
                    egui::ScrollArea::vertical()
                        .id_salt("eula_modal_scroll")
                        .max_height(320.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let sections = crate::eula::get_eula_sections();
                            for section in sections {
                                let sec_bg = if is_dark {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 7)
                                } else {
                                    Color32::from_rgba_unmultiplied(0, 0, 0, 5)
                                };
                                let sec_border = if is_dark {
                                    Color32::from_rgb(52, 52, 58)
                                } else {
                                    Color32::from_rgb(230, 230, 235)
                                };

                                egui::Frame::new()
                                    .fill(sec_bg)
                                    .stroke(Stroke::new(1.0, sec_border))
                                    .corner_radius(6.0)
                                    .inner_margin(Margin::symmetric(12, 10))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(section.title)
                                                    .size(13.5)
                                                    .strong()
                                                    .color(if section.is_warning {
                                                        Color32::from_rgb(255, 120, 80)
                                                    } else {
                                                        Color32::from_rgb(0, 160, 255)
                                                    })
                                            );
                                            if let Some(badge) = section.badge {
                                                ui.label(
                                                    RichText::new(format!("[{badge}]"))
                                                        .size(11.0)
                                                        .color(Color32::from_gray(140))
                                                );
                                            }
                                        });

                                        ui.add_space(4.0);

                                        for (idx, (item_title, item_desc)) in section.items.iter().enumerate() {
                                            ui.horizontal_top(|ui| {
                                                ui.label(
                                                    RichText::new(format!("{}.", idx + 1))
                                                        .size(11.5)
                                                        .color(Color32::from_gray(140))
                                                );
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(*item_title)
                                                            .size(12.0)
                                                            .strong()
                                                            .color(if is_dark { Color32::from_gray(230) } else { Color32::from_gray(40) })
                                                    );
                                                    ui.label(
                                                        RichText::new(*item_desc)
                                                            .size(11.5)
                                                            .color(if is_dark { Color32::from_gray(175) } else { Color32::from_gray(85) })
                                                    );
                                                });
                                            });
                                            ui.add_space(4.0);
                                        }
                                    });

                                ui.add_space(8.0);
                            }
                        });

                    ui.add_space(10.0);

                    // 3. 底部同意勾选与操作按钮栏
                    if is_first_launch {
                        // 首次启动：复选框必须勾选
                        let checkbox_text = "我已完整阅读并自愿同意《专有软件商业许可 (EULA)》、《第三方凭据隐私授权》与《服务与安全免责声明》，自愿承担使用风险并承诺合规使用。";
                        let cb = egui::Checkbox::new(
                            &mut state.eula_checkbox_checked,
                            RichText::new(checkbox_text).size(12.0).color(if is_dark { Color32::WHITE } else { Color32::BLACK })
                        );
                        ui.add(cb);

                        ui.add_space(12.0);

                        // 按钮排版 (自适应折行)
                        responsive_flow(ui, |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // 同意按钮 (仅在勾选时激活)
                                if state.eula_checkbox_checked {
                                    if accent_action_btn(ui, "✔ 同意并进入软件", Some(Color32::from_rgb(0, 120, 215))).clicked() {
                                        action_accept = true;
                                    }
                                } else {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::new(RichText::new("请先阅读并勾选同意协议").size(12.5))
                                            .min_size(egui::vec2(0.0, 30.0))
                                            .corner_radius(6.0)
                                    );
                                }

                                ui.add_space(8.0);

                                if danger_action_btn(ui, "❌ 拒绝并退出").clicked() {
                                    action_reject_exit = true;
                                }
                            });
                        });
                    } else {
                        // 常驻查阅模式：显示签署状态及关闭按钮
                        responsive_flow(ui, |ui| {
                            ui.label(
                                RichText::new(format!("✔ 已签署协议 (生效版本: {})", crate::eula::CURRENT_EULA_VERSION))
                                    .size(12.0)
                                    .color(Color32::from_rgb(76, 175, 80))
                                    .strong()
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if action_btn(ui, "关闭").clicked() {
                                    action_dismiss = true;
                                }
                            });
                        });
                    }
                });
        });

    let escape = modal.is_top_modal
        && !modal.any_popup_open
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

    // 处理用户操作
    if action_accept {
        state.accept_eula();
    } else if action_reject_exit {
        // 用户拒绝：优雅退出应用程序
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        std::process::exit(0);
    } else if !is_first_launch && (modal.should_close() || escape || action_dismiss) {
        state.close_eula_modal();
    }
}
