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

#[derive(Debug)]
pub enum AsyncWorkerEvent {
    LaLoginResult(Result<crate::la_client::LaSession, String>),
    CloudSessionSyncResult(
        Option<crate::models::UserSession>,
        Option<crate::la_client::LaCloudSession>,
        Option<Vec<crate::models::HomeworkItem>>,
        Option<Vec<crate::la_client::LaDevice>>,
    ),
    HomeworkRefreshResult(Result<Vec<crate::models::HomeworkItem>, String>),
    LocalScanResult(Vec<crate::models::HomeworkItem>),
}

#[derive(Debug)]
pub struct SecureWorkerMessage {
    pub payload: AsyncWorkerEvent,
    pub timestamp: u64,
    pub salt: u64,
    pub mask: u64,
}

impl SecureWorkerMessage {
    pub fn new(payload: AsyncWorkerEvent) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let salt = {
            let u = uuid::Uuid::new_v4();
            let mut b = [0u8; 8];
            b.copy_from_slice(&u.as_bytes()[0..8]);
            u64::from_le_bytes(b)
        };
        let mask = Self::calculate_mask(timestamp, salt);
        Self {
            payload,
            timestamp,
            salt,
            mask,
        }
    }

    #[inline(never)]
    pub fn calculate_mask(timestamp: u64, salt: u64) -> u64 {
        crate::themida_tiger_white!({
            let mut h = (timestamp ^ salt).wrapping_mul(0x9E3779B97F4A7C15);
            h = (h ^ (h >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            h = (h ^ (h >> 27)).wrapping_mul(0x94D049BB133111EB);
            h ^ (h >> 31) ^ salt.rotate_left(17)
        })
    }

    #[inline(never)]
    pub fn is_valid(&self) -> bool {
        self.mask == Self::calculate_mask(self.timestamp, self.salt)
    }
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
    pub homework_refreshing: bool,
    pub la_login_in_progress: bool,
    pub la_login_stage_text: String,

    // 本地 ETS.db 数据库与账号缓存
    pub ets_local_db_path: Option<String>,
    pub ets_local_accounts: Vec<crate::ets_local_db::EtsLocalAccount>,
    pub ets_local_scanning: bool,
    pub ets_local_scanned: bool,
    pub local_ets_answer_matched: bool,

    // 跨线程安全通信信道 (动态随机校验掩码与防篡改防护)
    pub async_worker_tx: std::sync::mpsc::Sender<SecureWorkerMessage>,
    pub async_worker_rx: std::sync::mpsc::Receiver<SecureWorkerMessage>,

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
        let (async_worker_tx, async_worker_rx) = std::sync::mpsc::channel();
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
            homework_refreshing: false,
            la_login_in_progress: false,
            la_login_stage_text: String::new(),
            ets_local_db_path: None,
            ets_local_accounts: Vec::new(),
            ets_local_scanning: false,
            ets_local_scanned: false,
            local_ets_answer_matched: false,
            async_worker_tx,
            async_worker_rx,
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
    /// 获取异步 Worker 消息发送器克隆
    pub fn worker_sender(&self) -> std::sync::mpsc::Sender<SecureWorkerMessage> {
        self.async_worker_tx.clone()
    }

    /// 向信道安全发送 Worker 事件
    pub fn send_worker_event(tx: &std::sync::mpsc::Sender<SecureWorkerMessage>, event: AsyncWorkerEvent) {
        let _ = tx.send(SecureWorkerMessage::new(event));
    }

    /// 轮询并消费跨线程安全异步 Worker 事件 (依赖 Tier 3: calculate_mask 保护校验)
    pub fn poll_async_events(&mut self) {
        let mut pending = Vec::new();
        while let Ok(msg) = self.async_worker_rx.try_recv() {
            pending.push(msg);
        }

        for msg in pending {
            if msg.is_valid() {
                self.dispatch_async_event(msg.payload);
            } else {
                eprintln!("[Security] 跨线程异步 Worker 事件随机掩码校验失败，防御性阻断潜在内存篡改注入！");
            }
        }
    }

    fn dispatch_async_event(&mut self, event: AsyncWorkerEvent) {
        match event {
            AsyncWorkerEvent::LaLoginResult(res) => {
                match res {
                    Ok(sess) => {
                        crate::la_client::sync_auth_to_resolver(&sess.access_token);
                        self.la_session = Some(sess);
                        self.la_password_input.clear();
                        self.la_captcha_needed = false;
                        self.la_captcha_token.clear();
                        self.la_login_stage_text = "正在同步云端托管会话与试卷...".to_string();
                    }
                    Err(e) => {
                        self.la_login_in_progress = false;
                        self.la_login_stage_text.clear();
                        if e.contains("人机验证") {
                            self.la_captcha_needed = true;
                            if self.captcha_server.is_none() {
                                if let Ok(srv) = crate::utils::captcha_server::CaptchaServer::start() {
                                    let port = srv.port();
                                    self.captcha_server = Some(srv);
                                    let _ = open::that(format!("http://127.0.0.1:{port}/"));
                                }
                            } else if let Some(srv) = &self.captcha_server {
                                let _ = open::that(format!("http://127.0.0.1:{}/", srv.port()));
                            }
                            self.notification = Some((
                                "需要完成人机安全验证，已为您在浏览器打开验证页面".to_string(),
                                InfoBarKind::Warning,
                            ));
                        } else {
                            self.notification = Some((
                                format!("登录失败: {e}"),
                                InfoBarKind::Error,
                            ));
                        }
                    }
                }
            }
            AsyncWorkerEvent::CloudSessionSyncResult(user_sess_opt, cloud_sess_opt, hws_opt, devs_opt) => {
                let was_logging_in = self.la_login_in_progress;
                self.la_login_in_progress = false;
                self.la_login_stage_text.clear();

                if let Some(user_sess) = user_sess_opt {
                    self.session = user_sess;
                }
                if let Some(cloud_sess) = cloud_sess_opt {
                    self.la_cloud_session = Some(cloud_sess);
                }
                if let Some(hws) = hws_opt {
                    self.homework_items = hws;
                }
                if let Some(devs) = devs_opt {
                    self.la_devices = devs;
                }

                if was_logging_in {
                    if self.session.is_logged_in {
                        self.notification = Some((
                            format!("LA 会员登录成功！已自动托管激活 ETS100 学生会话 ({})", self.session.user_name),
                            InfoBarKind::Success,
                        ));
                    } else if let Some(ref sess) = self.la_session {
                        self.notification = Some((
                            format!("欢迎回来，{}！", sess.user.username),
                            InfoBarKind::Success,
                        ));
                    }
                } else if self.session.is_logged_in {
                    self.notification = Some((
                        "云端做题会话激活成功！已同步最新作业".to_string(),
                        InfoBarKind::Success,
                    ));
                }
            }
            AsyncWorkerEvent::HomeworkRefreshResult(res) => {
                self.homework_refreshing = false;
                match res {
                    Ok(items) => {
                        let count = items.len();
                        if count == 0 && self.homework_items.is_empty() {
                            if let Some(db_path) = crate::ets_local_db::detect_db_path() {
                                if let Ok(cached) = crate::ets_local_db::read_cached_homeworks(&db_path) {
                                    if !cached.is_empty() {
                                        let c_count = cached.len();
                                        self.homework_items = cached;
                                        self.notification = Some((
                                            format!("📂 已自动加载本地离线作业列表 ({} 项)", c_count),
                                            InfoBarKind::Informational,
                                        ));
                                        return;
                                    }
                                }
                            }
                        }
                        self.homework_items = items;
                        let desc = match self.selected_category {
                            crate::models::HomeworkCategory::Current => "当前日常作业",
                            crate::models::HomeworkCategory::History => "历史归档作业",
                            crate::models::HomeworkCategory::VocabPk => "在线词汇PK竞技任务",
                            crate::models::HomeworkCategory::Local => "本地试卷",
                        };
                        self.notification = Some((
                            format!("成功获取到 {} 份{}", count, desc),
                            InfoBarKind::Success,
                        ));
                    }
                    Err(e) => {
                        if self.homework_items.is_empty() {
                            if let Some(db_path) = crate::ets_local_db::detect_db_path() {
                                if let Ok(cached) = crate::ets_local_db::read_cached_homeworks(&db_path) {
                                    if !cached.is_empty() {
                                        let c_count = cached.len();
                                        self.homework_items = cached;
                                        self.notification = Some((
                                            format!("📂 已自动加载本地离线作业列表 ({} 项)", c_count),
                                            InfoBarKind::Informational,
                                        ));
                                        return;
                                    }
                                }
                            }
                        }
                        self.notification = Some((
                            format!("获取作业失败: {}", e),
                            InfoBarKind::Error,
                        ));
                    }
                }
            }
            AsyncWorkerEvent::LocalScanResult(items) => {
                self.local_scanning = false;
                let count = items.len();
                self.local_items = items;
                self.notification = Some((
                    format!("扫描完成，发现 {} 份本地试卷 (已建立极速索引)", count),
                    InfoBarKind::Success,
                ));
            }
        }
    }

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

    /// 扫描/重新检测本地 ETS.db 数据库并载入账号列表
    pub fn scan_ets_local_db(&mut self) {
        self.ets_local_scanning = true;
        let db_path = if let Some(ref path_str) = self.ets_local_db_path {
            let p = PathBuf::from(path_str);
            if p.exists() {
                Some(p)
            } else {
                eprintln!("[ETS Local DB] 指定的数据库文件不存在: {}", p.display());
                None
            }
        } else {
            crate::ets_local_db::detect_db_path()
        };

        if let Some(path) = db_path {
            self.ets_local_db_path = Some(path.to_string_lossy().to_string());
            match crate::ets_local_db::read_local_accounts(&path) {
                Ok(accounts) => {
                    self.ets_local_accounts = accounts;
                }
                Err(e) => {
                    eprintln!("[ETS Local DB] 读取本地账号失败: {e}");
                    self.ets_local_accounts.clear();
                }
            }
        } else {
            self.ets_local_accounts.clear();
        }

        self.ets_local_scanning = false;
        self.ets_local_scanned = true;
    }

    /// 触发刷新云端作业列表（后台异步加载）
    pub fn refresh_cloud_homework(&mut self) {
        if !self.session.is_logged_in {
            return;
        }
        self.homework_refreshing = true;
        if let Some(ref la_sess) = self.la_session {
            crate::la_client::sync_auth_to_resolver(&la_sess.access_token);
        }
        let tx = self.async_worker_tx.clone();
        let session = self.session.clone();
        let cat = self.selected_category;
        std::thread::spawn(move || {
            let result = match cat {
                crate::models::HomeworkCategory::Current => {
                    crate::cloud_client::CloudClient::get_homework_list(&session, "1")
                }
                crate::models::HomeworkCategory::History => {
                    crate::cloud_client::CloudClient::get_homework_list(&session, "2")
                }
                crate::models::HomeworkCategory::VocabPk => {
                    let mut all = Vec::new();
                    let mut err = None;
                    match crate::cloud_client::CloudClient::get_homework_list(&session, "1") {
                        Ok(curr) => all.extend(curr),
                        Err(e) => err = Some(e),
                    }
                    match crate::cloud_client::CloudClient::get_homework_list(&session, "2") {
                        Ok(hist) => all.extend(hist),
                        Err(e) => {
                            if err.is_none() {
                                err = Some(e);
                            }
                        }
                    }
                    if all.is_empty() && let Some(e) = err {
                        Err(e)
                    } else {
                        let mut seen = std::collections::HashSet::new();
                        all.retain(|h| seen.insert(h.id.clone()));
                        all.retain(|h| h.is_pk_or_vocab());
                        Ok(all)
                    }
                }
                crate::models::HomeworkCategory::Local => Ok(Vec::new()),
            };
            let _ = tx.send(crate::state::SecureWorkerMessage::new(
                crate::state::AsyncWorkerEvent::HomeworkRefreshResult(result),
            ));
        });
    }

    /// 100% 零顶号直接接管本地 ETS 账号会话
    pub fn apply_ets_local_direct_session(&mut self, idx: usize) -> Result<(), String> {
        let acc = self.ets_local_accounts.get(idx).ok_or("账号索引越界")?.clone();
        if acc.token.trim().is_empty() {
            return Err("该账号在本地未保存有效 Token".to_string());
        }

        self.session = crate::models::UserSession {
            phone: acc.account.clone(),
            token: acc.token.clone(),
            device_code: if !self.session.device_code.is_empty() {
                self.session.device_code.clone()
            } else {
                crate::cloud_client::CloudClient::get_or_generate_device_code(None)
            },
            parent_account_id: acc.cur_res_item_id.clone(),
            user_name: acc.name.clone(),
            is_logged_in: true,
        };
        crate::cloud_client::CloudClient::save_session(&self.session);

        self.refresh_cloud_homework();

        self.notification = Some((
            "✔ 已无感接管本地 e听说 会话（零顶号），正在同步作业列表...".to_string(),
            InfoBarKind::Success,
        ));

        Ok(())
    }

    /// 自动将本地已保存账号和解密明文密码填入登录表单
    pub fn fill_ets_local_login_form(&mut self, idx: usize) {
        if let Some(acc) = self.ets_local_accounts.get(idx).cloned() {
            self.session.phone = acc.account.clone();
            if let Some(ref pwd) = acc.password_plain {
                self.login_password = pwd.clone();
            }
            self.notification = Some((
                "已自动填入账号与解密密码，点击【立即登录】即可获取全新 Token".to_string(),
                InfoBarKind::Success,
            ));
        }
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

    #[test]
    fn test_secure_worker_message_mask_verification() {
        let msg = SecureWorkerMessage::new(AsyncWorkerEvent::LocalScanResult(Vec::new()));
        assert!(msg.is_valid(), "Initial message mask must be valid");

        // 篡改时间戳
        let tampered_time = SecureWorkerMessage {
            payload: AsyncWorkerEvent::LocalScanResult(Vec::new()),
            timestamp: msg.timestamp.wrapping_add(1),
            salt: msg.salt,
            mask: msg.mask,
        };
        assert!(!tampered_time.is_valid(), "Tampered timestamp must fail validation");

        // 篡改盐
        let tampered_salt = SecureWorkerMessage {
            payload: AsyncWorkerEvent::LocalScanResult(Vec::new()),
            timestamp: msg.timestamp,
            salt: msg.salt.wrapping_add(1),
            mask: msg.mask,
        };
        assert!(!tampered_salt.is_valid(), "Tampered salt must fail validation");

        // 篡改掩码
        let tampered_mask = SecureWorkerMessage {
            payload: AsyncWorkerEvent::LocalScanResult(Vec::new()),
            timestamp: msg.timestamp,
            salt: msg.salt,
            mask: msg.mask ^ 0xFF,
        };
        assert!(!tampered_mask.is_valid(), "Tampered mask must fail validation");
    }

    #[test]
    fn test_async_worker_channel_and_poll_events() {
        let mut state = AppState::default();
        let tx = state.worker_sender();

        // 1. 测试作业刷新事件安全消费
        state.homework_refreshing = true;
        let _ = tx.send(SecureWorkerMessage::new(AsyncWorkerEvent::HomeworkRefreshResult(Ok(Vec::new()))));
        state.poll_async_events();
        assert!(!state.homework_refreshing);
        let notif_str = state.notification.as_ref().map(|n| n.0.as_str()).unwrap_or("");
        assert!(notif_str.contains("成功获取到") || notif_str.contains("已自动加载本地离线作业列表"));

        // 2. 测试本地试卷扫描事件安全消费
        state.local_scanning = true;
        let _ = tx.send(SecureWorkerMessage::new(AsyncWorkerEvent::LocalScanResult(Vec::new())));
        state.poll_async_events();
        assert!(!state.local_scanning);
        assert!(state.notification.as_ref().map(|n| &n.0).unwrap_or(&String::new()).contains("扫描完成，发现 0 份"));

        // 3. 测试防篡改过滤：恶意伪造消息被安全丢弃，不改变状态
        state.homework_refreshing = true;
        let forged = SecureWorkerMessage {
            payload: AsyncWorkerEvent::HomeworkRefreshResult(Ok(Vec::new())),
            timestamp: 123456,
            salt: 999999,
            mask: 0xDEADBEEF,
        };
        let _ = tx.send(forged);
        state.poll_async_events();
        // 伪造消息被丢弃，homework_refreshing 仍然保持不变
        assert!(state.homework_refreshing);
    }

    #[test]
    fn test_ets_local_state_machine() {
        let mut state = AppState::default();
        assert!(!state.ets_local_scanned);
        assert!(!state.ets_local_scanning);
        assert!(state.ets_local_accounts.is_empty());

        // 模拟一个本地账号
        let mock_acc = crate::ets_local_db::EtsLocalAccount {
            account: "13800138000".to_string(),
            name: "张三".to_string(),
            cover_url: "".to_string(),
            token: "mock_jwt_token_12345".to_string(),
            password_plain: Some("plain_pass_123".to_string()),
            cur_res_item_id: "res_999".to_string(),
            student_card_no: "BJ123456".to_string(),
            student_card_pwd: "pass".to_string(),
            school_name: "测试学校".to_string(),
            class_name: "3".to_string(),
            teacher_name: "李老师".to_string(),
            expiry_time: "2027-01-01".to_string(),
            grade_name: "高一".to_string(),
        };
        state.ets_local_accounts.push(mock_acc);

        // 测试自动填充表单
        state.fill_ets_local_login_form(0);
        assert_eq!(state.session.phone, "13800138000");
        assert_eq!(state.login_password, "plain_pass_123");
        assert!(state.notification.as_ref().unwrap().0.contains("已自动填入账号与解密密码"));

        // 测试免密零顶号直接接管
        let res = state.apply_ets_local_direct_session(0);
        assert!(res.is_ok());
        assert!(state.session.is_logged_in);
        assert_eq!(state.session.token, "mock_jwt_token_12345");
        assert_eq!(state.session.user_name, "张三");
        assert_eq!(state.session.parent_account_id, "res_999");
        assert!(state.homework_refreshing);
        assert!(state.notification.as_ref().unwrap().0.contains("已无感接管本地 e听说 会话（零顶号）"));

        // 测试空 Token 错误拦截
        state.ets_local_accounts[0].token = "".to_string();
        assert!(state.apply_ets_local_direct_session(0).is_err());

        // 测试索引越界
        assert!(state.apply_ets_local_direct_session(99).is_err());

        // 测试不存在路径扫描
        state.ets_local_db_path = Some("non_existent_ets_xyz.db".to_string());
        state.scan_ets_local_db();
        assert!(state.ets_local_scanned);
        assert!(!state.ets_local_scanning);
        assert!(state.ets_local_accounts.is_empty());

        // 测试真实环境探测或扫描（若本地真实数据库存在）
        let real_path = std::path::PathBuf::from(r"F:\fepc\ETS\localdata\ETS.db");
        if real_path.exists() {
            state.ets_local_db_path = Some(real_path.to_string_lossy().to_string());
            state.scan_ets_local_db();
            assert!(state.ets_local_scanned);
            assert!(!state.ets_local_accounts.is_empty());
        }
    }
}


