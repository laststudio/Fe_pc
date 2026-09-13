use eframe::egui::{self, CentralPanel, Color32, Frame, Margin, Panel, RichText, ScrollArea};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::{self, NavigationView, NavigationViewItem};

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
}

impl FepcApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut state = AppState::default();

        // 尝试加载已保存的云端会话
        if let Some(session) = CloudClient::load_saved_session() {
            state.session = session;
            // 静默加载作业列表
            if let Ok(hw_list) = CloudClient::get_homework_list(&state.session, "1") {
                state.homework_items = hw_list;
            }
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

        if std::env::var("FEPC_AUTO_OPEN_ANSWER").is_ok() {
            if let Some(first) = state.local_items.first() {
                state.selected_homework = Some(first.clone());
                state.answer_view_mode = crate::state::AnswerViewMode::Detailed;
            }
        }

        if std::env::var("FEPC_AUTO_OPEN_CONCISE").is_ok() {
            if let Some(first) = state.local_items.first() {
                state.selected_homework = Some(first.clone());
                state.answer_view_mode = crate::state::AnswerViewMode::Concise;
            }
        }

        if std::env::var("FEPC_TEST_LIGHT_THEME").is_ok() {
            state.theme_config.theme = fluent_egui::winui::FluentTheme::Light;
        }

        Self {
            state,
            selected_nav_index,
            logo_texture: None,
        }
    }
}

impl eframe::App for FepcApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // 1. 同步主题配置与窗口材质
        winui::apply_theme(ui.ctx(), self.state.theme_config);
        if let Some(window) = frame.winit_window() {
            let _ = winui::sync_window_backdrop(
                window,
                self.state.theme_config.theme,
                self.state.backdrop_kind,
            );
        }

        if self.logo_texture.is_none() {
            if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../fe_logo.png")) {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                );
                self.logo_texture = Some(ui.ctx().load_texture("app_logo", color_image, egui::TextureOptions::LINEAR));
            }
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
                    .show(&mut *fluent_nav);

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
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let avail_w = ui.available_width();
                        let pad_x = if avail_w > 960.0 {
                            (((avail_w - 960.0) / 2.0) as i8).min(100) + 24
                        } else {
                            24
                        };

                        Frame::new()
                            .inner_margin(Margin {
                                left: pad_x,
                                right: pad_x,
                                top: 20,
                                bottom: 36,
                            })
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                let mut fluent_main = FluentUi::from_ui(ui);

                                // 顶部通知条提示
                                if let Some((msg, kind)) = &self.state.notification {
                                    fluent_egui::winui::InfoBar::new("系统通知", msg)
                                        .kind(*kind)
                                        .show(&mut *fluent_main);
                                    fluent_main.add_space(12.0);
                                }

                                // 如果选中了具体作业，展示题目与答案详情；否则展示当前导航页
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
                    });
            });
    }
}
