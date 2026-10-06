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
    pub local_device_code: Option<String>,
    pub local_accounts: Vec<crate::models::LocalAccountInfo>,
    pub local_scanning: bool,

    // 主题、外观与页面转场动效
    pub theme_config: ThemeConfig,
    pub backdrop_kind: BackdropKind,
    pub nav_revision: u64,
    pub last_nav_tab: Option<NavTab>,
    pub last_selected_hw: bool,
    pub last_answer_view_mode: AnswerViewMode,

    // 通知与弹窗
    pub notification: Option<(String, InfoBarKind)>,
    pub last_notification_text: Option<String>,
    pub notification_created_at: Option<f64>,
    pub notification_expires_at: Option<f64>,
    pub notification_dismissing: bool,
    pub notification_dismiss_start: Option<f64>,
    pub login_password: String,
    pub rebind_dialog_open: bool,
    pub rebind_code_input: String,

    // LA Studio 账号系统与策略门禁
    pub software_policy: Option<crate::la_client::LaSoftwarePolicy>,
    pub la_session: Option<LaSession>,
    pub la_cloud_session: Option<LaCloudSession>,
    pub la_devices: Vec<LaDevice>,
    pub la_account_sessions: Vec<crate::la_client::LaAccountSession>,
    pub change_password_dialog_open: bool,
    pub change_password_old_input: String,
    pub change_password_new_input: String,
    pub change_password_confirm_input: String,
    pub change_password_loading: bool,
    pub la_username_input: String,
    pub la_password_input: String,
    pub la_bind_phone_input: String,
    pub la_bind_password_input: String,
    pub changyan_bind_dialog_open: bool,
    pub changyan_uid_input: String,
    pub changyan_token_input: String,
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
    pub auto_questions_done: Vec<(String, String, f64, f64, String, Option<String>)>,
    pub auto_rx: Option<std::sync::mpsc::Receiver<crate::auto_complete::AutoProgressMsg>>,

    // LA 云端声音资产漫游
    pub la_voices: Vec<crate::la_client::VoiceProfile>,
    pub la_voices_loading: bool,

    // 多提供商 TTS 矩阵
    pub tts_settings: crate::workflow::tts::TtsMatrixSettings,
    pub tts_testing: bool,
    pub tts_test_result: Option<String>,

    // 动态功能特性与鉴权门控
    pub features: std::collections::HashMap<String, crate::la_client::LaFeature>,
    pub features_loaded: bool,
    // 权限拦截弹窗状态
    pub auth_gate_modal_open: bool,
    pub auth_gate_modal_title: String,
    pub auth_gate_modal_message: String,
    pub auth_gate_modal_feature_name: String,
    pub auth_gate_modal_requires_login: bool,
    pub auth_gate_modal_required_tier: String,

    // 最终用户许可协议 (EULA) 与首次启动门禁
    pub eula_accepted: bool,
    pub eula_modal_open: bool,
    pub eula_checkbox_checked: bool,
    pub eula_is_first_launch: bool,

    // 全局与版本专属公告
    pub announcement_modal_open: bool,
    pub announcement_banner_dismissed: bool,
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
            local_device_code: None,
            local_accounts: Vec::new(),
            local_scanning: false,
            theme_config: ThemeConfig::new(FluentTheme::Dark).with_animations(true).with_material_opacity(1.0),
            backdrop_kind: BackdropKind::None,
            nav_revision: 1,
            last_nav_tab: Some(NavTab::HomeworkList),
            last_selected_hw: false,
            last_answer_view_mode: AnswerViewMode::Detailed,
            notification: None,
            last_notification_text: None,
            notification_created_at: None,
            notification_expires_at: None,
            notification_dismissing: false,
            notification_dismiss_start: None,
            login_password: String::new(),
            rebind_dialog_open: false,
            rebind_code_input: String::new(),

            // LA Studio
            software_policy: None,
            la_session: None,
            la_cloud_session: None,
            la_devices: Vec::new(),
            la_account_sessions: Vec::new(),
            change_password_dialog_open: false,
            change_password_old_input: String::new(),
            change_password_new_input: String::new(),
            change_password_confirm_input: String::new(),
            change_password_loading: false,
            la_username_input: String::new(),
            la_password_input: String::new(),
            la_bind_phone_input: String::new(),
            la_bind_password_input: String::new(),
            changyan_bind_dialog_open: false,
            changyan_uid_input: String::new(),
            changyan_token_input: String::new(),
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

            // LA 云端声音资产漫游
            la_voices: Vec::new(),
            la_voices_loading: false,

            // 多提供商 TTS 矩阵
            tts_settings: crate::workflow::tts::TtsMatrixSettings::load(),
            tts_testing: false,
            tts_test_result: None,

            // 动态功能特性与鉴权门控
            features: std::collections::HashMap::new(),
            features_loaded: false,
            auth_gate_modal_open: false,
            auth_gate_modal_title: String::new(),
            auth_gate_modal_message: String::new(),
            auth_gate_modal_feature_name: String::new(),
            auth_gate_modal_requires_login: false,
            auth_gate_modal_required_tier: String::new(),
            eula_accepted: false,
            eula_modal_open: false,
            eula_checkbox_checked: false,
            eula_is_first_launch: false,

            // 全局与版本专属公告
            announcement_modal_open: false,
            announcement_banner_dismissed: false,
        }
    }
}

