// =========================================================================================
// ⚠️ 云端账号中心排版规范：
// 1. 顶部模式切换、操作按钮、设备列表与弹窗表单统一使用 `responsive_flow`，
//    杜绝硬编码 `ui.row` 导致的右侧卡片边距丢失或窗口挤出。
// 2. 按钮统一使用 `choice_btn` / `action_btn` / `accent_action_btn` / `danger_action_btn`。
// =========================================================================================

use eframe::egui::{Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::cloud_client::CloudClient;
use crate::la_client::LaClient;
use crate::models::UserSession;
use crate::state::AppState;
use crate::views::{
    accent_action_btn, action_btn, choice_btn, danger_action_btn, render_header, responsive_flow,
};

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(
        ui,
        "账号与云端授权中心",
        "管理 LA Studio 会员账号、云端托管代登中继及 ETS100 官方直连会话",
    );

    // 模式切换分栏选择
    responsive_flow(ui, |ui| {
        let is_la_tab = state.la_active_tab == 0;
        let is_ets_tab = state.la_active_tab == 1;

        if choice_btn(ui, "✨ LA Studio 会员与云端托管 (推荐)", is_la_tab, state.theme_config.accent).clicked() {
            state.la_active_tab = 0;
        }

        if choice_btn(ui, "📱 ETS100 官方直连模式", is_ets_tab, state.theme_config.accent).clicked() {
            state.la_active_tab = 1;
        }
    });

    ui.add_space(8.0);

    if state.la_active_tab == 0 {
        render_la_studio_tab(ui, state);
    } else {
        render_ets_direct_tab(ui, state);
    }

    ui.add_space(8.0);

    // 风险提示
    ui.card("安全与风控说明", "云端安全协议须知", |ui| {
        ui.label(RichText::new("⚠ 注意事项:").strong().color(Color32::from_rgb(255, 179, 0)));
        ui.label("1. LA Studio 云端托管模式采用加密传输，支持无需验证码直接激活 ETS100 学生会话。");
        ui.label("2. 若手机官方客户端正在做题或处于模拟考试，请避免同时进行云端操作，以防触发单点登录互踢。");
        ui.label("3. Windows 平台设备码已在本机安全隔离并持久化，切勿随意更换硬件码导致账号触发异常风控。");
    });
}

