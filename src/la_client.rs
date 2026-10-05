use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::utils::device_info;

pub const LA_BASE_URL: &str = "https://api.lastudio.cc";
pub const LA_SOFTWARE_KEY: &str = "fepc";
pub const LA_CLIENT_KEY: &str = "fepc-windows";
pub const LA_APP_VERSION_CODE: i64 = 106;

pub fn get_user_agent() -> String {
    let pkg_ver = env!("CARGO_PKG_VERSION");
    let plat_ver = device_info::get_platform_version();
    let os_ver = plat_ver.strip_prefix("Windows ").unwrap_or(&plat_ver);
    format!("Fe-PC/{pkg_ver} Windows/{os_ver}")
}

pub const LA_USER_AGENT: &str = concat!("Fe-PC/", env!("CARGO_PKG_VERSION"), " Windows/11");
pub const TIMEOUT_SECS: u64 = 10;

#[inline]
pub(crate) fn apply_common_headers(req: ureq::Request) -> ureq::Request {
    req.set("User-Agent", &get_user_agent())
        .set("X-App-Version-Code", &LA_APP_VERSION_CODE.to_string())
        .set("X-Software-Key", LA_SOFTWARE_KEY)
        .set("X-Client-Key", LA_CLIENT_KEY)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LaUser {
    pub id: i64,
    pub username: String,
    pub member_level: i32,
    pub tier: String,
    pub avatar_url: Option<String>,
    #[serde(default = "default_user_role")]
    pub role: i32,
    #[serde(default)]
    pub role_display: Option<String>,
    #[serde(default)]
    pub member_expire_time: Option<String>,
}

fn default_user_role() -> i32 {
    3
}

impl Default for LaUser {
    fn default() -> Self {
        Self {
            id: 0,
            username: String::new(),
            member_level: 0,
            tier: "free".to_string(),
            avatar_url: None,
            role: 3,
            role_display: None,
            member_expire_time: None,
        }
    }
}

impl LaUser {
    pub fn tier_display_name(&self) -> String {
        if self.role == 0 {
            "超级管理员 (Ultra)".to_string()
        } else if self.role == 1 {
            "管理员 (Ultra)".to_string()
        } else {
            match self.tier.to_lowercase().as_str() {
                "ultra" => "Ultra 会员".to_string(),
                "pro" => "Pro 会员".to_string(),
                _ => "免费版".to_string(),
            }
        }
    }

    pub fn is_member(&self) -> bool {
        self.role <= 1 || self.member_level > 0 || self.tier.eq_ignore_ascii_case("pro") || self.tier.eq_ignore_ascii_case("ultra")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaSession {
    pub session_id: String,
    pub device_id: Option<String>,
    pub user: LaUser,
    pub access_token: String,
    pub refresh_token: String,
    pub cached_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaCloudSession {
    pub account_display: String,
    pub token: String,
    pub device_code: String,
    pub parent_account_id: String,
    pub selected_account: Value,
    pub valid_accounts: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaDevice {
    pub id: i64,
    pub device_name: String,
    pub platform_version: String,
    pub client_key: String,
    pub is_current: bool,
    pub last_active_at: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaSoftwarePolicy {
    #[serde(alias = "is_kill_switch_on", alias = "kill_switch", default)]
    pub is_kill_switch_on: bool,
    #[serde(alias = "latest_version_code", default)]
    pub latest_version_code: i64,
    #[serde(alias = "is_force", alias = "force_update", default)]
    pub is_force: bool,
    #[serde(alias = "update_url", default)]
    pub update_url: String,
    #[serde(alias = "announcement_title", default)]
    pub announcement_title: Option<String>,
    #[serde(alias = "announcement_message", alias = "message", default)]
    pub announcement_message: Option<String>,
    #[serde(alias = "notice_message", default)]
    pub notice_message: Option<String>,
    #[serde(alias = "update_message", default)]
    pub update_message: Option<String>,
    #[serde(alias = "announcement_url", default)]
    pub announcement_url: Option<String>,
    #[serde(alias = "changelog_summary", default)]
    pub changelog_summary: Option<String>,
    #[serde(default)]
    pub features: Value,
}

impl LaSoftwarePolicy {
    pub fn kill_switch(&self) -> bool {
        self.is_kill_switch_on
    }
    pub fn force_update(&self) -> bool {
        self.is_force
    }
    pub fn message(&self) -> &str {
        self.announcement_message.as_deref().or(self.notice_message.as_deref()).unwrap_or("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaAccountSession {
    #[serde(alias = "session_id", default)]
    pub id: String,
    #[serde(alias = "software_key", default)]
    pub client_key: String,
    #[serde(alias = "display_name", alias = "name", default)]
    pub device_name: String,
    #[serde(alias = "ip_address", default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub is_current: bool,
    #[serde(alias = "last_seen_at", alias = "updated_at", default)]
    pub last_active_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub created_at: String,
}

pub const DEFAULT_SIGNING_PUBLIC_KEY: &str = "iIsgUPrBJidorQGIs-BJCt5EiLrRHVbAWStnNaPn3cI";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaFeature {
    pub feature_key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    #[serde(default)]
    pub authorized: Option<bool>,
    #[serde(default)]
    pub available_for_version: Option<bool>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub requires_login: bool,
    #[serde(default)]
    pub requires_membership: bool,
    #[serde(default)]
    pub min_member_level: i32,
    #[serde(default)]
    pub min_member_tier: Option<String>,
    #[serde(default)]
    pub deny_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureCheckResult {
    pub allowed: bool,
    pub reason: String,
    pub required_tier: String,
    pub requires_login: bool,
    pub requires_membership: bool,
}

impl Default for FeatureCheckResult {
    fn default() -> Self {
        Self {
            allowed: true,
            reason: String::new(),
            required_tier: String::new(),
            requires_login: false,
            requires_membership: false,
        }
    }
}

static DYNAMIC_SIGNING_PUBLIC_KEY: std::sync::RwLock<Option<Vec<u8>>> = std::sync::RwLock::new(None);

/// 设置自定义验签公钥（主要用于自动化测试或离线模拟）
pub fn set_custom_signing_public_key(key: Option<Vec<u8>>) {
    if let Ok(mut lock) = DYNAMIC_SIGNING_PUBLIC_KEY.write() {
        *lock = key;
    }
}

/// 解码 Base64/Hex 格式的 Ed25519 32 字节公钥
pub fn decode_public_key_bytes(key_str: &str) -> Option<Vec<u8>> {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    use base64::Engine;

    let s = key_str.trim();
    if let Ok(b) = URL_SAFE_NO_PAD.decode(s) && b.len() == 32 {
        return Some(b);
    }
    if let Ok(b) = URL_SAFE.decode(s) && b.len() == 32 {
        return Some(b);
    }
    if let Ok(b) = STANDARD.decode(s) && b.len() == 32 {
        return Some(b);
    }
    if let Ok(b) = STANDARD_NO_PAD.decode(s) && b.len() == 32 {
        return Some(b);
    }
    if s.len() == 64
        && let Ok(b) = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
        && b.len() == 32
    {
        return Some(b);
    }
    None
}

/// 解码 Base64/Hex 格式的 Ed25519 64 字节签名
pub fn decode_signature_bytes(sig_str: &str) -> Option<Vec<u8>> {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    use base64::Engine;

    let s = sig_str.trim();
    if let Ok(b) = URL_SAFE_NO_PAD.decode(s) && b.len() == 64 {
        return Some(b);
    }
    if let Ok(b) = URL_SAFE.decode(s) && b.len() == 64 {
        return Some(b);
    }
    if let Ok(b) = STANDARD.decode(s) && b.len() == 64 {
        return Some(b);
    }
    if let Ok(b) = STANDARD_NO_PAD.decode(s) && b.len() == 64 {
        return Some(b);
    }
    if s.len() == 128
        && let Ok(b) = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
        && b.len() == 64
    {
        return Some(b);
    }
    None
}

/// 从服务端 /software-auth/signing-key 获取最新 Ed25519 验签公钥
pub fn fetch_signing_public_key() -> Result<Vec<u8>, String> {
    let url = format!("{LA_BASE_URL}/software-auth/signing-key");
    let req = ureq::get(&url)
        .timeout(Duration::from_secs(5))
        .set("Accept", "application/json");
    let resp = apply_common_headers(req)
        .call()
        .map_err(|e| format!("获取验签公钥失败: {e}"))?;
    let body_str = resp.into_string().map_err(|e| format!("读取签名公钥失败: {e}"))?;

    if let Ok(val) = serde_json::from_str::<Value>(&body_str) {
        let key_str = val
            .get("public_key")
            .or_else(|| val.get("key"))
            .or_else(|| val.get("signing_key"))
            .or_else(|| val.get("data").and_then(|d| d.get("public_key").or_else(|| d.get("key"))))
            .and_then(|s| s.as_str());
        if let Some(k) = key_str
            && let Some(bytes) = decode_public_key_bytes(k) {
                return Ok(bytes);
        }
    }
    if let Some(bytes) = decode_public_key_bytes(&body_str) {
        return Ok(bytes);
    }
    Err("未能从公钥接口解析出合法的 Ed25519 32 字节公钥".to_string())
}

/// 获取当前激活的验签公钥（优先自定义缓存/服务端拉取，兜底使用内置常量）
pub fn get_or_fetch_signing_public_key() -> Vec<u8> {
    if let Ok(lock) = DYNAMIC_SIGNING_PUBLIC_KEY.read()
        && let Some(ref k) = *lock {
            return k.clone();
    }
    let k = fetch_signing_public_key().unwrap_or_else(|_| {
        decode_public_key_bytes(DEFAULT_SIGNING_PUBLIC_KEY).expect("内置兜底公钥必须有效")
    });
    if let Ok(mut lock) = DYNAMIC_SIGNING_PUBLIC_KEY.write()
        && lock.is_none() {
            *lock = Some(k.clone());
    }
    k
}

/// 利用 ring 校验 Ed25519 签名与响应 body 防篡改
pub fn verify_signature(
    body: &[u8],
    signature_str: &str,
    public_key_bytes: &[u8],
) -> Result<(), String> {
    let sig_bytes = decode_signature_bytes(signature_str)
        .ok_or_else(|| "X-Signature 格式错误或长度不是 64 字节".to_string())?;

    let peer_public_key =
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key_bytes);
    peer_public_key
        .verify(body, &sig_bytes)
        .map_err(|_| "Ed25519 签名校验失败: 数据可能被篡改或伪造".to_string())
}

/// 校验服务端响应的签名 (使用默认/动态公钥)
pub fn verify_response_signature(body: &[u8], signature_str: &str) -> Result<(), String> {
    let pub_key = get_or_fetch_signing_public_key();
    verify_signature(body, signature_str, &pub_key)
}

/// 解析服务端返回的功能特性 JSON 数据（兼容多种包装格式与字典映射）
pub fn parse_features_from_json(body_str: &str) -> Result<Vec<LaFeature>, String> {
    if let Ok(features) = serde_json::from_str::<Vec<LaFeature>>(body_str) {
        return Ok(features);
    }
    let val: Value = serde_json::from_str(body_str)
        .map_err(|e| format!("解析功能列表响应 JSON 失败: {e}"))?;

    let array_candidate = val
        .as_array()
        .or_else(|| val.get("features").and_then(|v| v.as_array()))
        .or_else(|| val.get("data").and_then(|d| d.as_array()))
        .or_else(|| val.get("data").and_then(|d| d.get("features")).and_then(|v| v.as_array()));

    if let Some(arr) = array_candidate {
        let mut features = Vec::new();
        for item in arr {
            if let Ok(feat) = serde_json::from_value::<LaFeature>(item.clone()) {
                features.push(feat);
            }
        }
        return Ok(features);
    }

    // 检查字典映射形式 (例如 {"features": {"auto_complete": { ... }}} )
    let map_candidate = val
        .get("features")
        .and_then(|v| v.as_object())
        .or_else(|| val.get("data").and_then(|d| d.get("features")).and_then(|v| v.as_object()))
        .or_else(|| val.get("data").and_then(|d| d.as_object()))
        .or_else(|| val.as_object());

    if let Some(map) = map_candidate {
        let mut features = Vec::new();
        for (k, v) in map {
            if let Some(obj) = v.as_object() {
                let mut full_obj = obj.clone();
                if !full_obj.contains_key("feature_key") {
                    full_obj.insert("feature_key".to_string(), json!(k));
                }
                if let Ok(feat) = serde_json::from_value::<LaFeature>(Value::Object(full_obj)) {
                    features.push(feat);
                }
            }
        }
        if !features.is_empty() {
            return Ok(features);
        }
    }

    Err("响应中未包含有效的功能列表数据".to_string())
}

#[derive(Default, Clone, Copy, Debug)]
pub struct LaClient;

impl LaClient {
    pub fn new() -> Self {
        Self
    }

    fn session_file_path() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let dir = PathBuf::from(app_data).join("fepc");
            let _ = fs::create_dir_all(&dir);
            return dir.join("la_session.json");
        }
        PathBuf::from("la_session.json")
    }

    #[inline(never)]
    pub fn load_saved_session() -> Option<LaSession> {
        let path = Self::session_file_path();
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path) {
                return serde_json::from_str::<LaSession>(&content).ok();
            }
        None
    }

    pub fn save_session(session: &LaSession) {
        let path = Self::session_file_path();
        if let Ok(json_str) = serde_json::to_string_pretty(session) {
            let _ = fs::write(&path, json_str);
        }
    }

    pub fn clear_session() {
        let path = Self::session_file_path();
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn parse_error_response(status: u16, body_str: &str) -> String {
        if body_str.contains("人机验证") || body_str.contains("captcha_required") {
            return "需要完成人机验证".to_string();
        }

        let fallback = match status {
            401 => "登录状态已失效，请重新登录".to_string(),
            403 => "当前账号或客户端无权使用此功能（需升级会员）".to_string(),
            429 => "请求过于频繁，请稍后重试".to_string(),
            _ => format!("LA Studio 服务端错误 ({status})"),
        };

        if let Ok(val) = serde_json::from_str::<Value>(body_str) {
            if let Some(detail) = val.get("detail") {
                if let Some(s) = detail.as_str()
                    && !s.trim().is_empty() {
                        return s.to_string();
                    }
                if let Some(obj) = detail.as_object()
                    && let Some(msg) = obj.get("message").and_then(|v| v.as_str())
                        && !msg.trim().is_empty() {
                            return msg.to_string();
                        }
            }
            if let Some(msg) = val.get("message").and_then(|v| v.as_str())
                && !msg.trim().is_empty() {
                    return msg.to_string();
                }
        }

        if !body_str.trim().is_empty() && body_str.len() < 200 {
            return body_str.to_string();
        }
        fallback
    }

    /// 用户名密码登录
    pub fn login(username: &str, password: &str, cap_token: Option<&str>) -> Result<LaSession, String> {
        let trimmed_user = username.trim().to_lowercase();
        if trimmed_user.is_empty() {
            return Err("用户名不能为空".to_string());
        }
        if password.is_empty() {
            return Err("密码不能为空".to_string());
        }

        let device_payload = device_info::get_device_payload();
        let mut body = json!({
            "client_key": LA_CLIENT_KEY,
            "username": trimmed_user,
            "password": password,
            "device": device_payload
        });

        if let Some(token) = cap_token {
            let t = token.trim();
            if !t.is_empty()
                && let Some(obj) = body.as_object_mut() {
                    obj.insert("cap_token".to_string(), json!(t));
                }
        }

        let url = format!("{LA_BASE_URL}/client-auth/login");
        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json");
        let resp_res = apply_common_headers(req).send_json(&body);

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取登录响应失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                if code == 401 && (err_body.contains("unknown_client") || err_body.contains("客户端标识无效")) {
                    // 平滑过渡: 若远端尚未同步登记 fepc-windows，回退至兼容通道重试
                    if let Some(obj) = body.as_object_mut() {
                        obj.insert("client_key".to_string(), json!("fe-android"));
                    }
                    let retry_req = ureq::post(&url)
                        .timeout(Duration::from_secs(TIMEOUT_SECS))
                        .set("Accept", "application/json")
                        .set("Content-Type", "application/json");
                    let retry_res = apply_common_headers(retry_req).send_json(&body);
                    match retry_res {
                        Ok(r) => r.into_string().map_err(|e| format!("读取登录响应失败: {e}"))?,
                        Err(ureq::Error::Status(c, r)) => {
                            let b = r.into_string().unwrap_or_default();
                            return Err(Self::parse_error_response(c, &b));
                        }
                        Err(e) => return Err(format!("无法连接到 LA Studio 服务端: {e}")),
                    }
                } else {
                    let err_msg = Self::parse_error_response(code, &err_body);

                    let requires_captcha = err_body.contains("人机验证")
                        || err_body.contains("captcha_required")
                        || err_msg.contains("需要完成人机验证")
                        || serde_json::from_str::<Value>(&err_body)
                            .ok()
                            .and_then(|v| v.get("requires_captcha").and_then(|b| b.as_bool()))
                            .unwrap_or(false);

                    if requires_captcha && cap_token.is_none() {
                        // 后台静默求解 Cap 人机验证
                        if let Ok(solved_token) = crate::utils::native_cap_solver::solve_challenge("https://lastudio.cc/cap/f0d2ec6d51/") {
                            return Self::login(username, password, Some(&solved_token));
                        }
                    }

                    return Err(err_msg);
                }
            }
            Err(e) => return Err(format!("无法连接到 LA Studio 服务端: {e}")),
        };

        let json_val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "LA Studio 登录返回的数据格式无效".to_string())?;

        let access_token = json_val.get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "登录响应中缺少 access_token".to_string())?
            .to_string();

        let refresh_token = json_val.get("refresh_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "登录响应中缺少 refresh_token".to_string())?
            .to_string();

        let mut session = Self::get_session(&access_token)?;
        if let Some(user_val) = json_val.get("user") {
            if let Some(role) = user_val.get("role").and_then(|v| v.as_i64()) {
                session.user.role = role as i32;
            }
            if let Some(rd) = user_val.get("role_display").and_then(|v| v.as_str()) {
                session.user.role_display = Some(rd.to_string());
            }
            if let Some(et) = user_val.get("member_expire_time")
                .or_else(|| user_val.get("expire_time"))
                .or_else(|| user_val.get("expires_at"))
                .and_then(|v| v.as_str())
            {
                session.user.member_expire_time = Some(et.to_string());
            }
        }
        session.access_token = access_token;
        session.refresh_token = refresh_token;
        session.cached_at = Self::now_secs();
        Self::save_session(&session);

        Ok(session)
    }

    /// 获取会话与用户信息
    #[inline(never)]
    pub fn get_session(access_token: &str) -> Result<LaSession, String> {
        let url = format!("{LA_BASE_URL}/client-auth/session");
        let req = ureq::get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).call();

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取会话失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("连接会话服务失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "会话数据格式错误".to_string())?;

        let user_val = val.get("user")
            .ok_or_else(|| "响应缺少 user 节点".to_string())?;

        let (session_id, device_id, user) = crate::themida_lion_white!({
            let session_id = val.get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let device_id = val.get("device_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let uid = user_val.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let uname = user_val.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let member_level = user_val.get("member_level").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let role = user_val.get("role").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
            let role_display = user_val.get("role_display").and_then(|v| v.as_str()).map(|s| s.to_string());
            let member_expire_time = user_val.get("member_expire_time")
                .or_else(|| user_val.get("expire_time"))
                .or_else(|| user_val.get("expires_at"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let tier = user_val.get("tier").and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if role <= 1 {
                        "ultra".to_string()
                    } else {
                        match member_level {
                            2 => "ultra".to_string(),
                            1 => "pro".to_string(),
                            _ => "free".to_string(),
                        }
                    }
                });
            let avatar_url = user_val.get("avatar_url").and_then(|v| v.as_str()).map(|s| s.to_string());

            (session_id, device_id, LaUser {
                id: uid,
                username: uname,
                member_level,
                tier,
                avatar_url,
                role,
                role_display,
                member_expire_time,
            })
        });

        Ok(LaSession {
            session_id,
            device_id,
            user,
            access_token: access_token.to_string(),
            refresh_token: String::new(),
            cached_at: Self::now_secs(),
        })
    }

    /// 使用 refresh_token 刷新访问凭据
    pub fn refresh(refresh_token: &str) -> Result<(String, String), String> {
        let url = format!("{LA_BASE_URL}/client-auth/refresh");
        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareRefresh {refresh_token}"));
        let resp_res = apply_common_headers(req).call();

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("刷新令牌失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("连接令牌刷新服务失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "刷新令牌返回格式错误".to_string())?;

        let new_access = val.get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少新 access_token".to_string())?
            .to_string();

        let new_refresh = val.get("refresh_token")
            .and_then(|v| v.as_str())
            .unwrap_or(refresh_token)
            .to_string();

        Ok((new_access, new_refresh))
    }

    /// 获取云端托管的 ETS100 自动做题会话（无需学生输密码）
    pub fn get_cloud_session(access_token: &str, preferred_account_id: &str) -> Result<LaCloudSession, String> {
        let url = format!("{LA_BASE_URL}/client-auth/cloud/session");
        let body = json!({
            "preferred_account_id": preferred_account_id
        });

        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).send_json(&body);

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取云端会话失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("连接云端中继服务失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "云端会话数据解析失败".to_string())?;

        let token = val.get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "云端响应缺少 ETS100 令牌".to_string())?
            .to_string();
        let device_code = val.get("device_code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let parent_account_id = val.get("parent_account_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let account_display = val.get("account_display")
            .and_then(|v| v.as_str())
            .unwrap_or("云端已绑定账号")
            .to_string();

        let selected_account = val.get("selected_account").cloned().unwrap_or(json!({}));
        let valid_accounts = val.get("valid_accounts")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(LaCloudSession {
            account_display,
            token,
            device_code,
            parent_account_id,
            selected_account,
            valid_accounts,
        })
    }

    /// 获取云端生成的设备码
    pub fn get_cloud_device_code(access_token: &str) -> Result<String, String> {
        let url = format!("{LA_BASE_URL}/client-auth/cloud/device-code");
        let req = ureq::get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).call();

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取云端设备码失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("获取云端设备码失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "设备码数据解析失败".to_string())?;

        val.get("device_code")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "响应缺少 device_code 字段".to_string())
    }

    /// 将 ETS100 账号绑定到当前 LA 账号（用于云端代理登录）
    pub fn bind_ets100_account(
        access_token: &str,
        phone: &str,
        password: &str,
        device_code: &str,
    ) -> Result<(), String> {
        let url = format!("{LA_BASE_URL}/client-auth/platform-account/bind");
        let body = json!({
            "provider": "ets100-phone",
            "account": phone.trim(),
            "password": password,
            "platform_device_code": device_code.trim()
        });

        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).send_json(&body);

        match resp_res {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                Err(Self::parse_error_response(code, &err_body))
            }
            Err(e) => Err(format!("绑定账号失败: {e}")),
        }
    }

    /// 将畅言 (科大讯飞) 账号绑定到当前 LA 账号并获取会话
    pub fn bind_changyan_account(
        access_token: &str,
        login: &str,
        user_id: &str,
        suc_user_token: &str,
        preferred_account_id: &str,
    ) -> Result<LaCloudSession, String> {
        let url = format!("{LA_BASE_URL}/client-auth/platform-account/changyan");
        let body = json!({
            "login": login.trim(),
            "user_id": user_id.trim(),
            "suc_user_token": suc_user_token.trim(),
            "preferred_account_id": preferred_account_id.trim(),
        });

        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).send_json(&body);

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取畅言绑定响应失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("连接畅言绑定服务失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "畅言绑定响应数据解析失败".to_string())?;

        let token = val.get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "云端响应缺少 ETS100 令牌".to_string())?
            .to_string();
        let device_code = val.get("device_code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let parent_account_id = val.get("parent_account_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let account_display = val.get("account_display")
            .and_then(|v| v.as_str())
            .unwrap_or("云端已绑定账号")
            .to_string();

        let selected_account = val.get("selected_account").cloned().unwrap_or(json!({}));
        let valid_accounts = val.get("valid_accounts")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(LaCloudSession {
            account_display,
            token,
            device_code,
            parent_account_id,
            selected_account,
            valid_accounts,
        })
    }

    /// 查询当前绑定的设备列表
    pub fn list_devices(access_token: &str) -> Result<Vec<LaDevice>, String> {
        let url = format!("{LA_BASE_URL}/client-auth/account/devices");
        let req = ureq::get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).call();

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("获取设备列表失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("连接设备管理服务失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "设备列表数据格式错误".to_string())?;

        let mut devices = Vec::new();
        let current_platform_id = device_info::get_platform_device_id();
        if let Some(arr) = val.as_array() {
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                let device_name = item.get("display_name")
                    .or_else(|| item.get("device_name"))
                    .or_else(|| item.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("未知设备")
                    .to_string();
                let platform_version = item.get("platform_version")
                    .or_else(|| item.get("platform"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let client_key = item.get("client_key")
                    .or_else(|| item.get("software_key"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let dev_id_str = item.get("device_id").and_then(|v| v.as_str()).unwrap_or("");
                let is_current = item.get("is_current").and_then(|v| v.as_bool()).unwrap_or(false)
                    || (!dev_id_str.is_empty() && dev_id_str == current_platform_id);
                let last_active_at = item.get("last_active_at").and_then(|v| v.as_str()).map(|s| s.to_string());
                let created_at = item.get("created_at").and_then(|v| v.as_str()).map(|s| s.to_string());

                devices.push(LaDevice {
                    id,
                    device_name,
                    platform_version,
                    client_key,
                    is_current,
                    last_active_at,
                    created_at,
                });
            }
        }

        Ok(devices)
    }

    /// 解除设备绑定
    pub fn revoke_device(access_token: &str, device_row_id: i64) -> Result<(), String> {
        let url = format!("{LA_BASE_URL}/client-auth/account/devices/{device_row_id}");
        let req = ureq::delete(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).call();

        match resp_res {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                Err(Self::parse_error_response(code, &err_body))
            }
            Err(e) => Err(format!("解除设备失败: {e}")),
        }
    }

    /// 退出登录
    pub fn logout(access_token: &str) {
        let url = format!("{LA_BASE_URL}/client-auth/logout");
        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let _ = apply_common_headers(req).call();
        Self::clear_session();
    }

    /// 检查软件启动策略与版本更新
    pub fn check_software_policy() -> Result<LaSoftwarePolicy, String> {
        let urls = [
            format!("{LA_BASE_URL}/software/{LA_SOFTWARE_KEY}/config?version_code={LA_APP_VERSION_CODE}"),
            format!("{LA_BASE_URL}/software/fe/config?version_code={LA_APP_VERSION_CODE}"),
        ];

        let mut last_err = String::from("无法获取软件策略配置");
        for url in &urls {
            let req = ureq::get(url)
                .timeout(Duration::from_secs(TIMEOUT_SECS))
                .set("Accept", "application/json");
            let resp_res = apply_common_headers(req).call();

            match resp_res {
                Ok(resp) => {
                    let body_str = resp.into_string().map_err(|e| format!("读取策略响应失败: {e}"))?;
                    let val: Value = serde_json::from_str(&body_str)
                        .map_err(|e| format!("解析策略响应失败: {e}"))?;

                    let policy_val = if val.get("data").is_some_and(|d| d.is_object()) {
                        val["data"].clone()
                    } else {
                        val
                    };

                    return serde_json::from_value::<LaSoftwarePolicy>(policy_val)
                        .map_err(|e| format!("解析策略数据模型失败: {e}"));
                }
                Err(ureq::Error::Status(404, _)) => {
                    continue;
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let err_body = resp.into_string().unwrap_or_default();
                    last_err = Self::parse_error_response(code, &err_body);
                }
                Err(e) => {
                    last_err = format!("连接策略服务失败: {e}");
                }
            }
        }

        Err(last_err)
    }

    /// 查询当前账号的所有在线会话
    pub fn list_sessions(access_token: &str) -> Result<Vec<LaAccountSession>, String> {
        let url = format!("{LA_BASE_URL}/client-auth/account/sessions");
        let req = ureq::get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).call();

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("获取会话列表失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("连接会话管理服务失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "会话列表数据格式错误".to_string())?;

        let arr = if let Some(a) = val.as_array() {
            a.clone()
        } else if let Some(a) = val.get("sessions").or_else(|| val.get("data")).and_then(|v| v.as_array()) {
            a.clone()
        } else {
            Vec::new()
        };

        let mut sessions = Vec::new();
        for item in arr {
            if let Ok(sess) = serde_json::from_value::<LaAccountSession>(item) {
                sessions.push(sess);
            }
        }
        Ok(sessions)
    }

    /// 下线/吊销指定在线会话
    pub fn revoke_session(access_token: &str, session_id: &str) -> Result<(), String> {
        let url = format!("{LA_BASE_URL}/client-auth/account/sessions/{session_id}");
        let req = ureq::delete(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).call();

        match resp_res {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                Err(Self::parse_error_response(code, &err_body))
            }
            Err(e) => Err(format!("下线会话失败: {e}")),
        }
    }

    /// 修改账号密码
    pub fn change_password(access_token: &str, old_pw: &str, new_pw: &str) -> Result<(), String> {
        let url = format!("{LA_BASE_URL}/client-auth/account/change-password");
        let body = json!({
            "old_password": old_pw,
            "new_password": new_pw,
            "old_pw": old_pw,
            "new_pw": new_pw,
        });
        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).send_json(&body);

        match resp_res {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                Err(Self::parse_error_response(code, &err_body))
            }
            Err(e) => Err(format!("修改密码失败: {e}")),
        }
    }

    /// 请求核心功能 Ed25519 鉴权授权
    pub fn authorize_feature(access_token: &str, feature_key: &str) -> Result<Value, String> {
        if access_token.trim().is_empty() {
            return Err("访问令牌为空".to_string());
        }

        use sha2::Digest;
        let seed = format!("{}:{}:{}", uuid::Uuid::new_v4(), Self::now_secs(), feature_key);
        let hash = sha2::Sha256::digest(seed.as_bytes());
        let nonce: String = hash[..24].iter().map(|b| format!("{b:02x}")).collect();

        let body = json!({
            "software_key": LA_SOFTWARE_KEY,
            "feature_key": feature_key,
            "operation_nonce": nonce,
            "app_version_code": LA_APP_VERSION_CODE,
        });

        let url = format!("{LA_BASE_URL}/client-auth/features/authorize");
        let req = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"));
        let resp_res = apply_common_headers(req).send_json(&body);

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取鉴权响应失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
            }
            Err(e) => return Err(format!("连接功能鉴权服务失败: {e}")),
        };

        let val: Value = serde_json::from_str(&body_str)
            .map_err(|_| "鉴权响应数据格式错误".to_string())?;

        let allowed = val.get("allowed").and_then(|v| v.as_bool()).unwrap_or(false);
        if !allowed {
            let msg = val.get("message").and_then(|v| v.as_str()).unwrap_or("功能鉴权未通过");
            return Err(msg.to_string());
        }

        // 若成功获得签名，配置到 resolver
        #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
        {
            resolver::auth::configure_auth(
                LA_BASE_URL,
                LA_SOFTWARE_KEY,
                feature_key,
                None,
                LA_APP_VERSION_CODE,
            ).ok();
            resolver::auth::set_token(access_token);
            resolver::auth::set_auth_enabled(true);
        }

        Ok(val)
    }

    /// 获取用户云端声音资产列表
    pub fn get_user_voices(&self, token: &str) -> Result<Vec<VoiceProfile>, String> {
        let auth_hdr = if token.starts_with("Bearer ") || token.starts_with("SoftwareAccess ") {
            token.to_string()
        } else {
            format!("SoftwareAccess {token}")
        };

        let urls = [
            format!("{LA_BASE_URL}/client-auth/account/voices"),
            "https://lastudio.cc/client-auth/account/voices".to_string(),
            format!("{LA_BASE_URL}/user/voices"),
            "https://lastudio.cc/user/voices".to_string(),
        ];

        let mut last_err = String::from("无法连接到声音资产服务");
        for url in &urls {
            let req = ureq::get(url)
                .timeout(Duration::from_secs(TIMEOUT_SECS))
                .set("Accept", "application/json")
                .set("Authorization", &auth_hdr);
            let resp_res = apply_common_headers(req).call();

            match resp_res {
                Ok(resp) => {
                    let body_str = resp.into_string().map_err(|e| format!("读取声音列表响应失败: {e}"))?;
                    if let Ok(profiles) = serde_json::from_str::<Vec<VoiceProfile>>(&body_str) {
                        return Ok(profiles);
                    }
                    if let Ok(val) = serde_json::from_str::<Value>(&body_str)
                        && let Some(arr) = val.get("voices").or_else(|| val.get("data")).and_then(|v| v.as_array())
                        && let Ok(profiles) = serde_json::from_value::<Vec<VoiceProfile>>(Value::Array(arr.clone()))
                    {
                        return Ok(profiles);
                    }
                    return Err("声音列表数据格式解析失败".to_string());
                }
                Err(ureq::Error::Status(code, resp)) => {
                    if code == 404 {
                        continue;
                    }
                    let err_body = resp.into_string().unwrap_or_default();
                    last_err = Self::parse_error_response(code, &err_body);
                }
                Err(e) => {
                    last_err = format!("连接声音服务失败: {e}");
                }
            }
        }

        Err(last_err)
    }

    /// 下载云端声音参考音频文件并保存至指定本地路径
    pub fn download_voice_audio(
        &self,
        voice_id: &str,
        token: &str,
        dest_path: &std::path::Path,
    ) -> Result<(), String> {
        let auth_hdr = if token.starts_with("Bearer ") || token.starts_with("SoftwareAccess ") {
            token.to_string()
        } else {
            format!("SoftwareAccess {token}")
        };

        let urls = [
            format!("{LA_BASE_URL}/client-auth/account/voices/{voice_id}/audio"),
            format!("https://lastudio.cc/client-auth/account/voices/{voice_id}/audio"),
            format!("{LA_BASE_URL}/user/voices/{voice_id}/audio"),
            format!("https://lastudio.cc/user/voices/{voice_id}/audio"),
        ];

        let mut last_err = String::from("无法下载声音参考音频");
        for url in &urls {
            let req = ureq::get(url)
                .timeout(Duration::from_secs(TIMEOUT_SECS))
                .set("Accept", "*/*")
                .set("Authorization", &auth_hdr);
            let resp_res = apply_common_headers(req).call();

            match resp_res {
                Ok(resp) => {
                    let content_type = resp.header("Content-Type").unwrap_or("").to_lowercase();
                    if content_type.contains("text/html") {
                        continue;
                    }
                    use std::io::{Read, Write};
                    let mut reader = resp.into_reader();
                    let mut header_buf = [0u8; 16];
                    let n = reader.read(&mut header_buf).unwrap_or(0);
                    if n >= 4 && (&header_buf[..4] == b"<!DO" || &header_buf[..4] == b"<htm" || &header_buf[..2] == b"{\"") {
                        continue;
                    }
                    if let Some(parent) = dest_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let mut file = fs::File::create(dest_path).map_err(|e| format!("创建目标音频文件失败: {e}"))?;
                    if n > 0 {
                        file.write_all(&header_buf[..n]).map_err(|e| format!("写入音频文件失败: {e}"))?;
                    }
                    std::io::copy(&mut reader, &mut file).map_err(|e| format!("写入音频文件失败: {e}"))?;
                    return Ok(());
                }
                Err(ureq::Error::Status(code, resp)) => {
                    if code == 404 {
                        continue;
                    }
                    let err_body = resp.into_string().unwrap_or_default();
                    last_err = Self::parse_error_response(code, &err_body);
                }
                Err(e) => {
                    last_err = format!("下载声音音频连接失败: {e}");
                }
            }
        }

        Err(last_err)
    }

    /// 拉取公开功能特性列表 (免登录)
    pub fn fetch_public_features() -> Result<Vec<LaFeature>, String> {
        let urls = [
            format!("{LA_BASE_URL}/software/{LA_SOFTWARE_KEY}/features?version_code={LA_APP_VERSION_CODE}"),
            format!("{LA_BASE_URL}/software/fe/features?version_code={LA_APP_VERSION_CODE}"),
        ];

        let mut last_err = String::from("无法获取公开功能特性清单");
        for url in &urls {
            let req = ureq::get(url)
                .timeout(Duration::from_secs(TIMEOUT_SECS))
                .set("Accept", "application/json");
            let resp_res = apply_common_headers(req).call();

            match resp_res {
                Ok(resp) => {
                    let sig_header = resp.header("X-Signature").map(|s| s.to_string());
                    let mut reader = resp.into_reader();
                    let mut body_bytes = Vec::new();
                    use std::io::Read;
                    if let Err(e) = reader.read_to_end(&mut body_bytes) {
                        return Err(format!("读取公开功能响应失败: {e}"));
                    }

                    if let Some(ref sig) = sig_header
                        && let Err(e) = verify_response_signature(&body_bytes, sig) {
                            eprintln!("[WARN] [Security] fetch_public_features 响应验签告警: {e}");
                    }

                    let body_str = String::from_utf8_lossy(&body_bytes);
                    return parse_features_from_json(&body_str);
                }
                Err(ureq::Error::Status(404, _)) => {
                    continue;
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let err_body = resp.into_string().unwrap_or_default();
                    last_err = Self::parse_error_response(code, &err_body);
                }
                Err(e) => {
                    last_err = format!("连接公开功能特性服务失败: {e}");
                }
            }
        }

        Err(last_err)
    }

    /// 拉取当前登录用户的功能权限清单
    pub fn fetch_user_features(access_token: &str) -> Result<Vec<LaFeature>, String> {
        if access_token.trim().is_empty() {
            return Err("访问令牌为空".to_string());
        }

        let auth_hdr = if access_token.starts_with("Bearer ") || access_token.starts_with("SoftwareAccess ") {
            access_token.to_string()
        } else {
            format!("SoftwareAccess {access_token}")
        };

        let urls = [
            format!("{LA_BASE_URL}/client-auth/features"),
            format!("{LA_BASE_URL}/client-auth/features/list"),
        ];

        let mut last_err = String::from("无法获取用户功能特性清单");
        for url in &urls {
            let req = ureq::get(url)
                .timeout(Duration::from_secs(TIMEOUT_SECS))
                .set("Accept", "application/json")
                .set("Authorization", &auth_hdr);
            let resp_res = apply_common_headers(req).call();

            match resp_res {
                Ok(resp) => {
                    let sig_header = resp.header("X-Signature").map(|s| s.to_string());
                    let mut reader = resp.into_reader();
                    let mut body_bytes = Vec::new();
                    use std::io::Read;
                    if let Err(e) = reader.read_to_end(&mut body_bytes) {
                        return Err(format!("读取用户功能响应失败: {e}"));
                    }

                    if let Some(ref sig) = sig_header
                        && let Err(e) = verify_response_signature(&body_bytes, sig) {
                            eprintln!("[WARN] [Security] fetch_user_features 响应验签告警: {e}");
                    }

                    let body_str = String::from_utf8_lossy(&body_bytes);
                    return parse_features_from_json(&body_str);
                }
                Err(ureq::Error::Status(404, _)) => {
                    continue;
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let err_body = resp.into_string().unwrap_or_default();
                    last_err = Self::parse_error_response(code, &err_body);
                }
                Err(e) => {
                    last_err = format!("连接用户功能列表服务失败: {e}");
                }
            }
        }

        Err(last_err)
    }
}

