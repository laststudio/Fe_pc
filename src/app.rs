use eframe::egui::{self, CentralPanel, Color32, Frame, Margin, Panel, RichText, ScrollArea};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::{self, BackdropKind, InfoBarKind, NavigationView, NavigationViewItem};

use crate::cloud_client::CloudClient;
use crate::local::LocalScanner;
use crate::state::{AppState, NavTab};
use crate::views::{
    answer_view, auto_complete_view, cloud_account, credits, homework_list, local_scan, settings,
};

pub struct FepcApp {
    pub state: AppState,
    pub selected_nav_index: usize,
    pub logo_texture: Option<egui::TextureHandle>,
    pub frame_count: u32,
    pub screenshot_target: Option<String>,
}

impl FepcApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        crate::telemetry::start_telemetry_pipeline();

        let mut state = AppState::default();
        let screenshot_target = std::env::var("FEPC_SCREENSHOT").ok();

        // 最终用户许可协议 (EULA) 首次启动门禁检测
        let eula_accepted = crate::eula::is_eula_accepted();
        state.eula_accepted = eula_accepted;
        if !eula_accepted {
            state.open_eula_modal(true);
        }

        // 拉取启动策略门禁与版本检查
        if let Ok(policy) = crate::la_client::LaClient::check_software_policy() {
            if policy.is_kill_switch_on {
                let msg = policy.announcement_message.as_deref().unwrap_or("客户端服务已被紧急停用 (Kill Switch 激活)，所有自动化流水线已锁定。");
                state.notification = Some((format!("【紧急停用阻断】{msg}"), InfoBarKind::Error));
                state.auto_running = false;
            } else if policy.latest_version_code > crate::la_client::LA_APP_VERSION_CODE {
                let msg = format!("发现新版本 (v{})，建议及时更新！", policy.latest_version_code);
                state.notification = Some((msg, InfoBarKind::Warning));
            } else if let Some(ref title) = policy.announcement_title
                && !title.trim().is_empty() {
                let msg = policy.announcement_message.as_deref().unwrap_or("");
                state.notification = Some((format!("【公告: {title}】{msg}"), InfoBarKind::Informational));
            }
            state.software_policy = Some(policy);
        }

        // 动态加载云端功能权限与防篡改规则字典
        if let Ok(features) = crate::la_client::LaClient::fetch_public_features() {
            for f in features {
                state.features.insert(f.feature_key.clone(), f);
            }
            state.features_loaded = true;
        }

        // 尝试加载已保存的 LA Studio 会话
        if let Some(la_sess) = crate::la_client::LaClient::load_saved_session() {
            state.la_session = Some(la_sess.clone());
            // 如果已保存 LA 会话，尝试加载用户个性化授权
            if let Ok(user_features) = crate::la_client::LaClient::fetch_user_features(&la_sess.access_token) {
                for f in user_features {
                    state.features.insert(f.feature_key.clone(), f);
                }
                state.features_loaded = true;
            }
            // 尝试恢复云端托管的 ETS100 会话
            if let Ok(cloud_sess) = crate::la_client::LaClient::get_cloud_session(&la_sess.access_token, "") {
                state.session = crate::models::UserSession {
                    phone: cloud_sess.account_display.clone(),
                    token: cloud_sess.token.clone(),
                    device_code: cloud_sess.device_code.clone(),
                    parent_account_id: cloud_sess.parent_account_id.clone(),
                    user_name: cloud_sess.account_display.clone(),
                    is_logged_in: true,
                };
                state.la_cloud_session = Some(cloud_sess);
            }
            // 接入 Ed25519 鉴权平滑门控
            let _ = crate::la_client::LaClient::authorize_feature(&la_sess.access_token, "answer_parser");
            // 加载在线会话列表
            if let Ok(sessions) = crate::la_client::LaClient::list_sessions(&la_sess.access_token) {
                state.la_account_sessions = sessions;
            }
        }

        // 尝试加载已保存的官方 ETS 直连会话 (如果未被 LA 云端会话覆盖)
        if !state.session.is_logged_in
            && let Some(session) = CloudClient::load_saved_session() {
                state.session = session;
            }

        // 若会话有效，静默加载作业列表
        if state.session.is_logged_in
            && let Ok(hw_list) = CloudClient::get_homework_list(&state.session, "1") {
                state.homework_items = hw_list;
            }

        // 自动发现本地 ETS 客户端目录
        state.local_ets_dirs = LocalScanner::find_ets_local_dirs();
        if let Some(first) = state.local_ets_dirs.first() {
            state.selected_local_dir = Some(first.clone());
            state.local_items = LocalScanner::scan_homeworks(first);
        }

        let mut selected_nav_index = 0;

        if let Ok(tab_str) = std::env::var("FEPC_INITIAL_TAB") {
            match tab_str.as_str() {
                "homework_list" => {
                    state.current_tab = NavTab::HomeworkList;
                    selected_nav_index = 0;
                }
                "local_scan" => {
                    state.current_tab = NavTab::LocalScan;
                    selected_nav_index = 1;
                }
                "auto_complete" => {
                    state.current_tab = NavTab::AutoComplete;
                    selected_nav_index = 2;
                }
                "cloud_account" => {
                    state.current_tab = NavTab::CloudAccount;
                    selected_nav_index = 3;
                }
                "settings" => {
                    state.current_tab = NavTab::Settings;
                    selected_nav_index = 4;
                }
                "credits" => {
                    state.current_tab = NavTab::Credits;
                    selected_nav_index = 5;
                }
                _ => {}
            }
        }

        if let Ok(la_tab) = std::env::var("FEPC_LA_ACTIVE_TAB")
            && let Ok(idx) = la_tab.parse::<usize>() {
                state.la_active_tab = idx;
            }

        if std::env::var("FEPC_AUTO_OPEN_ANSWER").is_ok()
            && let Some(first) = state.local_items.first() {
                state.selected_homework = Some(first.clone());
                state.answer_view_mode = crate::state::AnswerViewMode::Detailed;
            }

        if std::env::var("FEPC_AUTO_OPEN_CONCISE").is_ok()
            && let Some(first) = state.local_items.first() {
                state.selected_homework = Some(first.clone());
                state.answer_view_mode = crate::state::AnswerViewMode::Concise;
            }

        if std::env::var("FEPC_TEST_LIGHT_THEME").is_ok() {
            state.theme_config.theme = fluent_egui::winui::FluentTheme::Light;
        }

        if std::env::var("FEPC_TEST_AUTO_RUNNING").is_ok() {
            state.la_session = Some(crate::la_client::LaSession {
                session_id: "test_sess".to_string(),
                device_id: None,
                user: crate::la_client::LaUser {
                    id: 1337,
                    username: "leitianshuo".to_string(),
                    member_level: 2,
                    tier: "ultra".to_string(),
                    avatar_url: None,
                },
                access_token: "test".to_string(),
                refresh_token: "test".to_string(),
                cached_at: 0,
            });
            state.la_cloud_session = Some(crate::la_client::LaCloudSession {
                account_display: "学生雷天硕 (138****0000)".to_string(),
                token: "token".to_string(),
                device_code: "DEV".to_string(),
                parent_account_id: "1".to_string(),
                selected_account: serde_json::json!({}),
                valid_accounts: vec![],
            });
            state.session.token = "token".to_string();
            state.auto_running = true;
            state.auto_progress = Some((5, 8, "正在合成第 3 题语音 (Edge TTS) 并生成评测 XML...".to_string()));
            state.auto_logs = vec![
                "🚀 启动作业 [本地试卷 675819] 8阶段全自动化做题流水线...".to_string(),
                "✓ 题目结构就绪: 共 14 题，目标总分: 28.5 / 30.0 分".to_string(),
                "✓ 云端会话开启成功 (checkstart: OK)".to_string(),
                "✓ [阶段 3/8] 成功获取阿里云 OSS 凭据与上传策略".to_string(),
                "👉 [5/14] 正在处理第 3 题 (collector.choose)...".to_string(),
            ];
            state.auto_questions_done = vec![
                ("1".to_string(), "听后选择".to_string(), 2.0, 2.0, "已完成".to_string(), None),
                ("2".to_string(), "短文朗读".to_string(), 3.8, 4.0, "已完成".to_string(), None),
            ];
        }

        Self {
            state,
            selected_nav_index,
            logo_texture: None,
            frame_count: 0,
            screenshot_target,
        }
    }
}