fn render_la_studio_tab(ui: &mut FluentUi, state: &mut AppState) {
    if let Some(ref la_sess) = state.la_session.clone() {
        // 已登录 LA 账号
        ui.card("LA Studio 会员中心", "已连接至 LA Studio 云端认证服务", |ui| {
            responsive_flow(ui, |ui| {
                ui.label(RichText::new(format!("👤 用户名: {}", la_sess.user.username)).strong().size(16.0));
                let tier_text = la_sess.user.tier_display_name();
                let is_admin = la_sess.user.role <= 1;
                let tier_color = if is_admin {
                    Color32::from_rgb(255, 215, 0) // Gold
                } else {
                    match la_sess.user.tier.to_lowercase().as_str() {
                        "ultra" => Color32::from_rgb(255, 215, 0), // Gold
                        "pro" => Color32::from_rgb(0, 188, 212),    // Cyan
                        _ => Color32::from_rgb(158, 158, 158),
                    }
                };

                if is_admin {
                    let role_text = la_sess.user.role_display.as_deref().unwrap_or(if la_sess.user.role == 0 { "超级管理员" } else { "管理员" });
                    ui.label(RichText::new(format!(" [ 👑 {role_text} · ULTRA ] ")).strong().color(tier_color));
                } else {
                    ui.label(RichText::new(format!(" [ {tier_text} ] ")).strong().color(tier_color));
                }
                ui.label(format!("(ID: {})", la_sess.user.id));
            });

            if let Some(ref expire) = la_sess.user.member_expire_time {
                if !expire.trim().is_empty() {
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("⏳ 会员到期时间: {expire}")).size(12.5).color(Color32::from_rgb(100, 200, 255)));
                }
            } else if la_sess.user.role <= 1 {
                ui.add_space(2.0);
                ui.label(RichText::new("♾ 权限有效期: 永久特权 (系统管理员)").size(12.5).color(Color32::from_rgb(255, 215, 0)));
            }

            ui.add_space(4.0);
            ui.label(format!("🔑 会话编号: {}", la_sess.session_id));
            if let Some(ref dev_id) = la_sess.device_id {
                ui.label(format!("💻 绑定设备 ID: {dev_id}"));
            }

            ui.separator();

            responsive_flow(ui, |ui| {
                if danger_action_btn(ui, "🚪 退出登录").clicked() {
                    LaClient::logout(&la_sess.access_token);
                    state.la_session = None;
                    state.la_cloud_session = None;
                    state.la_devices.clear();
                    state.la_account_sessions.clear();
                    state.notification = Some(("已退出 LA Studio 账号".to_string(), InfoBarKind::Informational));
                }

                if action_btn(ui, "🔄 刷新设备列表").clicked() {
                    match LaClient::list_devices(&la_sess.access_token) {
                        Ok(devs) => {
                            state.la_devices = devs;
                            state.notification = Some(("设备列表已刷新".to_string(), InfoBarKind::Success));
                        }
                        Err(e) => {
                            state.notification = Some((format!("获取设备列表失败: {e}"), InfoBarKind::Error));
                        }
                    }
                }

                let pw_label = if state.change_password_dialog_open { "✖ 关闭修改密码" } else { "🔑 修改密码" };
                if action_btn(ui, pw_label).clicked() {
                    state.change_password_dialog_open = !state.change_password_dialog_open;
                }

                if action_btn(ui, "💻 刷新在线会话").clicked() {
                    match LaClient::list_sessions(&la_sess.access_token) {
                        Ok(sessions) => {
                            state.la_account_sessions = sessions;
                            state.notification = Some(("在线会话列表已刷新".to_string(), InfoBarKind::Success));
                        }
                        Err(e) => {
                            state.notification = Some((format!("获取会话列表失败: {e}"), InfoBarKind::Error));
                        }
                    }
                }
            });
        });

        // 修改密码卡片
        if state.change_password_dialog_open {
            ui.add_space(8.0);
            ui.card("修改账号密码", "更新您的 LA Studio 账号登录密码", |ui| {
                ui.label("原密码:");
                ui.password_box(&mut state.change_password_old_input);
                ui.add_space(4.0);
                ui.label("新密码:");
                ui.password_box(&mut state.change_password_new_input);
                ui.add_space(4.0);
                ui.label("确认新密码:");
                ui.password_box(&mut state.change_password_confirm_input);
                ui.add_space(6.0);

                responsive_flow(ui, |ui| {
                    if accent_action_btn(ui, "✔ 确认修改密码", None).clicked() {
                        let old = state.change_password_old_input.trim();
                        let new = state.change_password_new_input.trim();
                        let confirm = state.change_password_confirm_input.trim();

                        if old.is_empty() || new.is_empty() {
                            state.notification = Some(("原密码和新密码不能为空".to_string(), InfoBarKind::Warning));
                        } else if new != confirm {
                            state.notification = Some(("两次输入的新密码不一致".to_string(), InfoBarKind::Warning));
                        } else if new.len() < 6 {
                            state.notification = Some(("新密码长度至少需要 6 位".to_string(), InfoBarKind::Warning));
                        } else {
                            match LaClient::change_password(&la_sess.access_token, old, new) {
                                Ok(_) => {
                                    state.change_password_old_input.clear();
                                    state.change_password_new_input.clear();
                                    state.change_password_confirm_input.clear();
                                    state.change_password_dialog_open = false;
                                    state.notification = Some(("密码修改成功！".to_string(), InfoBarKind::Success));
                                }
                                Err(e) => {
                                    state.notification = Some((format!("修改密码失败: {e}"), InfoBarKind::Error));
                                }
                            }
                        }
                    }

                    if action_btn(ui, "取消").clicked() {
                        state.change_password_dialog_open = false;
                    }
                });
            });
        }

        // 在线会话列表卡片
        if !state.la_account_sessions.is_empty() {
            ui.add_space(8.0);
            ui.card("在线会话管理", "管理当前账号在各设备上的活跃会话，支持远程强制下线", |ui| {
                let sessions = state.la_account_sessions.clone();
                let count = sessions.len();
                for (idx, sess) in sessions.into_iter().enumerate() {
                    let is_current = sess.is_current || sess.id == la_sess.session_id;
                    let icon = match sess.client_key.as_str() {
                        "fepc-windows" => "💻",
                        "fe-android" => "📱",
                        "web" => "🌐",
                        _ => "🖥",
                    };
                    let client_name = match sess.client_key.as_str() {
                        "fepc-windows" => "Fe PC 桌面端",
                        "fe-android" => "Fe Android 客户端",
                        "web" => "Web 网页端",
                        k if !k.is_empty() => k,
                        _ => "未知终端",
                    };
                    let dev_name = if !sess.device_name.is_empty() {
                        sess.device_name.as_str()
                    } else {
                        client_name
                    };

                    let avail_w = ui.available_width();
                    if avail_w < 480.0 {
                        responsive_flow(ui, |ui| {
                            let cur_badge = if is_current { " [当前会话]" } else { "" };
                            ui.label(RichText::new(format!("{icon} {dev_name}{cur_badge}")).strong().size(13.5));
                        });
                        ui.add_space(2.0);
                        responsive_flow(ui, |ui| {
                            if !is_current {
                                let sess_id = sess.id.clone();
                                if danger_action_btn(ui, "下线会话").clicked() {
                                    match LaClient::revoke_session(&la_sess.access_token, &sess_id) {
                                        Ok(_) => {
                                            state.la_account_sessions.retain(|s| s.id != sess_id);
                                            state.notification = Some(("已成功下线该会话".to_string(), InfoBarKind::Success));
                                        }
                                        Err(e) => {
                                            state.notification = Some((format!("下线会话失败: {e}"), InfoBarKind::Error));
                                        }
                                    }
                                }
                            } else {
                                ui.label(
                                    RichText::new("🟢 当前活跃")
                                        .size(11.5)
                                        .color(Color32::from_rgb(76, 175, 80))
                                        .strong(),
                                );
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let cur_badge = if is_current { " [当前会话]" } else { "" };
                            ui.label(RichText::new(format!("{icon} {dev_name}{cur_badge}")).strong().size(13.5));

                            ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                                if !is_current {
                                    let sess_id = sess.id.clone();
                                    if danger_action_btn(ui, "下线会话").clicked() {
                                        match LaClient::revoke_session(&la_sess.access_token, &sess_id) {
                                            Ok(_) => {
                                                state.la_account_sessions.retain(|s| s.id != sess_id);
                                                state.notification = Some(("已成功下线该会话".to_string(), InfoBarKind::Success));
                                            }
                                            Err(e) => {
                                                state.notification = Some((format!("下线会话失败: {e}"), InfoBarKind::Error));
                                            }
                                        }
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("🟢 当前活跃")
                                            .size(11.5)
                                            .color(Color32::from_rgb(76, 175, 80))
                                            .strong(),
                                    );
                                }
                            });
                        });
                    }

                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        ui.label(RichText::new(format!("会话: {}", sess.id)).size(11.5).color(Color32::from_gray(160)));
                        ui.label(RichText::new(format!("客户端: {client_name}")).size(11.5).color(Color32::from_gray(160)));
                        if let Some(ref ip) = sess.ip {
                            ui.label(RichText::new(format!("IP: {ip}")).size(11.5).color(Color32::from_gray(140)));
                        }
                        if let Some(ref active) = sess.last_active_at {
                            ui.label(RichText::new(format!("活跃: {active}")).size(11.5).color(Color32::from_gray(140)));
                        }
                    });

                    if idx + 1 < count {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }
                }
            });
        }

        ui.add_space(8.0);

        // 学生账号绑定与托管引导卡片 (客户端不提供绑定与解绑，统一至网页端进行)
        ui.card("学生账号绑定与托管", "绑定、解绑与更换学生账号统一在 LA Studio 网页端安全进行", |ui| {
            ui.label("为保障学生账号与会话凭据资产安全，学生 e听说 / 畅言 账号的绑定、解绑与更换统一在 LA Studio 网页控制台安全进行。");
            ui.label("客户端无需在本地输入学生密码，直接通过云端免密中继通道安全代登并同步作业。");
            ui.add_space(6.0);
            responsive_flow(ui, |ui| {
                if accent_action_btn(ui, "🌐 前往 LA Studio 网页端绑定/管理学生账号", None).clicked() {
                    let _ = open::that("https://lastudio.cc/account");
                    state.notification = Some(("已在浏览器中打开 LA Studio 账号控制台".to_string(), InfoBarKind::Informational));
                }
            });
        });

        ui.add_space(8.0);

        // 云端免密中继代理卡片
        ui.card("云端免密中继代理", "由 LA Studio 云端代登获取学生会话，无需本地短信验证码", |ui| {
            if let Some(ref cloud) = state.la_cloud_session {
                ui.label(RichText::new(format!("🟢 当前已激活代理学生账号: {}", cloud.account_display)).strong().color(Color32::from_rgb(76, 175, 80)));
                ui.label(format!("关联账号 ID: {}", cloud.parent_account_id));
                ui.label(format!("设备识别码: {}", cloud.device_code));
                ui.add_space(4.0);
            } else {
                ui.label(RichText::new("⚪ 尚未激活云端做题会话").color(Color32::from_rgb(180, 180, 180)));
                ui.label("若已在网页端完成学生账号绑定，请点击下方按钮一键拉取并激活；若尚未绑定，请先前往网页端绑定。");
                ui.add_space(4.0);
            }

            responsive_flow(ui, |ui| {
                if accent_action_btn(ui, "🚀 一键激活 / 刷新云端做题会话", None).clicked() {
                    let tx = state.async_worker_tx.clone();
                    let token = la_sess.access_token.clone();
                    let fallback_dev = state.session.device_code.clone();
                    state.notification = Some(("正在同步云端做题会话与作业...".to_string(), InfoBarKind::Informational));
                    std::thread::spawn(move || {
                        crate::la_client::sync_auth_to_resolver(&token);
                        match LaClient::get_cloud_session(&token, "") {
                            Ok(cloud) => {
                                let dev_code = if !cloud.device_code.is_empty() {
                                    cloud.device_code.clone()
                                } else {
                                    LaClient::get_cloud_device_code(&token).unwrap_or(fallback_dev)
                                };
                                let user_name = if let Some(name) = cloud.selected_account.get("name").and_then(|v| v.as_str()) {
                                    name.to_string()
                                } else {
                                    cloud.account_display.clone()
                                };
                                let user_sess = UserSession {
                                    phone: cloud.account_display.clone(),
                                    token: cloud.token.clone(),
                                    device_code: dev_code,
                                    parent_account_id: cloud.parent_account_id.clone(),
                                    user_name,
                                    is_logged_in: true,
                                };
                                let hws = CloudClient::get_homework_list(&user_sess, "1").ok();
                                let devs = LaClient::list_devices(&token).ok();
                                let _ = tx.send(crate::state::SecureWorkerMessage::new(
                                    crate::state::AsyncWorkerEvent::CloudSessionSyncResult(
                                        Some(user_sess),
                                        Some(cloud),
                                        hws,
                                        devs,
                                    ),
                                ));
                            }
                            Err(e) => {
                                eprintln!("get_cloud_session err: {e}");
                            }
                        }
                    });
                }
            });
        });

        // 设备管理卡片 (查看已授权设备，不设本地解绑按钮)
        if !state.la_devices.is_empty() {
            ui.add_space(8.0);
            ui.card("已绑定的授权设备", "查看您在 LA Studio 登录过的终端设备 (设备绑定与解绑管理统一在网页控制台进行)", |ui| {
                let devices = state.la_devices.clone();
                let count = devices.len();
                for (idx, dev) in devices.into_iter().enumerate() {
                    let (friendly_name, icon) = format_friendly_device_name(&dev);
                    let is_current = dev.is_current;

                    // 1. 标题与状态行：宽屏两端对齐，窄屏自适应折行
                    let avail_w = ui.available_width();
                    if avail_w < 480.0 {
                        responsive_flow(ui, |ui| {
                            let cur_badge = if is_current { " [当前设备]" } else { "" };
                            let resp = ui.label(
                                RichText::new(format!("{icon} {friendly_name}{cur_badge}"))
                                    .strong()
                                    .size(13.5),
                            );
                            if friendly_name != dev.device_name {
                                resp.on_hover_text(format!("原始设备完整标识:\n{}", dev.device_name));
                            }
                        });
                        ui.add_space(2.0);
                        responsive_flow(ui, |ui| {
                            if is_current {
                                ui.label(
                                    RichText::new("🟢 本机终端")
                                        .size(11.5)
                                        .color(Color32::from_rgb(76, 175, 80))
                                        .strong(),
                                );
                            } else {
                                ui.label(
                                    RichText::new("🖥 授权设备")
                                        .size(11.5)
                                        .color(Color32::from_gray(160)),
                                );
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let cur_badge = if is_current { " [当前设备]" } else { "" };
                            let resp = ui.label(
                                RichText::new(format!("{icon} {friendly_name}{cur_badge}"))
                                    .strong()
                                    .size(13.5),
                            );
                            if friendly_name != dev.device_name {
                                resp.on_hover_text(format!("原始设备完整标识:\n{}", dev.device_name));
                            }

                            ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                                if is_current {
                                    ui.label(
                                        RichText::new("🟢 本机终端")
                                            .size(11.5)
                                            .color(Color32::from_rgb(76, 175, 80))
                                            .strong(),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new("🖥 授权设备")
                                            .size(11.5)
                                            .color(Color32::from_gray(160)),
                                    );
                                }
                            });
                        });
                    }

                    ui.add_space(2.0);

                    let is_windows = dev.is_current
                        || dev.platform_version.to_lowercase().contains("windows")
                        || dev.device_name.starts_with("PC-")
                        || dev.device_name.to_uppercase().contains("DESKTOP")
                        || dev.device_name.to_uppercase().contains("LAPTOP")
                        || dev.device_name.to_uppercase().contains("WORKSTATION");

                    // 2. 设备元数据信息行：自动换行（horizontal_wrapped），绝不横向溢出
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        if !dev.platform_version.is_empty() {
                            let plat_display = if is_windows && dev.platform_version.contains("Android") {
                                "Windows 11 / 10"
                            } else {
                                dev.platform_version.as_str()
                            };
                            ui.label(
                                RichText::new(format!("系统: {plat_display}"))
                                    .size(11.5)
                                    .color(Color32::from_gray(160)),
                            );
                        }
                        if !dev.client_key.is_empty() {
                            let client_display = match dev.client_key.as_str() {
                                "fepc-windows" => "Fe PC 桌面端",
                                "fe-android" => "Fe Android",
                                "web" => "Web 网页端",
                                k => k,
                            };
                            ui.label(
                                RichText::new(format!("客户端: {client_display}"))
                                    .size(11.5)
                                    .color(Color32::from_gray(160)),
                            );
                        }
                        if let Some(ref active_time) = dev.last_active_at
                            && !active_time.trim().is_empty()
                        {
                            ui.label(
                                RichText::new(format!("活跃时间: {active_time}"))
                                    .size(11.5)
                                    .color(Color32::from_gray(140)),
                            );
                        }
                        if friendly_name != dev.device_name {
                            ui.label(
                                RichText::new("ℹ 鼠标悬浮名称查看完整UA")
                                    .size(10.5)
                                    .color(Color32::from_rgb(0, 150, 220)),
                            )
                            .on_hover_text(format!("原始设备完整标识:\n{}", dev.device_name));
                        }
                    });

                    if idx + 1 < count {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }
                }
            });
        }
    } else {
        // 未登录 LA 账号，显示登录卡片
        ui.card("登录 LA Studio 账号", "登录后可享免输验证码做题、会员特权及云端会话中继", |ui| {
            ui.text_box_hint(&mut state.la_username_input, "请输入 LA Studio 用户名");
            ui.add_space(4.0);
            ui.password_box(&mut state.la_password_input);
            ui.add_space(8.0);

            responsive_flow(ui, |ui| {
                let btn_text = if state.la_login_in_progress {
                    let t = (ui.ctx().input(|i| i.time) * 4.0) as usize % 4;
                    let spinner = ["◐", "◓", "◑", "◒"][t];
                    let stage = if state.la_login_stage_text.is_empty() {
                        "正在安全鉴权与同步会话..."
                    } else {
                        &state.la_login_stage_text
                    };
                    format!("[ {} {} ]", spinner, stage)
                } else {
                    "🔑 登录 LA 账号".to_string()
                };

                let btn_resp = accent_action_btn(ui, &btn_text, None);
                if !state.la_login_in_progress && btn_resp.clicked() {
                    let u = state.la_username_input.trim().to_string();
                    let p = state.la_password_input.clone();

                    if u.is_empty() || p.is_empty() {
                        state.notification = Some(("用户名和密码不能为空".to_string(), InfoBarKind::Warning));
                        return;
                    }

                    let cap_token_opt = if state.la_captcha_token.trim().is_empty() {
                        None
                    } else {
                        Some(state.la_captcha_token.trim().to_string())
                    };

                    start_async_la_login(state, u, p, cap_token_opt);
                }
            });
        });

        if state.la_captcha_needed {
            ui.add_space(8.0);
            ui.card("🛡️ 人机安全验证", "服务端要求完成人机安全验证", |ui| {
                ui.label(RichText::new("服务端要求进行安全验证。完成验证后，桌面端将自动恢复并登录：").color(Color32::from_rgb(255, 180, 0)));
                ui.add_space(4.0);
                responsive_flow(ui, |ui| {
                    if accent_action_btn(ui, "🌐 在默认浏览器中打开验证页面", None).clicked() {
                        if state.captcha_server.is_none() {
                            if let Ok(srv) = crate::utils::captcha_server::CaptchaServer::start() {
                                let port = srv.port();
                                state.captcha_server = Some(srv);
                                let _ = open::that(format!("http://127.0.0.1:{port}/"));
                            }
                        } else if let Some(srv) = &state.captcha_server {
                            let _ = open::that(format!("http://127.0.0.1:{}/", srv.port()));
                        }
                    }
                    if state.captcha_server.is_some() {
                        ui.label(RichText::new("● 正在监听浏览器验证结果...").color(Color32::from_rgb(0, 200, 255)));
                    }
                });
                ui.add_space(6.0);
                ui.separator();
                ui.label("或手动粘贴已获取的验证 Token 登录：");
                responsive_flow(ui, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.la_captcha_token).hint_text("粘贴 cap_token").desired_width(180.0));
                    let token_btn_text = if state.la_login_in_progress {
                        "[ ⏳ 正在安全鉴权... ]"
                    } else {
                        "带 Token 登录"
                    };
                    let token_btn_resp = action_btn(ui, token_btn_text);
                    if !state.la_login_in_progress && token_btn_resp.clicked() {
                        let u = state.la_username_input.trim().to_string();
                        let p = state.la_password_input.clone();
                        let token = state.la_captcha_token.trim().to_string();
                        if !token.is_empty() {
                            start_async_la_login(state, u, p, Some(token));
                        }
                    }
                });
            });
        }
    }
}