/// 拉取公开功能清单 (便捷顶层函数)
pub fn fetch_public_features() -> Result<Vec<LaFeature>, String> {
    LaClient::fetch_public_features()
}

/// 拉取当前登录用户的功能权限清单 (便捷顶层函数)
pub fn fetch_user_features(access_token: &str) -> Result<Vec<LaFeature>, String> {
    LaClient::fetch_user_features(access_token)
}

/// 将 access_token 同步至 resolver::auth 与 cloud::auth 并开启内部授权
pub fn sync_auth_to_resolver(access_token: &str) {
    #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
    {
        let token = access_token.trim();
        let enabled = !token.is_empty();

        // 1. 同步题目解析引擎 (resolver)
        resolver::auth::configure_auth(
            LA_BASE_URL,
            LA_SOFTWARE_KEY,
            "answer_parser",
            None,
            LA_APP_VERSION_CODE,
        ).ok();
        resolver::auth::set_token(token);
        resolver::auth::set_auth_enabled(enabled);

        // 2. 同步云端接口与作业下载引擎 (cloud)
        cloud::auth::configure_auth(
            LA_BASE_URL,
            LA_SOFTWARE_KEY,
            "cloud_mode",
            None,
            LA_APP_VERSION_CODE,
        ).ok();
        cloud::auth::set_token(token);
        cloud::auth::set_auth_enabled(enabled);
    }
}

