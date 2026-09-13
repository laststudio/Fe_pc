use std::path::PathBuf;

use fluent_egui::winui::{BackdropKind, FluentTheme, InfoBarKind, ThemeConfig};

use crate::auto_complete::AutoCompleteConfig;
use crate::models::{HomeworkItem, UserSession};

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
    pub local_ets_dirs: Vec<PathBuf>,
    pub selected_local_dir: Option<PathBuf>,

    // 主题与外观
    pub theme_config: ThemeConfig,
    pub backdrop_kind: BackdropKind,

    // 通知与弹窗
    pub notification: Option<(String, InfoBarKind)>,
    pub login_password: String,
    pub rebind_dialog_open: bool,
    pub rebind_code_input: String,

    // 自动做题流水线状态 (Scheme B)
    pub auto_config: AutoCompleteConfig,
    pub auto_selected_hw_id: Option<String>,
    pub auto_running: bool,
    pub auto_progress: Option<(usize, usize, String)>,
    pub auto_logs: Vec<String>,
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
            local_ets_dirs: Vec::new(),
            selected_local_dir: None,
            theme_config: ThemeConfig::new(FluentTheme::Dark).with_animations(true).with_material_opacity(1.0),
            backdrop_kind: BackdropKind::None,
            notification: None,
            login_password: String::new(),
            rebind_dialog_open: false,
            rebind_code_input: String::new(),
            auto_config: AutoCompleteConfig::default(),
            auto_selected_hw_id: None,
            auto_running: false,
            auto_progress: None,
            auto_logs: Vec::new(),
            auto_rx: None,
        }
    }
}
