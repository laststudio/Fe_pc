use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::utils::device_info;

pub const LA_BASE_URL: &str = "https://api.lastudio.cc";
pub const LA_CLIENT_KEY: &str = "fe-android";
pub const LA_SOFTWARE_KEY: &str = "fe";
pub const LA_USER_AGENT: &str = "Fe/1.3.11 Android";
pub const TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LaUser {
    pub id: i64,
    pub username: String,
    pub member_level: i32,
    pub tier: String,
    pub avatar_url: Option<String>,
}

impl LaUser {
    pub fn tier_display_name(&self) -> &str {
        match self.tier.to_lowercase().as_str() {
            "ultra" => "Ultra 会员",
            "pro" => "Pro 会员",
            _ => "免费版",
        }
    }

    pub fn is_member(&self) -> bool {
        self.member_level > 0 || self.tier.eq_ignore_ascii_case("pro") || self.tier.eq_ignore_ascii_case("ultra")
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaSoftwarePolicy {
    pub kill_switch: bool,
    pub latest_version_code: i32,
    pub force_update: bool,
    pub update_url: String,
    pub message: String,
}

pub struct LaClient;

impl LaClient {
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
        let resp_res = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("User-Agent", LA_USER_AGENT)
            .send_json(&body);

        let body_str = match resp_res {
            Ok(resp) => resp.into_string().map_err(|e| format!("读取登录响应失败: {e}"))?,
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                return Err(Self::parse_error_response(code, &err_body));
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
        let resp_res = ureq::get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .call();

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
            let tier = user_val.get("tier").and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| match member_level {
                    2 => "ultra".to_string(),
                    1 => "pro".to_string(),
                    _ => "free".to_string(),
                });
            let avatar_url = user_val.get("avatar_url").and_then(|v| v.as_str()).map(|s| s.to_string());

            (session_id, device_id, LaUser {
                id: uid,
                username: uname,
                member_level,
                tier,
                avatar_url,
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
        let resp_res = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareRefresh {refresh_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .call();

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

        let resp_res = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .send_json(&body);

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
        let resp_res = ureq::get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .call();

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

        let resp_res = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .send_json(&body);

        match resp_res {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, resp)) => {
                let err_body = resp.into_string().unwrap_or_default();
                Err(Self::parse_error_response(code, &err_body))
            }
            Err(e) => Err(format!("绑定账号失败: {e}")),
        }
    }

    /// 查询当前绑定的设备列表
    pub fn list_devices(access_token: &str) -> Result<Vec<LaDevice>, String> {
        let url = format!("{LA_BASE_URL}/client-auth/account/devices");
        let resp_res = ureq::get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .call();

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
        let resp_res = ureq::delete(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .call();

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
        let _ = ureq::post(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .set("Accept", "application/json")
            .set("Authorization", &format!("SoftwareAccess {access_token}"))
            .set("User-Agent", LA_USER_AGENT)
            .call();
        Self::clear_session();
    }
}
