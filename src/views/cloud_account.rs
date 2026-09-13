use eframe::egui::{Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::InfoBarKind;

use crate::cloud_client::CloudClient;
use crate::state::AppState;
use crate::views::render_header;

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(ui, "云端账号管理", "登录 ETS100 账号以在线拉取作业和解析答案");

    if state.session.is_logged_in {
        ui.card("当前已登录账号", "ETS100 官方会话已激活", |ui| {
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

    // 未登录时显示登录表单
    ui.card("ETS 账号密码登录", "直接登录官方接口", |ui| {
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
                        // 自动拉取作业列表
                        if let Ok(list) = CloudClient::get_homework_list(&state.session, "1") {
                            state.homework_items = list;
                        }
                    }
                    Err(e) if e == "device_bind_required" => {
                        state.rebind_dialog_open = true;
                        state.notification = Some(("该账号在当前设备需要换绑，已准备发送短信验证码".to_string(), InfoBarKind::Warning));
                    }
                    Err(err) => {
                        state.notification = Some((format!("登录失败: {}", err), InfoBarKind::Error));
                    }
                }
            }

            if ui.button("🌐 畅言网页统一登录 (WebView2)").clicked() {
                let changyan_url = "https://pass.changyan.com/login?nextpage=https%3A%2F%2Fwww.ets100.com%2FloginCheck.html&customConfig=aW4iLHByb2R1Y3RfYXBwa2V5OiJxaW5nZGFvX2V0cyIsIm5lZWRUaWNrZXQiOiJ0cnVlIiwibG9naW5fbm90QXV0byI6InRydWUifQ&from=ew&appId=pass6port18";
                let _ = open::that(changyan_url);
                state.notification = Some(("已在浏览器/WebView2 中拉起畅言登录页面，登录后将自动同步 Token".to_string(), InfoBarKind::Informational));
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
                    if CloudClient::send_rebind_code(&state.session.phone).is_ok() {
                        state.notification = Some(("验证码已发送至您的手机，请查收".to_string(), InfoBarKind::Informational));
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
                        "",
                        &state.session.device_code,
                        code,
                    ) {
                        Ok(sess) => {
                            state.session = sess;
                            state.rebind_dialog_open = false;
                            state.notification = Some(("设备成功换绑并完成登录！".to_string(), InfoBarKind::Success));
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

    ui.add_space(8.0);

    // 风险提示
    ui.card("安全与风险提示", "使用云端模式须知", |ui| {
        ui.label(RichText::new("⚠ 注意事项:").strong().color(Color32::from_rgb(255, 179, 0)));
        ui.label("1. 云端登录会向 ETS100 接口发起认证，若手机官方客户端正在运行可能触发“账号在其他设备登录”顶号。");
        ui.label("2. 请勿在正在进行正式口语考试、录音作答提交的过程中使用云端模式。");
        ui.label("3. 设备码在本机保存并加密，避免频繁更换触发官方风控设备限制。");
    });
}
