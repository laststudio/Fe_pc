use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrashReport {
    pub report_id: String,
    pub occurred_at: String, // ISO8601
    pub error_type: String,  // "panic", "runtime_error", etc.
    pub error_message: String,
    pub error_location: String,
    pub stack_trace: String,
    pub installation_id: String,
    pub platform_device_id: String,
    pub client_key: String,
    pub app_version: String,
    pub app_version_code: i64,
    pub os_platform: String,
    pub os_version: String,
    pub device_name: String,
}

/// Convert unix epoch seconds into ISO8601 UTC string (YYYY-MM-DDTHH:MM:SSZ)
pub fn epoch_seconds_to_iso8601(secs: u64) -> String {
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let mut days = (secs / 86400) as i64;

    // Howard Hinnant algorithm
    days += 719468;
    let era = (if days >= 0 { days } else { days - 146096 }) / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m_month = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_year = if m_month <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y_year, m_month, d, h, m, s)
}

pub fn current_iso8601() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    epoch_seconds_to_iso8601(secs)
}

pub fn get_crashes_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("FEPC_CRASHES_DIR")
        && !custom.trim().is_empty()
    {
        let dir = PathBuf::from(custom);
        let _ = fs::create_dir_all(&dir);
        return dir;
    }
    if let Ok(app_data) = std::env::var("APPDATA")
        && !app_data.trim().is_empty()
    {
        let dir = PathBuf::from(app_data).join("fepc").join("crashes");
        let _ = fs::create_dir_all(&dir);
        return dir;
    }
    let dir = PathBuf::from("crashes");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_archived_crashes_dir() -> PathBuf {
    let dir = get_crashes_dir().join("archived");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn save_crash_to_disk(report: &CrashReport) -> Result<PathBuf, String> {
    let crashes_dir = get_crashes_dir();
    let file_name = format!("crash_{}.json", report.report_id);
    let tmp_name = format!("crash_{}.tmp", report.report_id);
    let final_path = crashes_dir.join(&file_name);
    let tmp_path = crashes_dir.join(&tmp_name);

    let json_bytes = serde_json::to_vec_pretty(report)
        .map_err(|e| format!("序列化崩溃日志失败: {e}"))?;

    fs::write(&tmp_path, &json_bytes)
        .map_err(|e| format!("写入临时崩溃文件失败: {e}"))?;

    if fs::rename(&tmp_path, &final_path).is_err() {
        fs::write(&final_path, &json_bytes)
            .map_err(|e| format!("写入最终崩溃文件失败: {e}"))?;
        let _ = fs::remove_file(&tmp_path);
    }

    Ok(final_path)
}

pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let error_message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<Any> panic payload".to_string()
        };

        let error_location = if let Some(loc) = panic_info.location() {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        } else {
            "unknown location".to_string()
        };

        let stack_trace = std::backtrace::Backtrace::force_capture().to_string();

        let report = CrashReport {
            report_id: uuid::Uuid::new_v4().to_string(),
            occurred_at: current_iso8601(),
            error_type: "panic".to_string(),
            error_message,
            error_location,
            stack_trace,
            installation_id: crate::utils::device_info::get_installation_id(),
            platform_device_id: crate::utils::device_info::get_platform_device_id(),
            client_key: crate::la_client::LA_CLIENT_KEY.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            app_version_code: crate::la_client::LA_APP_VERSION_CODE,
            os_platform: "windows".to_string(),
            os_version: crate::utils::device_info::get_platform_version(),
            device_name: crate::utils::device_info::get_device_name(),
        };

        let _ = save_crash_to_disk(&report);

        default_hook(panic_info);
    }));
}

pub fn record_runtime_error(
    error_type: &str,
    error_message: &str,
    location: &str,
    stack_trace: Option<&str>,
) {
    let captured_stack = stack_trace
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::backtrace::Backtrace::force_capture().to_string());

    let report = CrashReport {
        report_id: uuid::Uuid::new_v4().to_string(),
        occurred_at: current_iso8601(),
        error_type: error_type.to_string(),
        error_message: error_message.to_string(),
        error_location: location.to_string(),
        stack_trace: captured_stack,
        installation_id: crate::utils::device_info::get_installation_id(),
        platform_device_id: crate::utils::device_info::get_platform_device_id(),
        client_key: crate::la_client::LA_CLIENT_KEY.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        app_version_code: crate::la_client::LA_APP_VERSION_CODE,
        os_platform: "windows".to_string(),
        os_version: crate::utils::device_info::get_platform_version(),
        device_name: crate::utils::device_info::get_device_name(),
    };

    let _ = save_crash_to_disk(&report);
}

pub fn list_pending_crashes() -> Vec<(PathBuf, CrashReport)> {
    let dir = get_crashes_dir();
    let mut list = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return list;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && ext == "json"
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(report) = serde_json::from_str::<CrashReport>(&content)
        {
            list.push((path, report));
        }
    }

    list.sort_by(|a, b| a.1.occurred_at.cmp(&b.1.occurred_at));
    list
}

pub fn archive_crash(path: &Path) {
    let archived_dir = get_archived_crashes_dir();
    let file_name = match path.file_name() {
        Some(name) => name,
        None => return,
    };
    let dest_path = archived_dir.join(file_name);

    if path != dest_path && path.exists() {
        if dest_path.exists() {
            let _ = fs::remove_file(&dest_path);
        }
        if fs::rename(path, &dest_path).is_err()
            && fs::copy(path, &dest_path).is_ok()
        {
            let _ = fs::remove_file(path);
        }
    }

    prune_archived_crashes(&archived_dir, 20);
}

fn prune_archived_crashes(archived_dir: &Path, max_keep: usize) {
    let Ok(entries) = fs::read_dir(archived_dir) else {
        return;
    };

    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            files.push((p, modified));
        }
    }

    if files.len() > max_keep {
        files.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let remove_count = files.len() - max_keep;
        for (to_del, _) in files.iter().take(remove_count) {
            let _ = fs::remove_file(to_del);
        }
    }
}