/// 启动 LA 账号全流程安全异步登录与会话同步流水线 (转移 5 步网络请求至后台 Worker 线程)
pub fn start_async_la_login(
    state: &mut AppState,
    u: String,
    p: String,
    cap_token_opt: Option<String>,
) {
    if state.la_login_in_progress {
        return;
    }
    state.la_login_in_progress = true;
    state.la_login_stage_text = "正在安全鉴权与同步会话...".to_string();

    let tx = state.async_worker_tx.clone();
    let fallback_dev_code = state.session.device_code.clone();

    std::thread::spawn(move || {
        let cap_token_ref = cap_token_opt.as_deref().filter(|s| !s.trim().is_empty());
        // 第 1 步：LaClient::login(u, p, cap_token_opt)
        match LaClient::login(&u, &p, cap_token_ref) {
            Ok(sess) => {
                crate::la_client::sync_auth_to_resolver(&sess.access_token);
                let _ = tx.send(crate::state::SecureWorkerMessage::new(
                    crate::state::AsyncWorkerEvent::LaLoginResult(Ok(sess.clone())),
                ));

                // 第 2 步：LaClient::get_cloud_device_code(&sess.access_token)
                let cloud_dev_code = LaClient::get_cloud_device_code(&sess.access_token).ok();

                // 第 3 步：LaClient::get_cloud_session(&sess.access_token, "")
                let cloud_res = LaClient::get_cloud_session(&sess.access_token, "");
                let (user_sess_opt, cloud_sess_opt, hws_opt) = match cloud_res {
                    Ok(cloud) => {
                        let dev_code = if !cloud.device_code.is_empty() {
                            cloud.device_code.clone()
                        } else {
                            cloud_dev_code.unwrap_or_else(|| fallback_dev_code.clone())
                        };
                        let user_name = if let Some(name) = cloud.selected_account.get("name").and_then(|v| v.as_str()) {
                            name.to_string()
                        } else {
                            cloud.account_display.clone()
                        };
                        let user_sess = UserSession {
                            phone: cloud.account_display.clone(),
                            token: cloud.token.clone(),
                            device_code: dev_code,
                            parent_account_id: cloud.parent_account_id.clone(),
                            user_name,
                            is_logged_in: true,
                        };

                        // 第 4 步：CloudClient::get_homework_list(&state.session, "1")
                        let hws = CloudClient::get_homework_list(&user_sess, "1").ok();
                        (Some(user_sess), Some(cloud), hws)
                    }
                    Err(_) => (None, None, None),
                };

                // 第 5 步：LaClient::list_devices(&sess.access_token)
                let devs = LaClient::list_devices(&sess.access_token).ok();

                let _ = tx.send(crate::state::SecureWorkerMessage::new(
                    crate::state::AsyncWorkerEvent::CloudSessionSyncResult(
                        user_sess_opt,
                        cloud_sess_opt,
                        hws_opt,
                        devs,
                    ),
                ));
            }
            Err(e) => {
                let _ = tx.send(crate::state::SecureWorkerMessage::new(
                    crate::state::AsyncWorkerEvent::LaLoginResult(Err(e)),
                ));
            }
        }
    });
}