impl AppState {
    /// 刷新当前选中本地目录的静态元数据 (仅在目录变更/初次加载时调用 1 次，严禁在 UI 渲染死循环中调用)
    pub fn refresh_selected_local_metadata(&mut self) {
        if let Some(ref dir) = self.selected_local_dir {
            self.local_device_code = crate::local::LocalScanner::read_local_device_code(dir);
            self.local_accounts = crate::local::LocalScanner::read_local_accounts(dir);
        } else {
            self.local_device_code = None;
            self.local_accounts.clear();
        }
    }

    /// 确保指定作业已被全量解析 (用于本地试卷 Lazy Resolver 按需懒解析)
    pub fn ensure_local_homework_parsed(&mut self, hw_id: &str) -> Option<&crate::models::HomeworkItem> {
        if let Some(pos) = self.local_items.iter().position(|h| h.id == hw_id) {
            if !self.local_items[pos].is_parsed() {
                crate::local::LocalScanner::ensure_homework_parsed(&mut self.local_items[pos]);
            }
            return Some(&self.local_items[pos]);
        }
        None
    }

    /// 异步上报功能调用统计 (后台线程无阻塞)
    pub fn report_feature_usage(&self, feature_key: &str) {
        let token_opt = self.la_session.as_ref().map(|s| s.access_token.clone())
            .or_else(|| crate::la_client::LaClient::load_saved_session().map(|s| s.access_token));
        if let Some(token) = token_opt {
            let key = feature_key.to_string();
            std::thread::spawn(move || {
                let _ = crate::la_client::LaClient::authorize_feature(&token, &key);
            });
        }
    }

    /// 动态特性安全鉴权判定引擎
    pub fn check_feature(&self, feature_key: &str) -> crate::la_client::FeatureCheckResult {
        self.evaluate_feature_permission(feature_key)
    }

