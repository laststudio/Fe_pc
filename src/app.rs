use eframe::egui::{self, CentralPanel, Color32, Frame, Margin, Panel, RichText, ScrollArea};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::{self, InfoBarKind, NavigationView, NavigationViewItem};

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
        let mut state = AppState::default();
        let screenshot_target = std::env::var("FEPC_SCREENSHOT").ok();

        // 尝试加载已保存的 LA Studio 会话
        if let Some(la_sess) = crate::la_client::LaClient::load_saved_session() {
            state.la_session = Some(la_sess.clone());
            // 如果已保存 LA 会话，尝试恢复云端托管的 ETS100 会话
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
                ("1".to_string(), "听后选择".to_string(), 2.0, 2.0, "已完成".to_string()),
                ("2".to_string(), "短文朗读".to_string(), 3.8, 4.0, "已完成".to_string()),
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
        winui::apply_theme(ui.ctx(), self.state.theme_config);
        if let Some(window) = frame.winit_window() {
            let _ = winui::sync_window_backdrop(
                window,
                self.state.theme_config.theme,
                self.state.backdrop_kind,
            );
        }

        if self.logo_texture.is_none()
            && let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../fe_logo.png")) {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                );
                self.logo_texture = Some(ui.ctx().load_texture("app_logo", color_image, egui::TextureOptions::LINEAR));
            }

        let is_dark = self.state.theme_config.theme.is_dark();

        // 2. 纯色不透明背景色，彻底杜绝底色桌面图标透光 (Issue 2 修复)
        let window_bg = if is_dark {
            Color32::from_rgb(32, 32, 32)
        } else {
            Color32::from_rgb(243, 243, 243)
        };
        let sidebar_bg = if is_dark {
            Color32::from_rgb(25, 25, 25)
        } else {
            Color32::from_rgb(235, 235, 235)
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
                // 应用标题与 Logo (采用去除黑底的精细透明 fe_logo)
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.logo_texture {
                        ui.image((tex.id(), egui::vec2(36.0, 36.0)));
                        ui.add_space(6.0);
                    }
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new("Fe PC")
                                .strong()
                                .size(20.0)
                                .color(Color32::from_rgb(0, 180, 255)),
                        );
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
                        let pad_x: i8 = 24;

                        Frame::new()
                            .inner_margin(Margin {
                                left: pad_x,
                                right: pad_x,
                                top: 20,
                                bottom: 36,
                            })
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());

                                // 顶部通知条提示 (带平滑滑入/折叠隐式动效)
                                let has_notif = self.state.notification.is_some();
                                fluent_egui::winui::ImplicitTransition::new("notification_bar", has_notif)
                                    .distance(12.0)
                                    .show(ui, |ui, _t| {
                                        if let Some((msg, kind)) = &self.state.notification {
                                            let mut fluent_n = FluentUi::from_ui(ui);
                                            fluent_egui::winui::InfoBar::new("系统通知", msg)
                                                .kind(*kind)
                                                .show(&mut fluent_n);
                                            fluent_n.add_space(12.0);
                                        }
                                    });

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
    }
}
