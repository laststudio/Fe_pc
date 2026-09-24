use std::path::PathBuf;

use fluent_egui::winui::{BackdropKind, FluentTheme, InfoBarKind, ThemeConfig};

use crate::auto_complete::AutoCompleteConfig;
use crate::la_client::{LaCloudSession, LaDevice, LaSession};
use crate::models::{HomeworkCategory, HomeworkItem, UserSession};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavTab {
    HomeworkList,
    LocalScan,
    AutoComplete,
    CloudAccount,
    Settings,
    Credits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerViewMode {
    Detailed,
    Concise,
}

pub struct AppState {
    pub current_tab: NavTab,
    pub session: UserSession,
    pub homework_items: Vec<HomeworkItem>,
    pub local_items: Vec<HomeworkItem>,
    pub selected_homework: Option<HomeworkItem>,
    pub answer_view_mode: AnswerViewMode,
    pub search_filter: String,
    pub selected_status_filter: String, // "all", "pending", "completed"
    pub selected_category: HomeworkCategory,
    pub local_ets_dirs: Vec<PathBuf>,
    pub selected_local_dir: Option<PathBuf>,

    // 主题、外观与页面转场动效
    pub theme_config: ThemeConfig,
    pub backdrop_kind: BackdropKind,
    pub nav_revision: u64,
    pub last_nav_tab: Option<NavTab>,
    pub last_selected_hw: bool,
    pub last_answer_view_mode: AnswerViewMode,

    // 通知与弹窗
    pub notification: Option<(String, InfoBarKind)>,
    pub login_password: String,
    pub rebind_dialog_open: bool,
    pub rebind_code_input: String,

    // LA Studio 账号系统
    pub la_session: Option<LaSession>,
    pub la_cloud_session: Option<LaCloudSession>,
    pub la_devices: Vec<LaDevice>,
    pub la_username_input: String,
    pub la_password_input: String,
    pub la_bind_phone_input: String,
    pub la_bind_password_input: String,
    pub la_captcha_needed: bool,
    pub la_captcha_token: String,
    pub captcha_server: Option<crate::utils::captcha_server::CaptchaServer>,
    pub la_loading: bool,
    pub la_active_tab: usize, // 0: LA 云端托管代登, 1: ETS 官方手机号直连

    // 自动做题流水线状态 (Scheme B)
    pub auto_config: AutoCompleteConfig,
    pub auto_selected_hw_id: Option<String>,
    pub auto_running: bool,
    pub auto_progress: Option<(usize, usize, String)>,
    pub auto_logs: Vec<String>,
    pub auto_questions_done: Vec<(String, String, f64, f64, String)>,
    pub auto_rx: Option<std::sync::mpsc::Receiver<crate::auto_complete::AutoProgressMsg>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_tab: NavTab::HomeworkList,
            session: UserSession::default(),
            homework_items: Vec::new(),
            local_items: Vec::new(),
            selected_homework: None,
            answer_view_mode: AnswerViewMode::Detailed,
            search_filter: String::new(),
            selected_status_filter: "all".to_string(),
            selected_category: HomeworkCategory::Current,
            local_ets_dirs: Vec::new(),
            selected_local_dir: None,
            theme_config: ThemeConfig::new(FluentTheme::Dark).with_animations(true).with_material_opacity(1.0),
            backdrop_kind: BackdropKind::None,
            nav_revision: 1,
            last_nav_tab: Some(NavTab::HomeworkList),
            last_selected_hw: false,
            last_answer_view_mode: AnswerViewMode::Detailed,
            notification: None,
            login_password: String::new(),
            rebind_dialog_open: false,
            rebind_code_input: String::new(),

            // LA Studio
            la_session: None,
            la_cloud_session: None,
            la_devices: Vec::new(),
            la_username_input: String::new(),
            la_password_input: String::new(),
            la_bind_phone_input: String::new(),
            la_bind_password_input: String::new(),
            la_captcha_needed: false,
            la_captcha_token: String::new(),
            captcha_server: None,
            la_loading: false,
            la_active_tab: 0,

            // 自动做题
            auto_config: AutoCompleteConfig::default(),
            auto_selected_hw_id: None,
            auto_running: false,
            auto_progress: None,
            auto_logs: Vec::new(),
            auto_questions_done: Vec::new(),
            auto_rx: None,
        }
    }
}