    #[inline(never)]
    fn evaluate_feature_permission(&self, feature_key: &str) -> crate::la_client::FeatureCheckResult {
        let is_admin = self.la_session.as_ref().is_some_and(|s| s.user.role <= 1);
        if is_admin {
            return crate::la_client::FeatureCheckResult {
                allowed: true,
                reason: "管理员特权放行".to_string(),
                required_tier: "ultra".to_string(),
                requires_login: false,
                requires_membership: false,
            };
        }

        let is_logged_in = self.la_session.is_some();
        let is_member = self.la_session.as_ref().map(|s| s.user.is_member()).unwrap_or(false);
        let user_member_level = self.la_session.as_ref().map(|s| s.user.member_level).unwrap_or(0);

        let feature_opt = if self.features_loaded || !self.features.is_empty() {
            self.features.get(feature_key)
        } else {
            None
        };

        let (has_feat, is_enabled, req_login, req_membership, min_member_level, authorized_flag, feat_reason, feat_tier) = match feature_opt {
            Some(feat) => {
                let auth_flag: i8 = match feat.authorized {
                    Some(true) => 1,
                    Some(false) => -1,
                    None => 0,
                };
                (
                    true,
                    feat.status == "enabled",
                    feat.requires_login || feat.requires_membership,
                    feat.requires_membership,
                    feat.min_member_level,
                    auth_flag,
                    feat.reason.clone().or_else(|| feat.deny_message.clone()),
                    feat.min_member_tier.clone().unwrap_or_default(),
                )
            }
            None => (false, false, false, false, 0, 0, None, String::new()),
        };

        let is_default_free = matches!(feature_key, "answer_parser" | "cloud_mode" | "copy_export");

        let decision_code: u8 = if has_feat {
            if !is_enabled {
                1
            } else if req_login && !is_logged_in {
                2
            } else if authorized_flag == -1 || (req_membership && user_member_level < min_member_level && authorized_flag != 1) {
                3
            } else {
                0
            }
        } else if is_default_free {
            0
        } else if !is_logged_in {
            2
        } else if !is_member {
            3
        } else {
            0
        };

        match decision_code {
            0 => crate::la_client::FeatureCheckResult {
                allowed: true,
                reason: if has_feat {
                    "已授权".to_string()
                } else if is_member {
                    "会员特权放行".to_string()
                } else {
                    "离线安全放行".to_string()
                },
                required_tier: if !feat_tier.is_empty() { feat_tier } else { "pro".to_string() },
                requires_login: false,
                requires_membership: false,
            },
            1 => crate::la_client::FeatureCheckResult {
                allowed: false,
                reason: feat_reason.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "该功能维护中".to_string()),
                required_tier: feat_tier,
                requires_login: false,
                requires_membership: false,
            },
            2 => crate::la_client::FeatureCheckResult {
                allowed: false,
                reason: "该功能需要登录后使用".to_string(),
                required_tier: if !feat_tier.is_empty() { feat_tier } else { "pro".to_string() },
                requires_login: true,
                requires_membership: req_membership || !is_default_free,
            },
            3 => {
                let default_tier = if !feat_tier.is_empty() {
                    feat_tier.clone()
                } else {
                    "pro".to_string()
                };
                let reason = feat_reason.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
                    if default_tier.eq_ignore_ascii_case("ultra") {
                        "该功能需要开通 Ultra 会员后使用".to_string()
                    } else {
                        "该功能需要开通 Pro 会员后使用".to_string()
                    }
                });
                crate::la_client::FeatureCheckResult {
                    allowed: false,
                    reason,
                    required_tier: default_tier,
                    requires_login: false,
                    requires_membership: true,
                }
            }
            _ => crate::la_client::FeatureCheckResult {
                allowed: false,
                reason: "权限校验未通过".to_string(),
                required_tier: feat_tier,
                requires_login: false,
                requires_membership: false,
            },
        }
    }

    /// 触发权限拦截模态弹窗
    pub fn trigger_auth_gate_modal(&mut self, feature_key: &str, feature_name: &str) {
        let check = self.check_feature(feature_key);
        let display_name = if !feature_name.trim().is_empty() {
            feature_name.to_string()
        } else if let Some(feat) = self.features.get(feature_key) {
            feat.name.clone()
        } else {
            match feature_key {
                "answer_parser" => "解析核心".to_string(),
                "cloud_mode" => "云端代登".to_string(),
                "copy_export" => "格式化导出".to_string(),
                "auto_complete" => "全自动模拟做题".to_string(),
                "voice_clone" => "声音复刻".to_string(),
                _ => feature_key.to_string(),
            }
        };

        self.auth_gate_modal_open = true;
        self.auth_gate_modal_feature_name = display_name.clone();
        self.auth_gate_modal_requires_login = check.requires_login;
        self.auth_gate_modal_required_tier = check.required_tier.clone();

        if check.requires_login {
            self.auth_gate_modal_title = format!("{display_name} 需要登录");
            self.auth_gate_modal_message = if check.reason.is_empty() {
                "该功能需要登录后使用，请先登录 LA Studio 账号。".to_string()
            } else {
                check.reason
            };
        } else {
            self.auth_gate_modal_title = format!("{display_name} 需要会员权限");
            self.auth_gate_modal_message = if check.reason.is_empty() {
                "该功能需要开通会员权限后使用。".to_string()
            } else {
                check.reason
            };
        }
    }

    /// 关闭权限拦截模态弹窗
    pub fn close_auth_gate_modal(&mut self) {
        self.auth_gate_modal_open = false;
    }

    /// 打开 EULA 协议模态弹窗
    pub fn open_eula_modal(&mut self, is_first_launch: bool) {
        self.eula_modal_open = true;
        self.eula_is_first_launch = is_first_launch;
        self.eula_checkbox_checked = self.eula_accepted;
    }

    /// 关闭 EULA 协议模态弹窗
    pub fn close_eula_modal(&mut self) {
        self.eula_modal_open = false;
        self.eula_is_first_launch = false;
    }

    /// 用户同意 EULA 协议并持久化落盘
    pub fn accept_eula(&mut self) {
        let _ = crate::eula::save_eula_accepted();
        self.eula_accepted = true;
        self.eula_modal_open = false;
        self.eula_is_first_launch = false;
        self.eula_checkbox_checked = true;
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::la_client::{LaFeature, LaSession, LaUser};

    fn make_session(level: i32, tier: &str) -> LaSession {
        LaSession {
            session_id: "test_session_id".to_string(),
            device_id: Some("dev_123".to_string()),
            user: LaUser {
                id: 1001,
                username: "test_user".to_string(),
                member_level: level,
                tier: tier.to_string(),
                avatar_url: None,
                role: 3,
                role_display: None,
                member_expire_time: None,
            },
            access_token: "mock_token".to_string(),
            refresh_token: "mock_refresh".to_string(),
            cached_at: 100000,
        }
    }

    #[test]
    fn test_offline_default_feature_permissions() {
        let mut state = AppState::default();
        state.features_loaded = false;

        // 1. 未登录场景
        state.la_session = None;
        assert!(state.check_feature("answer_parser").allowed, "离线默认放行 answer_parser");
        assert!(state.check_feature("cloud_mode").allowed, "离线默认放行 cloud_mode");
        assert!(state.check_feature("copy_export").allowed, "离线默认放行 copy_export");

        let ac_unauth = state.check_feature("auto_complete");
        assert!(!ac_unauth.allowed, "未登录禁止使用 auto_complete");
        assert!(ac_unauth.requires_login);

        let vc_unauth = state.check_feature("voice_clone");
        assert!(!vc_unauth.allowed, "未登录禁止使用 voice_clone");
        assert!(vc_unauth.requires_login);

        // 2. 免费登录用户 (member_level: 0, tier: "free")
        state.la_session = Some(make_session(0, "free"));
        assert!(state.check_feature("answer_parser").allowed);
        let ac_free = state.check_feature("auto_complete");
        assert!(!ac_free.allowed, "免费用户禁止使用 auto_complete");
        assert!(!ac_free.requires_login);
        assert!(ac_free.requires_membership);

        let vc_free = state.check_feature("voice_clone");
        assert!(!vc_free.allowed, "免费用户禁止使用 voice_clone");
        assert!(vc_free.requires_membership);

        // 3. Pro 会员用户 (member_level: 1, tier: "pro")
        state.la_session = Some(make_session(1, "pro"));
        assert!(state.check_feature("answer_parser").allowed);
        assert!(state.check_feature("auto_complete").allowed, "Pro 用户允许使用 auto_complete");
        assert!(state.check_feature("voice_clone").allowed, "Pro 用户允许使用 voice_clone");

        // 4. 管理员用户 (role <= 1): 直接放行所有功能
        let mut admin_sess = make_session(0, "free");
        admin_sess.user.role = 0; // 超级管理员
        state.la_session = Some(admin_sess);
        assert!(state.check_feature("auto_complete").allowed, "超级管理员直接放行 auto_complete");
        assert!(state.check_feature("voice_clone").allowed, "超级管理员直接放行 voice_clone");

        let mut admin1_sess = make_session(0, "free");
        admin1_sess.user.role = 1; // 管理员
        state.la_session = Some(admin1_sess);
        assert!(state.check_feature("auto_complete").allowed, "管理员直接放行 auto_complete");
        assert!(state.check_feature("voice_clone").allowed, "管理员直接放行 voice_clone");
    }

    #[test]
    fn test_dynamic_feature_gate_scenarios() {
        let mut state = AppState::default();
        state.features_loaded = true;

        // 特性 1: 维护中的功能 (不管登录和权限，统一阻断)
        state.features.insert(
            "tts_clone".to_string(),
            LaFeature {
                feature_key: "tts_clone".to_string(),
                name: "声音复刻".to_string(),
                description: None,
                status: "disabled".to_string(),
                authorized: Some(true),
                available_for_version: Some(true),
                reason: Some("服务维护升级中".to_string()),
                requires_login: false,
                requires_membership: false,
                min_member_level: 0,
                min_member_tier: None,
                deny_message: None,
            },
        );

        // 即使是 Pro 用户也拦截
        state.la_session = Some(make_session(1, "pro"));
        let disabled_res = state.check_feature("tts_clone");
        assert!(!disabled_res.allowed);
        assert_eq!(disabled_res.reason, "服务维护升级中");

        // 特性 2: 需要登录的功能
        state.features.insert(
            "cloud_sync".to_string(),
            LaFeature {
                feature_key: "cloud_sync".to_string(),
                name: "云端同步".to_string(),
                description: None,
                status: "enabled".to_string(),
                authorized: None,
                available_for_version: Some(true),
                reason: None,
                requires_login: true,
                requires_membership: false,
                min_member_level: 0,
                min_member_tier: None,
                deny_message: None,
            },
        );

        state.la_session = None;
        let sync_unauth = state.check_feature("cloud_sync");
        assert!(!sync_unauth.allowed);
        assert!(sync_unauth.requires_login);
        assert_eq!(sync_unauth.reason, "该功能需要登录后使用");

        state.la_session = Some(make_session(0, "free"));
        let sync_auth = state.check_feature("cloud_sync");
        assert!(sync_auth.allowed);

        // 特性 3: 需要会员的功能 (min_member_level: 1, min_member_tier: "pro")
        state.features.insert(
            "auto_pipeline".to_string(),
            LaFeature {
                feature_key: "auto_pipeline".to_string(),
                name: "自动化流水线".to_string(),
                description: None,
                status: "enabled".to_string(),
                authorized: None,
                available_for_version: Some(true),
                reason: None,
                requires_login: true,
                requires_membership: true,
                min_member_level: 1,
                min_member_tier: Some("pro".to_string()),
                deny_message: Some("需要开通专业版会员".to_string()),
            },
        );

        // 未登录
        state.la_session = None;
        let ap_unauth = state.check_feature("auto_pipeline");
        assert!(!ap_unauth.allowed);
        assert!(ap_unauth.requires_login);

        // 免费用户
        state.la_session = Some(make_session(0, "free"));
        let ap_free = state.check_feature("auto_pipeline");
        assert!(!ap_free.allowed);
        assert!(!ap_free.requires_login);
        assert!(ap_free.requires_membership);
        assert_eq!(ap_free.reason, "需要开通专业版会员");

        // Pro 用户
        state.la_session = Some(make_session(1, "pro"));
        let ap_pro = state.check_feature("auto_pipeline");
        assert!(ap_pro.allowed);

        // 特性 4: 服务端明确返回 authorized: Some(false)
        state.features.insert(
            "exclusive_feature".to_string(),
            LaFeature {
                feature_key: "exclusive_feature".to_string(),
                name: "专属特权".to_string(),
                description: None,
                status: "enabled".to_string(),
                authorized: Some(false),
                available_for_version: Some(true),
                reason: None,
                requires_login: true,
                requires_membership: true,
                min_member_level: 1,
                min_member_tier: Some("pro".to_string()),
                deny_message: Some("您的账号特权已被单独禁用".to_string()),
            },
        );

        let excl_res = state.check_feature("exclusive_feature");
        assert!(!excl_res.allowed);
        assert_eq!(excl_res.reason, "您的账号特权已被单独禁用");
    }

    #[test]
    fn test_trigger_and_close_auth_gate_modal() {
        let mut state = AppState::default();
        state.features_loaded = false;
        state.la_session = None;

        assert!(!state.auth_gate_modal_open);

        // 触发未登录拦截
        state.trigger_auth_gate_modal("auto_complete", "自动做题");
        assert!(state.auth_gate_modal_open);
        assert!(state.auth_gate_modal_requires_login);
        assert_eq!(state.auth_gate_modal_feature_name, "自动做题");
        assert!(state.auth_gate_modal_title.contains("需要登录"));

        // 关闭弹窗
        state.close_auth_gate_modal();
        assert!(!state.auth_gate_modal_open);

        // 登录为免费用户后触发会员不足拦截
        state.la_session = Some(make_session(0, "free"));
        state.trigger_auth_gate_modal("auto_complete", "自动做题");
        assert!(state.auth_gate_modal_open);
        assert!(!state.auth_gate_modal_requires_login);
        assert!(state.auth_gate_modal_title.contains("会员"));
    }
}


