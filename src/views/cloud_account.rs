use eframe::egui::{Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::cloud_client::CloudClient;
use crate::la_client::LaClient;
use crate::models::UserSession;
use crate::state::AppState;
use crate::views::render_header;

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(
        ui,
        "账号与云端授权中心",
        "管理 LA Studio 会员账号、云端托管代登中继及 ETS100 官方直连会话",
    );

    // 模式切换分栏选择
    ui.row(|ui| {
        let is_la_tab = state.la_active_tab == 0;
        let is_ets_tab = state.la_active_tab == 1;

        if is_la_tab {
            let _ = ui.accent_button("✨ LA Studio 会员与云端托管 (推荐)");
        } else if ui.button("✨ LA Studio 会员与云端托管 (推荐)").clicked() {
            state.la_active_tab = 0;
        }

        if is_ets_tab {
            let _ = ui.accent_button("📱 ETS100 官方直连模式");
        } else if ui.button("📱 ETS100 官方直连模式").clicked() {
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
            ui.row(|ui| {
                ui.label(RichText::new(format!("👤 用户名: {}", la_sess.user.username)).strong().size(16.0));
                let tier_text = la_sess.user.tier_display_name();
                let tier_color = match la_sess.user.tier.to_lowercase().as_str() {
                    "ultra" => Color32::from_rgb(255, 215, 0), // Gold
                    "pro" => Color32::from_rgb(0, 188, 212),    // Cyan
                    _ => Color32::from_rgb(158, 158, 158),
                };
                ui.label(RichText::new(format!(" [ {tier_text} ] ")).strong().color(tier_color));
                ui.label(format!("(ID: {})", la_sess.user.id));
            });

            ui.add_space(4.0);
            ui.label(format!("🔑 会话编号: {}", la_sess.session_id));
            if let Some(ref dev_id) = la_sess.device_id {
                ui.label(format!("💻 绑定设备 ID: {dev_id}"));
            }

            ui.separator();

            ui.row(|ui| {
                if ui.button("🚪 退出登录").clicked() {
                    LaClient::logout(&la_sess.access_token);
                    state.la_session = None;
                    state.la_cloud_session = None;
                    state.la_devices.clear();
                    state.notification = Some(("已退出 LA Studio 账号".to_string(), InfoBarKind::Informational));
                }

                if ui.button("🔄 刷新设备列表").clicked() {
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
            });
        });

        ui.add_space(8.0);

        // 云端免密中继激活卡片
        ui.card("ETS100 云端免密中继代理", "由 LA Studio 云端代登获取学生会话，无需本地短信验证码", |ui| {
            if let Some(ref cloud) = state.la_cloud_session {
                ui.label(RichText::new(format!("🟢 当前已激活代理: {}", cloud.account_display)).strong().color(Color32::from_rgb(76, 175, 80)));
                ui.label(format!("关联账号 ID: {}", cloud.parent_account_id));
                ui.label(format!("设备识别码: {}", cloud.device_code));
                ui.add_space(4.0);
            } else {
                ui.label(RichText::new("⚪ 尚未激活云端 ETS100 会话").color(Color32::from_rgb(180, 180, 180)));
            }

            ui.row(|ui| {
                if ui.accent_button("🚀 一键激活 / 刷新云端 ETS100 会话").clicked() {
                    match LaClient::get_cloud_session(&la_sess.access_token, "") {
                        Ok(cloud) => {
                            state.session = UserSession {
                                phone: cloud.account_display.clone(),
                                token: cloud.token.clone(),
                                device_code: cloud.device_code.clone(),
                                parent_account_id: cloud.parent_account_id.clone(),
                                user_name: cloud.account_display.clone(),
                                is_logged_in: true,
                            };
                            state.la_cloud_session = Some(cloud);
                            state.notification = Some(("云端 ETS100 会话激活成功！正在加载作业...".to_string(), InfoBarKind::Success));

                            // 自动同步拉取最新作业
                            if let Ok(hw_list) = CloudClient::get_homework_list(&state.session, "1") {
                                state.homework_items = hw_list;
                            }
                        }
                        Err(e) => {
                            state.notification = Some((format!("获取云端中继会话失败: {e}"), InfoBarKind::Error));
                        }
                    }
                }
            });
        });

        ui.add_space(8.0);

        // ETS100 账号绑定与托管卡片
        ui.card("关联 ETS100 账号至 LA 云端", "将学生 e听说 账号密码安全托管至 LA 云端，享受一键免密做题", |ui| {
            ui.label("输入您的 ETS100 学生手机号与密码，由 LA Studio 云端安全托管代登：");
            ui.add_space(4.0);
            ui.text_box_hint(&mut state.la_bind_phone_input, "ETS100 注册手机号");
            ui.add_space(4.0);
            ui.password_box(&mut state.la_bind_password_input);
            ui.add_space(8.0);

            ui.row(|ui| {
                if ui.accent_button("🔗 关联此 ETS 账号并拉取凭据").clicked() {
                    let phone = state.la_bind_phone_input.trim();
                    let pwd = state.la_bind_password_input.trim();

                    if phone.is_empty() || pwd.is_empty() {
                        state.notification = Some(("手机号和密码不能为空".to_string(), InfoBarKind::Warning));
                        return;
                    }

                    let dev_code = CloudClient::get_or_generate_device_code(None);
                    match LaClient::bind_ets100_account(&la_sess.access_token, phone, pwd, &dev_code) {
                        Ok(_) => {
                            state.la_bind_password_input.clear();

                            // 立即同步拉取凭据
                            match LaClient::get_cloud_session(&la_sess.access_token, "") {
                                Ok(cloud) => {
                                    state.session = UserSession {
                                        phone: cloud.account_display.clone(),
                                        token: cloud.token.clone(),
                                        device_code: cloud.device_code.clone(),
                                        parent_account_id: cloud.parent_account_id.clone(),
                                        user_name: cloud.account_display.clone(),
                                        is_logged_in: true,
                                    };
                                    state.la_cloud_session = Some(cloud);
                                    state.notification = Some(("云端 ETS 做题凭据同步成功！正在加载作业列表...".to_string(), InfoBarKind::Success));
                                    if let Ok(hw_list) = CloudClient::get_homework_list(&state.session, "1") {
                                        state.homework_items = hw_list;
                                    }
                                }
                                Err(e) => {
                                    state.notification = Some((format!("账号关联成功，但拉取凭据异常: {e}"), InfoBarKind::Warning));
                                }
                            }
                        }
                        Err(e) => {
                            state.notification = Some((format!("关联 ETS 账号失败: {e}"), InfoBarKind::Error));
                        }
                    }
                }
            });
        });

        // 设备管理卡片
        if !state.la_devices.is_empty() {
            ui.add_space(8.0);
            ui.card("已绑定的授权设备", "管理您在 LA Studio 登录过的终端设备", |ui| {
                for dev in state.la_devices.clone() {
                    ui.row(|ui| {
                        let cur_tag = if dev.is_current { " [当前设备]" } else { "" };
                        ui.label(RichText::new(format!("📱 {}{}", dev.device_name, cur_tag)).strong());
                        ui.label(format!("系统: {}", dev.platform_version));
                        ui.label(format!("客户端: {}", dev.client_key));

                        if !dev.is_current
                            && ui.button("解除授权").clicked() {
                                match LaClient::revoke_device(&la_sess.access_token, dev.id) {
                                    Ok(_) => {
                                        state.la_devices.retain(|d| d.id != dev.id);
                                        state.notification = Some(("设备已成功解绑".to_string(), InfoBarKind::Success));
                                    }
                                    Err(e) => {
                                        state.notification = Some((format!("解绑失败: {e}"), InfoBarKind::Error));
                                    }
                                }
                            }
                    });
                    ui.separator();
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

            ui.row(|ui| {
                if ui.accent_button("🔑 登录 LA 账号").clicked() {
                    let u = state.la_username_input.trim();
                    let p = &state.la_password_input;

                    if u.is_empty() || p.is_empty() {
                        state.notification = Some(("用户名和密码不能为空".to_string(), InfoBarKind::Warning));
                        return;
                    }

                    let cap_token_opt = if state.la_captcha_token.trim().is_empty() {
                        None
                    } else {
                        Some(state.la_captcha_token.trim())
                    };

                    match LaClient::login(u, p, cap_token_opt) {
                        Ok(sess) => {
                            state.la_session = Some(sess.clone());
                            state.la_password_input.clear();
                            state.la_captcha_needed = false;
                            state.la_captcha_token.clear();
                            state.notification = Some((format!("欢迎回来，{}！", sess.user.username), InfoBarKind::Success));

                            // 尝试自动激活绑定的 ETS 云端会话
                            if let Ok(cloud) = LaClient::get_cloud_session(&sess.access_token, "") {
                                state.session = UserSession {
                                    phone: cloud.account_display.clone(),
                                    token: cloud.token.clone(),
                                    device_code: cloud.device_code.clone(),
                                    parent_account_id: cloud.parent_account_id.clone(),
                                    user_name: cloud.account_display.clone(),
                                    is_logged_in: true,
                                };
                                state.la_cloud_session = Some(cloud);
                                if let Ok(hw_list) = CloudClient::get_homework_list(&state.session, "1") {
                                    state.homework_items = hw_list;
                                }
                            }

                            // 预拉取设备列表
                            if let Ok(devs) = LaClient::list_devices(&sess.access_token) {
                                state.la_devices = devs;
                            }
                        }
                        Err(e) => {
                            if e.contains("人机验证") {
                                state.la_captcha_needed = true;
                                if state.captcha_server.is_none() {
                                    if let Ok(srv) = crate::utils::captcha_server::CaptchaServer::start() {
                                        let port = srv.port();
                                        state.captcha_server = Some(srv);
                                        let _ = open::that(format!("http://127.0.0.1:{port}/"));
                                    }
                                } else if let Some(srv) = &state.captcha_server {
                                    let _ = open::that(format!("http://127.0.0.1:{}/", srv.port()));
                                }
                                state.notification = Some(("需要完成人机安全验证，已为您在浏览器打开验证页面".to_string(), InfoBarKind::Warning));
                            } else {
                                state.notification = Some((format!("登录失败: {e}"), InfoBarKind::Error));
                            }
                        }
                    }
                }
            });
        });

        if state.la_captcha_needed {
            ui.add_space(8.0);
            ui.card("🛡️ 人机安全验证", "服务端要求完成人机安全验证", |ui| {
                ui.label(RichText::new("服务端要求进行安全验证。完成验证后，桌面端将自动恢复并登录：").color(Color32::from_rgb(255, 180, 0)));
                ui.add_space(4.0);
                ui.row(|ui| {
                    if ui.accent_button("🌐 在默认浏览器中打开验证页面").clicked() {
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
                ui.row(|ui| {
                    ui.text_box_hint(&mut state.la_captcha_token, "粘贴 cap_token");
                    if ui.button("带 Token 登录").clicked() {
                        let u = state.la_username_input.trim();
                        let p = &state.la_password_input;
                        let token = state.la_captcha_token.trim();
                        if !token.is_empty() {
                            match LaClient::login(u, p, Some(token)) {
                                Ok(sess) => {
                                    state.la_session = Some(sess.clone());
                                    state.la_captcha_needed = false;
                                    state.la_captcha_token.clear();
                                    state.la_password_input.clear();
                                    state.notification = Some((format!("欢迎回来，{}！", sess.user.username), InfoBarKind::Success));
                                    if let Ok(cloud) = LaClient::get_cloud_session(&sess.access_token, "") {
                                        state.session = UserSession {
                                            phone: cloud.account_display.clone(),
                                            token: cloud.token.clone(),
                                            device_code: cloud.device_code.clone(),
                                            parent_account_id: cloud.parent_account_id.clone(),
                                            user_name: cloud.account_display.clone(),
                                            is_logged_in: true,
                                        };
                                        state.la_cloud_session = Some(cloud);
                                        if let Ok(hw_list) = CloudClient::get_homework_list(&state.session, "1") {
                                            state.homework_items = hw_list;
                                        }
                                    }
                                    if let Ok(devs) = LaClient::list_devices(&sess.access_token) {
                                        state.la_devices = devs;
                                    }
                                }
                                Err(e) => {
                                    state.notification = Some((format!("带 Token 登录失败: {e}"), InfoBarKind::Error));
                                }
                            }
                        }
                    }
                });
            });
        }
    }
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

            ui.row(|ui| {
                if ui.button("🚪 退出当前登录").clicked() {
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
            state.session.device_code = CloudClient::get_or_generate_device_code(None);
        }
        ui.text_box_hint(&mut state.session.device_code, "设备硬件码 (默认自动生成并持久化)");

        ui.add_space(8.0);

        ui.row(|ui| {
            if ui.accent_button("🔑 立即登录").clicked() {
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

            if ui.button("🌐 畅言网页统一登录").clicked() {
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

            ui.row(|ui| {
                if ui.button("📩 获取/重发验证码").clicked() {
                    match CloudClient::send_rebind_code(&state.session.phone) {
                        Ok(_) => {
                            state.notification = Some(("验证码已发送至您的手机，请查收".to_string(), InfoBarKind::Informational));
                        }
                        Err(e) => {
                            state.notification = Some((format!("发送验证码失败: {}", e), InfoBarKind::Error));
                        }
                    }
                }

                if ui.accent_button("✔ 确认换绑并登录").clicked() {
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

                if ui.button("取消").clicked() {
                    state.rebind_dialog_open = false;
                }
            });
        });
    }
}