impl eframe::App for FepcApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.frame_count = self.frame_count.saturating_add(1);

        // 首帧安全追加加载 Windows 系统 Emoji 与 Symbol 字体库（杜绝未初始化构造期调用 ctx.fonts 导致的 panic）
        if self.frame_count == 1 {
            #[cfg(windows)]
            {
                let win_fonts = std::path::Path::new("C:\\Windows\\Fonts");
                let has_emj = win_fonts.join("seguiemj.ttf").exists();
                let has_sym = win_fonts.join("seguisym.ttf").exists();
                if has_emj || has_sym {
                    let mut font_defs = ui.ctx().fonts(|f| f.definitions().clone());
                    let mut modified = false;

                    if let Ok(bytes) = std::fs::read(win_fonts.join("seguiemj.ttf")) {
                        font_defs.font_data.insert(
                            "Segoe UI Emoji".to_string(),
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(bytes)),
                        );
                        font_defs
                            .families
                            .entry(eframe::egui::FontFamily::Proportional)
                            .or_default()
                            .push("Segoe UI Emoji".to_string());
                        modified = true;
                    }

                    if let Ok(bytes) = std::fs::read(win_fonts.join("seguisym.ttf")) {
                        font_defs.font_data.insert(
                            "Segoe UI Symbol".to_string(),
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(bytes)),
                        );
                        font_defs
                            .families
                            .entry(eframe::egui::FontFamily::Proportional)
                            .or_default()
                            .push("Segoe UI Symbol".to_string());
                        modified = true;
                    }

                    if modified {
                        ui.ctx().set_fonts(font_defs);
                    }
                }
            }
        }

        // 自动化截图捕获支持
        if let Some(ref target_path) = self.screenshot_target.clone() {
            let screenshot = ui.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });

            if let Some(image) = screenshot {
                let w = image.size[0];
                let h = image.size[1];
                if w > 0 && h > 0 {
                    let rgba: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    let path = std::path::PathBuf::from(target_path);
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = image::save_buffer(&path, &rgba, w as u32, h as u32, image::ColorType::Rgba8);
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                self.screenshot_target = None;
            } else if self.frame_count >= 3 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new("fepc-snap")));
                ui.ctx().request_repaint();
            } else {
                ui.ctx().request_repaint();
            }
        }

        // 0. 轮询本地人机验证服务
        if let Some(ref server) = self.state.captcha_server {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            if let Some(token) = server.try_recv() {
                self.state.la_captcha_token = token.clone();
                self.state.captcha_server = None;
                self.state.la_captcha_needed = false;
                self.state.notification = Some(("已收到人机验证凭据，正在自动登录...".to_string(), InfoBarKind::Informational));

                let u = self.state.la_username_input.clone();
                let p = self.state.la_password_input.clone();
                match crate::la_client::LaClient::login(&u, &p, Some(&token)) {
                    Ok(session) => {
                        let ets_result = crate::la_client::LaClient::get_cloud_session(&session.access_token, "");
                        match ets_result {
                            Ok(cloud_sess) => {
                                self.state.session.phone = cloud_sess.account_display.clone();
                                self.state.session.token = cloud_sess.token.clone();
                                self.state.session.device_code = cloud_sess.device_code.clone();
                                self.state.session.parent_account_id = cloud_sess.parent_account_id.clone();
                                self.state.session.is_logged_in = true;
                                if let Some(name) = cloud_sess.selected_account.get("name").and_then(|v| v.as_str()) {
                                    self.state.session.user_name = name.to_string();
                                } else {
                                    self.state.session.user_name = cloud_sess.account_display.clone();
                                }
                                self.state.la_cloud_session = Some(cloud_sess);
                                self.state.notification = Some((
                                    format!("LA 会员登录成功！已自动托管激活 ETS100 学生会话 ({})", self.state.session.user_name),
                                    InfoBarKind::Success,
                                ));
                                if let Ok(hws) = CloudClient::get_homework_list(&self.state.session, "1") {
                                    self.state.homework_items = hws;
                                }
                            }
                            Err(_) => {
                                self.state.notification = Some((
                                    format!("LA 会员 [{}] 登录成功！", session.user.username),
                                    InfoBarKind::Success,
                                ));
                            }
                        }
                        self.state.la_devices = crate::la_client::LaClient::list_devices(&session.access_token).unwrap_or_default();
                        self.state.la_session = Some(session);
                    }
                    Err(e) => {
                        self.state.notification = Some((format!("登录失败: {e}"), InfoBarKind::Error));
                    }
                }
            }
        }

        // 1. 同步主题配置与窗口材质
        self.state.theme_config = self.state.theme_config.with_backdrop(self.state.backdrop_kind);
        winui::apply_theme(ui.ctx(), self.state.theme_config);
        if let Some(window) = frame.winit_window() {
            let _ = winui::sync_window_backdrop(
                window,
                self.state.theme_config.theme,
                self.state.backdrop_kind,
            );
        }

        // 自动消失通知倒计时管理 (6 秒后平滑淡出，鼠标悬浮时仅顺延超时，绝不篡改进场动画时间戳)
        let current_time = ui.ctx().input(|i| i.time);
        if let Some((ref msg, _)) = self.state.notification {
            if self.state.last_notification_text.as_deref() != Some(msg.as_str()) {
                self.state.last_notification_text = Some(msg.clone());
                self.state.notification_created_at = Some(current_time);
                self.state.notification_expires_at = Some(current_time + 6.0);
                self.state.notification_dismissing = false;
                self.state.notification_dismiss_start = None;
            }

            if !self.state.notification_dismissing
                && let Some(expires_at) = self.state.notification_expires_at
                && current_time >= expires_at
            {
                self.state.notification_dismissing = true;
                self.state.notification_dismiss_start = Some(current_time);
            }
        } else {
            self.state.last_notification_text = None;
            self.state.notification_created_at = None;
            self.state.notification_expires_at = None;
            self.state.notification_dismissing = false;
            self.state.notification_dismiss_start = None;
        }

        if self.logo_texture.is_none() {
            let texture_options = egui::TextureOptions {
                magnification: egui::TextureFilter::Linear,
                minification: egui::TextureFilter::Linear,
                wrap_mode: egui::TextureWrapMode::ClampToEdge,
                mipmap_mode: Some(egui::TextureFilter::Linear),
            };
            let raw_bytes = include_bytes!("../fe_logo_1024.png");
            if let Ok(dyn_img) = image::load_from_memory(raw_bytes) {
                let rgba = dyn_img.to_rgba8();
                let width = dyn_img.width() as usize;
                let height = dyn_img.height() as usize;
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [width, height],
                    &rgba.into_raw(),
                );
                self.logo_texture = Some(ui.ctx().load_texture("app_logo", color_image, texture_options));
            } else if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../fe_logo.png")) {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                );
                self.logo_texture = Some(ui.ctx().load_texture("app_logo", color_image, texture_options));
            }
        }

        let is_dark = self.state.theme_config.theme.is_dark();

        // 2. 窗口与侧边栏背景材质联动：Mica / Acrylic / Tabbed 启用时透出 Windows DWM 磨砂质感；None 纯色不透明杜绝透光
        let (window_bg, sidebar_bg) = match self.state.backdrop_kind {
            BackdropKind::None => {
                if is_dark {
                    (Color32::from_rgb(32, 32, 32), Color32::from_rgb(25, 25, 25))
                } else {
                    (Color32::from_rgb(243, 243, 243), Color32::from_rgb(235, 235, 235))
                }
            }
            BackdropKind::Acrylic => {
                // 亚克力材质：适度透光 (Alpha 170 / 140)，呈现磨砂半透质感
                if is_dark {
                    (Color32::from_rgba_unmultiplied(32, 32, 32, 175), Color32::from_rgba_unmultiplied(22, 22, 22, 140))
                } else {
                    (Color32::from_rgba_unmultiplied(243, 243, 243, 175), Color32::from_rgba_unmultiplied(235, 235, 235, 140))
                }
            }
            BackdropKind::Mica | BackdropKind::Tabbed | BackdropKind::Auto => {
                // Mica / Tabbed 云母材质：让 Windows 11 DWM 材质纹理透入主窗口与侧边栏 (Dark Alpha: 195/160)
                if is_dark {
                    (Color32::from_rgba_unmultiplied(32, 32, 32, 195), Color32::from_rgba_unmultiplied(24, 24, 24, 160))
                } else {
                    (Color32::from_rgba_unmultiplied(243, 243, 243, 195), Color32::from_rgba_unmultiplied(235, 235, 235, 160))
                }
            }
            _ => {
                if is_dark {
                    (Color32::from_rgb(32, 32, 32), Color32::from_rgb(25, 25, 25))
                } else {
                    (Color32::from_rgb(243, 243, 243), Color32::from_rgb(235, 235, 235))
                }
            }
        };

        // 3. 左侧固定侧边栏 (Panel::left 保证高度 100% 撑满窗口，杜绝被内容高度限制)
        Panel::left("sidebar_nav")
            .exact_size(220.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(sidebar_bg)
                    .inner_margin(Margin::symmetric(16, 20)),
            )
            .show(ui, |ui| {
                // 应用标题与 Logo (采用 1024x1024 硬件 Trilinear Mipmap 过滤超高清 fe_logo)
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.logo_texture {
                        ui.image((tex.id(), egui::vec2(48.0, 48.0)));
                        ui.add_space(8.0);
                    }
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new("Fe PC")
                                .strong()
                                .size(20.0)
                                .color(Color32::from_rgb(0, 180, 255)),
                        );
                        ui.add_space(1.0);
                        ui.label(
                            RichText::new("e听说提取器 · 桌面版")
                                .size(11.0)
                                .color(Color32::from_gray(160)),
                        );
                    });
                });

                let mut fluent_nav = FluentUi::from_ui(ui);
                fluent_nav.add_space(16.0);

                // 侧边栏导航条目
                let nav_items = [
                    NavigationViewItem::new("作业与试卷", FluentIcon::Home),
                    NavigationViewItem::new("本地试卷扫描", FluentIcon::OpenFile),
                    NavigationViewItem::new("⚡ 自动做题", FluentIcon::Play),
                    NavigationViewItem::new("云端账号管理", FluentIcon::Save),
                    NavigationViewItem::new("设置与偏好", FluentIcon::Settings),
                    NavigationViewItem::new("鸣谢与组件", FluentIcon::Info),
                ];

                // 同步选中的索引与 Tab
                let prev_tab = match self.selected_nav_index {
                    0 => NavTab::HomeworkList,
                    1 => NavTab::LocalScan,
                    2 => NavTab::AutoComplete,
                    3 => NavTab::CloudAccount,
                    4 => NavTab::Settings,
                    5 => NavTab::Credits,
                    _ => NavTab::HomeworkList,
                };

                if self.state.current_tab != prev_tab {
                    self.selected_nav_index = match self.state.current_tab {
                        NavTab::HomeworkList => 0,
                        NavTab::LocalScan => 1,
                        NavTab::AutoComplete => 2,
                        NavTab::CloudAccount => 3,
                        NavTab::Settings => 4,
                        NavTab::Credits => 5,
                    };
                }

                NavigationView::new(&mut self.selected_nav_index, &nav_items)
                    .width(188.0)
                    .show(&mut fluent_nav);

                self.state.current_tab = match self.selected_nav_index {
                    0 => NavTab::HomeworkList,
                    1 => NavTab::LocalScan,
                    2 => NavTab::AutoComplete,
                    3 => NavTab::CloudAccount,
                    4 => NavTab::Settings,
                    5 => NavTab::Credits,
                    _ => NavTab::HomeworkList,
                };

                if self.selected_nav_index != 0 {
                    self.state.selected_homework = None;
                }

                fluent_nav.add_space(24.0);

                // 底部会话状态小贴士
                fluent_nav.separator();
                if self.state.session.is_logged_in {
                    fluent_nav.label(
                        RichText::new("● 账号在线")
                            .color(Color32::from_rgb(76, 175, 80))
                            .size(12.0),
                    );
                    fluent_nav.label(
                        RichText::new(&self.state.session.phone)
                            .size(11.0)
                            .color(Color32::from_gray(160)),
                    );
                } else {
                    fluent_nav.label(
                        RichText::new("○ 离线/未登录")
                            .color(Color32::from_gray(140))
                            .size(12.0),
                    );
                }
            });

        // 4. 右侧 CentralPanel：全高、全宽、整页自然平滑垂直滚动 (杜绝遮挡、内容缺失和错位)
        CentralPanel::default()
            .frame(Frame::new().fill(window_bg))
            .show(ui, |ui| {
                // 每个页面与视图分配独立的 ScrollArea ID，彻底杜绝跨页面滚动状态同步/污染的 Bug
                let page_scroll_id = if let Some(hw) = &self.state.selected_homework {
                    format!("scroll_answer_{}_{:?}", hw.id, self.state.answer_view_mode)
                } else {
                    match self.state.current_tab {
                        NavTab::HomeworkList => "scroll_page_homework_list".to_string(),
                        NavTab::LocalScan => "scroll_page_local_scan".to_string(),
                        NavTab::AutoComplete => "scroll_page_auto_complete".to_string(),
                        NavTab::CloudAccount => "scroll_page_cloud_account".to_string(),
                        NavTab::Settings => "scroll_page_settings".to_string(),
                        NavTab::Credits => "scroll_page_credits".to_string(),
                    }
                };

                ScrollArea::vertical()
                    .id_salt(page_scroll_id)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // 动态断点边距：窗口较窄时（如 < 580px），将左右内边距从 24 紧缩至 12，给内容更多排版空间
                        let avail_w = ui.available_width();
                        let pad_x: i8 = if avail_w < 580.0 { 12 } else { 24 };

                        Frame::new()
                            .inner_margin(Margin {
                                left: pad_x,
                                right: pad_x,
                                top: 20,
                                bottom: 36,
                            })
                            .show(ui, |ui| {
                                // 严格锁定内容可用宽度（双向 min / max 锁定，彻底切断子组件将父容器横向撑爆的通道）
                                let content_w = ui.available_width();
                                ui.set_min_width(content_w);
                                ui.set_max_width(content_w);

                                // 启动策略门禁与版本更新横幅
                                if let Some(ref policy) = self.state.software_policy {
                                    let is_dark = self.state.theme_config.theme.is_dark();
                                    if policy.is_kill_switch_on {
                                        egui::Frame::new()
                                            .corner_radius(6.0)
                                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(220, 50, 50)))
                                            .fill(if is_dark { Color32::from_rgba_unmultiplied(60, 20, 20, 230) } else { Color32::from_rgb(253, 231, 233) })
                                            .inner_margin(egui::Margin::symmetric(14, 10))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new("🛑").size(16.0));
                                                    ui.label(RichText::new("【服务已熔断停用】").strong().color(Color32::from_rgb(220, 50, 50)));
                                                    let msg = policy.announcement_message.as_deref().unwrap_or("客户端服务已被紧急停用 (Kill Switch 激活)，所有自动化流水线已锁定。");
                                                    ui.label(RichText::new(msg).color(if is_dark { Color32::WHITE } else { Color32::BLACK }));
                                                });
                                            });
                                        ui.add_space(8.0);
                                    } else if policy.latest_version_code > crate::la_client::LA_APP_VERSION_CODE {
                                        egui::Frame::new()
                                            .corner_radius(6.0)
                                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0, 150, 255)))
                                            .fill(if is_dark { Color32::from_rgba_unmultiplied(20, 40, 60, 220) } else { Color32::from_rgb(235, 245, 255) })
                                            .inner_margin(egui::Margin::symmetric(14, 10))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new("🚀").size(15.0));
                                                    ui.label(RichText::new(format!("发现新版本 (v{})！当前版本: v{}", policy.latest_version_code, crate::la_client::LA_APP_VERSION_CODE)).strong().size(12.5));
                                                    if !policy.update_url.is_empty() {
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.button(RichText::new("🌐 前往下载更新").strong().color(Color32::from_rgb(0, 120, 215))).clicked() {
                                                                let _ = open::that(&policy.update_url);
                                                            }
                                                        });
                                                    }
                                                });
                                            });
                                        ui.add_space(8.0);
                                    }
                                }

                                // 顶部通知条提示 (紧凑 Fluent 2 InfoBar 结构，平滑进出过渡动效，杜绝生硬抽动)
                                if let Some((msg, kind)) = self.state.notification.clone() {
                                    let is_dark = self.state.theme_config.theme.is_dark();

                                    // 动效插值因子计算 (0.0 ~ 1.0)
                                    let anim_factor = if self.state.notification_dismissing {
                                        let start = self.state.notification_dismiss_start.unwrap_or(current_time);
                                        let elapsed = (current_time - start) as f32;
                                        let duration = 0.22;
                                        if elapsed >= duration {
                                            self.state.notification = None;
                                            self.state.last_notification_text = None;
                                            self.state.notification_created_at = None;
                                            self.state.notification_expires_at = None;
                                            self.state.notification_dismissing = false;
                                            self.state.notification_dismiss_start = None;
                                            0.0
                                        } else {
                                            ui.ctx().request_repaint();
                                            let t = (elapsed / duration).clamp(0.0, 1.0);
                                            let inv = 1.0 - t;
                                            inv * inv
                                        }
                                    } else {
                                        let created = self.state.notification_created_at.unwrap_or(current_time);
                                        let elapsed = (current_time - created) as f32;
                                        let duration = 0.18;
                                        if elapsed < duration {
                                            ui.ctx().request_repaint();
                                            (elapsed / duration).clamp(0.0, 1.0)
                                        } else {
                                            1.0
                                        }
                                    };

                                    if anim_factor > 0.001 {
                                        let (icon_str, title_str, base_icon, base_border, base_bg) = match kind {
                                            InfoBarKind::Success => (
                                                "✔",
                                                "操作成功",
                                                if is_dark { Color32::from_rgb(108, 203, 95) } else { Color32::from_rgb(15, 123, 15) },
                                                if is_dark { Color32::from_rgba_unmultiplied(46, 180, 80, 110) } else { Color32::from_rgba_unmultiplied(15, 123, 15, 90) },
                                                if is_dark { Color32::from_rgba_unmultiplied(28, 44, 32, 235) } else { Color32::from_rgb(223, 246, 221) },
                                            ),
                                            InfoBarKind::Warning => (
                                                "⚠",
                                                "系统提醒",
                                                if is_dark { Color32::from_rgb(252, 225, 0) } else { Color32::from_rgb(157, 93, 0) },
                                                if is_dark { Color32::from_rgba_unmultiplied(230, 180, 30, 110) } else { Color32::from_rgba_unmultiplied(157, 93, 0, 90) },
                                                if is_dark { Color32::from_rgba_unmultiplied(46, 42, 26, 235) } else { Color32::from_rgb(255, 244, 206) },
                                            ),
                                            InfoBarKind::Error => (
                                                "❌",
                                                "错误提示",
                                                if is_dark { Color32::from_rgb(255, 120, 120) } else { Color32::from_rgb(196, 43, 28) },
                                                if is_dark { Color32::from_rgba_unmultiplied(240, 70, 70, 130) } else { Color32::from_rgba_unmultiplied(196, 43, 28, 100) },
                                                if is_dark { Color32::from_rgba_unmultiplied(48, 28, 28, 235) } else { Color32::from_rgb(253, 231, 233) },
                                            ),
                                            InfoBarKind::Informational => (
                                                "ℹ",
                                                "系统通知",
                                                if is_dark { Color32::from_rgb(96, 205, 255) } else { Color32::from_rgb(0, 95, 184) },
                                                if is_dark { Color32::from_rgba_unmultiplied(0, 150, 255, 110) } else { Color32::from_rgba_unmultiplied(0, 95, 184, 90) },
                                                if is_dark { Color32::from_rgba_unmultiplied(30, 42, 54, 235) } else { Color32::from_rgb(240, 246, 254) },
                                            ),
                                        };

                                        let icon_color = Color32::from_rgba_unmultiplied(
                                            base_icon.r(), base_icon.g(), base_icon.b(),
                                            ((base_icon.a() as f32) * anim_factor) as u8,
                                        );
                                        let border_color = Color32::from_rgba_unmultiplied(
                                            base_border.r(), base_border.g(), base_border.b(),
                                            ((base_border.a() as f32) * anim_factor) as u8,
                                        );
                                        let bg_color = Color32::from_rgba_unmultiplied(
                                            base_bg.r(), base_bg.g(), base_bg.b(),
                                            ((base_bg.a() as f32) * anim_factor) as u8,
                                        );

                                        let mut close_clicked = false;
                                        let pad_y = (8.0 * anim_factor).round() as i8;
                                        let frame_resp = egui::Frame::new()
                                            .corner_radius(6.0)
                                            .stroke(egui::Stroke::new(1.0, border_color))
                                            .fill(bg_color)
                                            .inner_margin(egui::Margin {
                                                left: 14,
                                                right: 12,
                                                top: pad_y.max(2),
                                                bottom: pad_y.max(2),
                                            })
                                            .show(ui, |ui| {
                                                // Fluent 2 紧凑单列/流式结构：左图标，中垂直图文（紧凑对齐），右关闭按钮
                                                ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 10.0;
                                                    ui.label(RichText::new(icon_str).size(15.0).color(icon_color).strong());

                                                    ui.vertical(|ui| {
                                                        ui.spacing_mut().item_spacing.y = 2.0;
                                                        ui.horizontal_wrapped(|ui| {
                                                            ui.spacing_mut().item_spacing.x = 8.0;
                                                            ui.label(RichText::new(title_str).strong().size(13.0).color(icon_color));
                                                            ui.add(
                                                                egui::Label::new(
                                                                    RichText::new(&msg)
                                                                        .size(12.5)
                                                                        .color(if is_dark {
                                                                            Color32::from_rgba_unmultiplied(225, 225, 225, (225.0 * anim_factor) as u8)
                                                                        } else {
                                                                            Color32::from_rgba_unmultiplied(35, 35, 35, (225.0 * anim_factor) as u8)
                                                                        }),
                                                                )
                                                                .wrap(),
                                                            );
                                                        });
                                                    });

                                                    ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                                                        let close_color = if is_dark {
                                                            Color32::from_rgba_unmultiplied(180, 180, 180, (180.0 * anim_factor) as u8)
                                                        } else {
                                                            Color32::from_rgba_unmultiplied(90, 90, 90, (180.0 * anim_factor) as u8)
                                                        };
                                                        let close_btn = ui.add(
                                                            egui::Button::new(
                                                                RichText::new("\u{00D7}")
                                                                    .size(16.0)
                                                                    .color(close_color),
                                                            )
                                                            .frame(false),
                                                        );
                                                        if close_btn.hovered() {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                        }
                                                        if close_btn.clicked() {
                                                            close_clicked = true;
                                                        }
                                                    });
                                                });
                                            });

                                        if frame_resp.response.hovered() && !self.state.notification_dismissing {
                                            // 鼠标悬停顺延 6 秒超时倒计时，绝对不修改进场动画基准时间 notification_created_at
                                            self.state.notification_expires_at = Some(current_time + 6.0);
                                        }

                                        if close_clicked && !self.state.notification_dismissing {
                                            self.state.notification_dismissing = true;
                                            self.state.notification_dismiss_start = Some(current_time);
                                        }
                                        ui.add_space((10.0 * anim_factor).round());
                                    }
                                }

                                // 页面切换平滑过渡动效 (Fluent PageTransition: 20px 向上缓入 + 0.24s 立方缓出淡入)
                                let cur_tab = self.state.current_tab;
                                let cur_hw = self.state.selected_homework.is_some();
                                let cur_mode = self.state.answer_view_mode;
                                if self.state.last_nav_tab != Some(cur_tab)
                                    || self.state.last_selected_hw != cur_hw
                                    || self.state.last_answer_view_mode != cur_mode
                                {
                                    self.state.last_nav_tab = Some(cur_tab);
                                    self.state.last_selected_hw = cur_hw;
                                    self.state.last_answer_view_mode = cur_mode;
                                    self.state.nav_revision = self.state.nav_revision.wrapping_add(1);
                                }

                                if self.state.theme_config.animations {
                                    fluent_egui::winui::PageTransition::new("main_nav_page", self.state.nav_revision)
                                        .distance(20.0)
                                        .duration(0.24)
                                        .easing(fluent_egui::winui::EasingFunction::CubicOut)
                                        .show(ui, |ui| {
                                            let mut fluent_main = FluentUi::from_ui(ui);
                                            if self.state.selected_homework.is_some() {
                                                answer_view::render(&mut fluent_main, &mut self.state);
                                            } else {
                                                match self.state.current_tab {
                                                    NavTab::HomeworkList => {
                                                        homework_list::render(&mut fluent_main, &mut self.state)
                                                    }
                                                    NavTab::LocalScan => {
                                                        local_scan::render(&mut fluent_main, &mut self.state)
                                                    }
                                                    NavTab::AutoComplete => {
                                                        auto_complete_view::render(&mut fluent_main, &mut self.state)
                                                    }
                                                    NavTab::CloudAccount => {
                                                        cloud_account::render(&mut fluent_main, &mut self.state)
                                                    }
                                                    NavTab::Settings => {
                                                        settings::render(&mut fluent_main, &mut self.state)
                                                    }
                                                    NavTab::Credits => {
                                                        credits::render(&mut fluent_main, &mut self.state)
                                                    }
                                                }
                                            }
                                        });
                                } else {
                                    let mut fluent_main = FluentUi::from_ui(ui);
                                    if self.state.selected_homework.is_some() {
                                        answer_view::render(&mut fluent_main, &mut self.state);
                                    } else {
                                        match self.state.current_tab {
                                            NavTab::HomeworkList => {
                                                homework_list::render(&mut fluent_main, &mut self.state)
                                            }
                                            NavTab::LocalScan => {
                                                local_scan::render(&mut fluent_main, &mut self.state)
                                            }
                                            NavTab::AutoComplete => {
                                                auto_complete_view::render(&mut fluent_main, &mut self.state)
                                            }
                                            NavTab::CloudAccount => {
                                                cloud_account::render(&mut fluent_main, &mut self.state)
                                            }
                                            NavTab::Settings => {
                                                settings::render(&mut fluent_main, &mut self.state)
                                            }
                                            NavTab::Credits => {
                                                credits::render(&mut fluent_main, &mut self.state)
                                            }
                                        }
                                    }
                                }
                            });
                    });
            });

        // 渲染全局 WinUI 风格权限拦截与会员特权引导模态对话框
        crate::views::render_auth_gate_modal(ui.ctx(), &mut self.state);

        // 渲染全局 WinUI 风格用户服务协议与免责声明 (EULA) 模态对话框
        crate::views::render_eula_modal(ui.ctx(), &mut self.state);
    }
}