/// 后台异步上报功能调用统计
pub fn report_feature_usage_async(feature_key: &str) {
    if let Some(la_sess) = LaClient::load_saved_session() {
        let token = la_sess.access_token;
        let key = feature_key.to_string();
        std::thread::spawn(move || {
            let _ = LaClient::authorize_feature(&token, &key);
        });
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    #[test]
    fn test_default_signing_key_validity() {
        let key_bytes = decode_public_key_bytes(DEFAULT_SIGNING_PUBLIC_KEY);
        assert!(key_bytes.is_some(), "默认 Ed25519 公钥格式必须合法");
        let bytes = key_bytes.unwrap();
        assert_eq!(bytes.len(), 32, "Ed25519 公钥必须严格为 32 字节");
    }

    #[test]
    fn test_ed25519_signature_verification_and_tamper_proofing() {
        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("生成 Ed25519 密钥对失败");
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("解析 pkcs8 失败");
        let pub_key_bytes = key_pair.public_key().as_ref();

        let original_body = br#"{"code":0,"data":[{"feature_key":"auto_complete","status":"enabled"}]}"#;
        let sig = key_pair.sign(original_body);
        let sig_b64 = URL_SAFE_NO_PAD.encode(sig.as_ref());

        // 1. 正常数据与对应签名校验必须成功
        let verify_res = verify_signature(original_body, &sig_b64, pub_key_bytes);
        assert!(verify_res.is_ok(), "正常合法签名校验必须通过");

        // 2. 响应 Body 被篡改 (例如恶意篡改权限状态或价格)
        let tampered_body = br#"{"code":0,"data":[{"feature_key":"auto_complete","status":"free_unlocked"}]}"#;
        let tamper_res = verify_signature(tampered_body, &sig_b64, pub_key_bytes);
        assert!(tamper_res.is_err(), "篡改后的 Body 校验必须失败阻断");

        // 3. 伪造或篡改签名内容
        let mut tampered_sig_bytes = sig.as_ref().to_vec();
        tampered_sig_bytes[0] ^= 0xFF;
        let tampered_sig_b64 = URL_SAFE_NO_PAD.encode(&tampered_sig_bytes);
        let sig_tamper_res = verify_signature(original_body, &tampered_sig_b64, pub_key_bytes);
        assert!(sig_tamper_res.is_err(), "篡改后的签名必须校验失败");

        // 4. 非法长度或损坏的格式
        assert!(verify_signature(original_body, "not-a-valid-sig", pub_key_bytes).is_err());
    }

    #[test]
    fn test_parse_features_various_formats() {
        // 格式 1: 标准扁平数组
        let json_arr = r#"[
            {
                "feature_key": "auto_complete",
                "name": "全自动做题",
                "status": "enabled",
                "requires_login": true,
                "requires_membership": true,
                "min_member_level": 1
            }
        ]"#;
        let features = parse_features_from_json(json_arr).expect("解析扁平数组失败");
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].feature_key, "auto_complete");
        assert!(features[0].requires_login);

        // 格式 2: code/data 包装
        let json_wrapped = r#"{
            "code": 0,
            "data": [
                {
                    "feature_key": "voice_clone",
                    "name": "声音克隆",
                    "status": "disabled",
                    "reason": "算力节点升级中"
                }
            ]
        }"#;
        let features2 = parse_features_from_json(json_wrapped).expect("解析 data 包装失败");
        assert_eq!(features2.len(), 1);
        assert_eq!(features2[0].feature_key, "voice_clone");
        assert_eq!(features2[0].status, "disabled");
        assert_eq!(features2[0].reason.as_deref(), Some("算力节点升级中"));

        // 格式 3: 字典映射形式
        let json_map = r#"{
            "features": {
                "answer_parser": {
                    "name": "解析核心",
                    "status": "enabled",
                    "requires_login": false
                }
            }
        }"#;
        let features3 = parse_features_from_json(json_map).expect("解析字典映射失败");
        assert_eq!(features3.len(), 1);
        assert_eq!(features3[0].feature_key, "answer_parser");
        assert_eq!(features3[0].name, "解析核心");
    }

    #[test]
    fn test_la_user_role_and_tier_display() {
        let super_admin = LaUser {
            id: 1,
            username: "root".to_string(),
            member_level: 0,
            tier: "free".to_string(),
            avatar_url: None,
            role: 0,
            role_display: Some("超级管理员".to_string()),
            member_expire_time: None,
        };
        assert_eq!(super_admin.tier_display_name(), "超级管理员 (Ultra)");
        assert!(super_admin.is_member());

        let admin = LaUser {
            id: 2,
            username: "admin".to_string(),
            member_level: 0,
            tier: "free".to_string(),
            avatar_url: None,
            role: 1,
            role_display: Some("管理员".to_string()),
            member_expire_time: None,
        };
        assert_eq!(admin.tier_display_name(), "管理员 (Ultra)");
        assert!(admin.is_member());

        let pro_user = LaUser {
            id: 3,
            username: "pro_user".to_string(),
            member_level: 1,
            tier: "pro".to_string(),
            avatar_url: None,
            role: 3,
            role_display: None,
            member_expire_time: Some("2028-12-31".to_string()),
        };
        assert_eq!(pro_user.tier_display_name(), "Pro 会员");
        assert!(pro_user.is_member());

        let ultra_user = LaUser {
            id: 4,
            username: "ultra_user".to_string(),
            member_level: 2,
            tier: "ultra".to_string(),
            avatar_url: None,
            role: 3,
            role_display: None,
            member_expire_time: None,
        };
        assert_eq!(ultra_user.tier_display_name(), "Ultra 会员");
        assert!(ultra_user.is_member());

        let free_user = LaUser {
            id: 5,
            username: "free_user".to_string(),
            member_level: 0,
            tier: "free".to_string(),
            avatar_url: None,
            role: 3,
            role_display: None,
            member_expire_time: None,
        };
        assert_eq!(free_user.tier_display_name(), "免费版");
        assert!(!free_user.is_member());
    }
}