fn render_ets_direct_tab(ui: &mut FluentUi, state: &mut AppState) {
    ui.card("模式说明：官方直连查题模式", "直接连接官方 ETS100 服务，无需注册或登录 LA 账号", |ui| {
        ui.label(RichText::new("ℹ 本模式完全免费开放：").strong().color(Color32::from_rgb(0, 180, 255)));
        ui.label("• 可直接使用您本人的 ETS100 手机号与密码登录，安全直连官方服务器；");
        ui.label("• 支持完整查看老师布置的作业列表、下载并解析试卷、查看题目与答案速查；");
        ui.label("• 如需使用 AI 语音合成与全自动做题流水线，请在上方分栏切换至【✨ LA Studio 会员与云端托管】模式。");
    });
    ui.add_space(8.0);

    if state.session.is_logged_in {
        ui.card("ETS100 官方直连会话", "已建立直连会话", |ui| {
            ui.label(RichText::new(format!("👤 用户姓名: {}", if state.session.user_name.is_empty() { "学生" } else { &state.session.user_name })).strong().size(16.0));
            ui.label(format!("📱 登录手机号: {}", state.session.phone));
            ui.label(format!("💻 绑定设备码: {}", state.session.device_code));
            ui.label(format!("🆔 家长/学生关联 ID: {}", state.session.parent_account_id));
            ui.label(RichText::new("✔ 会话令牌有效").color(Color32::from_rgb(76, 175, 80)));

            ui.separator();

            responsive_flow(ui, |ui| {
                if danger_action_btn(ui, "🚪 退出当前登录").clicked() {
                    CloudClient::clear_session();
                    state.session = Default::default();
                    state.login_password.clear();
                    state.notification = Some(("已退出当前账号登录".to_string(), InfoBarKind::Informational));
                }
            });
        });
        return;
    }

    // 未登录时显示官方直连登录表单
    ui.card("ETS100 官方账号密码直连", "直接向官方接口发起鉴权登录", |ui| {
        ui.text_box_hint(&mut state.session.phone, "请输入 ETS100 绑定的手机号");
        ui.add_space(4.0);

        ui.password_box(&mut state.login_password);
        ui.add_space(4.0);

        if state.session.device_code.is_empty() {
            if let Some(code) = state.la_session.as_ref().and_then(|s| LaClient::get_cloud_device_code(&s.access_token).ok()) {
                state.session.device_code = code;
            }
        }
        if state.session.device_code.is_empty() {
            state.session.device_code = CloudClient::get_or_generate_device_code(None);
        }
        ui.text_box_hint(&mut state.session.device_code, "设备硬件码 (优先使用云端下发硬件标识)");

        ui.add_space(8.0);

        responsive_flow(ui, |ui| {
            if accent_action_btn(ui, "🔑 立即登录", None).clicked() {
                let phone = state.session.phone.trim();
                let dev_code = state.session.device_code.trim();

                if phone.is_empty() {
                    state.notification = Some(("手机号不能为空".to_string(), InfoBarKind::Warning));
                    return;
                }

                match CloudClient::login(phone, &state.login_password, dev_code) {
                    Ok(sess) => {
                        state.session = sess;
                        state.login_password.clear();
                        state.notification = Some(("登录成功！正在加载作业列表...".to_string(), InfoBarKind::Success));
                        if let Ok(list) = CloudClient::get_homework_list(&state.session, "1") {
                            state.homework_items = list;
                        }
                    }
                    Err(e) if e == "device_bind_required" => {
                        state.rebind_dialog_open = true;
                        state.rebind_code_input.clear();
                        match CloudClient::send_rebind_code(phone) {
                            Ok(_) => {
                                state.notification = Some(("检测到该账号需要绑定新设备，已自动向手机发送验证码，请输入验证码完成换绑".to_string(), InfoBarKind::Warning));
                            }
                            Err(send_err) => {
                                state.notification = Some((format!("检测到需要设备绑定，但自动发送验证码失败: {}。请点击“获取/重发验证码”", send_err), InfoBarKind::Warning));
                            }
                        }
                    }
                    Err(err) => {
                        state.notification = Some((format!("登录失败: {}", err), InfoBarKind::Error));
                    }
                }
            }

            if action_btn(ui, "🌐 畅言网页统一登录").clicked() {
                let changyan_url = "https://pass.changyan.com/login?nextpage=https%3A%2F%2Fwww.ets100.com%2FloginCheck.html&customConfig=aW4iLHByb2R1Y3RfYXBwa2V5OiJxaW5nZGFvX2V0cyIsIm5lZWRUaWNrZXQiOiJ0cnVlIiwibG9naW5fbm90QXV0byI6InRydWUifQ&from=ew&appId=pass6port18";
                let _ = open::that(changyan_url);
                state.notification = Some(("已在浏览器中拉起畅言登录页面".to_string(), InfoBarKind::Informational));
            }
        });
    });

    // 换绑弹窗
    if state.rebind_dialog_open {
        ui.card("设备换绑验证", "因更换设备或清除缓存，官方需要短信验证码确认换绑", |ui| {
            ui.label(format!("目标手机号: {}", state.session.phone));
            ui.text_box_hint(&mut state.rebind_code_input, "输入收到的 6 位短信验证码");

            responsive_flow(ui, |ui| {
                if action_btn(ui, "📩 获取/重发验证码").clicked() {
                    match CloudClient::send_rebind_code(&state.session.phone) {
                        Ok(_) => {
                            state.notification = Some(("验证码已发送至您的手机，请查收".to_string(), InfoBarKind::Informational));
                        }
                        Err(e) => {
                            state.notification = Some((format!("发送验证码失败: {}", e), InfoBarKind::Error));
                        }
                    }
                }

                if accent_action_btn(ui, "✔ 确认换绑并登录", None).clicked() {
                    let code = state.rebind_code_input.trim();
                    if code.is_empty() {
                        state.notification = Some(("请输入验证码".to_string(), InfoBarKind::Warning));
                        return;
                    }
                    match CloudClient::bind_device(
                        &state.session.phone,
                        &state.login_password,
                        &state.session.device_code,
                        code,
                    ) {
                        Ok(sess) => {
                            state.session = sess;
                            state.rebind_dialog_open = false;
                            state.login_password.clear();
                            state.rebind_code_input.clear();
                            state.notification = Some(("设备成功换绑并完成登录！正在加载作业列表...".to_string(), InfoBarKind::Success));
                            if let Ok(list) = CloudClient::get_homework_list(&state.session, "1") {
                                state.homework_items = list;
                            }
                        }
                        Err(err) => {
                            state.notification = Some((format!("换绑失败: {}", err), InfoBarKind::Error));
                        }
                    }
                }

                if action_btn(ui, "取消").clicked() {
                    state.rebind_dialog_open = false;
                }
            });
        });
    }
}

/// 智能格式化设备显示名称：提取人类可读友好名称，隐藏超长 User-Agent 字符串，支持鼠标悬浮查看完整标识
pub fn format_friendly_device_name(dev: &crate::la_client::LaDevice) -> (String, &'static str) {
    let raw = &dev.device_name;
    let raw_trim = raw.trim();
    let plat_lower = dev.platform_version.to_lowercase();
    let client_lower = dev.client_key.to_lowercase();

    // 1. 如果是网页端 UA
    if raw_trim.starts_with("Mozilla/") {
        let os = if raw_trim.contains("Windows NT 10.0") || raw_trim.contains("Windows NT 11.0") || raw_trim.contains("Windows") {
            "Windows PC"
        } else if raw_trim.contains("Macintosh") || raw_trim.contains("Mac OS X") {
            "Mac 电脑"
        } else if raw_trim.contains("Android") {
            "Android 设备"
        } else if raw_trim.contains("iPhone") || raw_trim.contains("iPad") {
            "iOS 设备"
        } else if raw_trim.contains("Linux") {
            "Linux PC"
        } else {
            "桌面端网页"
        };

        let browser = if raw_trim.contains("Edg/") {
            "Edge 浏览器"
        } else if raw_trim.contains("Chrome/") {
            "Chrome 浏览器"
        } else if raw_trim.contains("Firefox/") {
            "Firefox 浏览器"
        } else if raw_trim.contains("Safari/") {
            "Safari 浏览器"
        } else {
            "Web 浏览器"
        };

        return (format!("{os} · {browser}"), "🌐");
    }

    // 2. 权威识别 PC 设备：
    let is_windows = dev.is_current
        || plat_lower.contains("windows")
        || raw_trim.starts_with("PC-")
        || raw_trim.to_uppercase().contains("LAPTOP")
        || raw_trim.to_uppercase().contains("DESKTOP")
        || raw_trim.to_uppercase().contains("WORKSTATION")
        || raw_trim.to_uppercase().contains("WIN-")
        || client_lower.contains("windows")
        || client_lower == "fe_pc"
        || client_lower == "fe-pc";

    let is_mac = plat_lower.contains("mac")
        || plat_lower.contains("darwin")
        || raw_trim.contains("MacBook")
        || raw_trim.contains("iMac")
        || client_lower.contains("mac");

    let is_ios = plat_lower.contains("ios")
        || raw_trim.contains("iPhone")
        || raw_trim.contains("iPad");

    let is_android = !is_windows && !is_mac && !is_ios && (
        plat_lower.contains("android")
        || client_lower.contains("android")
        || client_lower == "fe"
    );

    let icon = if is_windows || is_mac {
        "💻"
    } else if is_android || is_ios {
        "📱"
    } else if raw_trim.contains("Windows") || raw_trim.contains("Desktop") {
        "💻"
    } else {
        "📱"
    };

    // 常规设备名称若过长（>32字），智能截断隐藏多余部分
    if raw_trim.chars().count() > 32 {
        let truncated: String = raw_trim.chars().take(29).collect();
        (format!("{truncated}..."), icon)
    } else if raw_trim.is_empty() {
        ("未知终端设备".to_string(), icon)
    } else {
        (raw_trim.to_string(), icon)
    }
}
