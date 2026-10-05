use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use serde_json::json;

use crate::telemetry::crash_handler::{archive_crash, list_pending_crashes};

static PIPELINE_STARTED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub fn reset_pipeline_started_for_test() {
    PIPELINE_STARTED.store(false, Ordering::SeqCst);
}

/// 启动遥测与崩溃补报后台流水线
pub fn start_telemetry_pipeline() {
    if PIPELINE_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let spawn_res = std::thread::Builder::new()
        .name("telemetry-worker".into())
        .spawn(move || {
            let _ = std::panic::catch_unwind(|| {
                run_telemetry_pipeline();
            });
        });

    if let Err(e) = spawn_res {
        eprintln!("[Telemetry] 启动 telemetry-worker 线程失败: {e}");
    }
}

pub fn run_telemetry_pipeline() {
    run_telemetry_pipeline_with_url(crate::la_client::LA_BASE_URL);
}

pub fn run_telemetry_pipeline_with_url(base_url: &str) {
    // 步骤 1: 匿名启动心跳上报
    if let Err(e) = send_launch_telemetry_to(base_url) {
        eprintln!("[Telemetry] 匿名启动心跳上报失败 (已静默记录): {e}");
    }

    // 步骤 2: 扫描并补报离线崩溃与错误日志
    if let Err(e) = report_pending_crashes_to(base_url) {
        eprintln!("[Telemetry] 离线崩溃补报失败 (已静默记录): {e}");
    }
}

pub fn send_launch_telemetry_to(base_url: &str) -> Result<(), String> {
    let payload = json!({
        "installation_id": crate::utils::device_info::get_installation_id(),
        "platform_device_id": crate::utils::device_info::get_platform_device_id(),
        "client_key": crate::la_client::LA_CLIENT_KEY,
        "app_version": env!("CARGO_PKG_VERSION"),
        "app_version_code": crate::la_client::LA_APP_VERSION_CODE,
        "os_platform": "windows",
        "os_version": crate::utils::device_info::get_platform_version(),
        "device_name": crate::utils::device_info::get_device_name(),
    });

    let url = format!("{base_url}/client-telemetry/launch");
    let req = ureq::post(&url)
        .timeout(Duration::from_secs(5))
        .set("Accept", "application/json")
        .set("Content-Type", "application/json");

    let req = crate::la_client::apply_common_headers(req);

    match req.send_json(&payload) {
        Ok(resp) => {
            let status = resp.status();
            if (200..300).contains(&status) {
                Ok(())
            } else {
                Err(format!("服务端响应非成功状态码: {status}"))
            }
        }
        Err(ureq::Error::Status(code, _)) => Err(format!("服务端返回 HTTP {code}")),
        Err(e) => Err(format!("心跳请求通信异常: {e}")),
    }
}

pub fn report_pending_crashes_to(base_url: &str) -> Result<usize, String> {
    let pending = list_pending_crashes();
    if pending.is_empty() {
        return Ok(0);
    }

    let url = format!("{base_url}/client-telemetry/errors");
    let mut success_count = 0;

    for (path, report) in pending {
        let req = ureq::post(&url)
            .timeout(Duration::from_secs(5))
            .set("Accept", "application/json")
            .set("Content-Type", "application/json");

        let req = crate::la_client::apply_common_headers(req);

        match req.send_json(&report) {
            Ok(resp) => {
                let status = resp.status();
                if status == 200 || status == 201 {
                    archive_crash(&path);
                    success_count += 1;
                } else {
                    eprintln!(
                        "[Telemetry] 崩溃日志 {} 上报响应状态码为 {}, 保留本地待下次重试",
                        report.report_id, status
                    );
                }
            }
            Err(ureq::Error::Status(code, _)) => {
                if code == 200 || code == 201 {
                    archive_crash(&path);
                    success_count += 1;
                } else {
                    eprintln!(
                        "[Telemetry] 崩溃日志 {} 上报返回 HTTP {}, 保留本地待下次重试",
                        report.report_id, code
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "[Telemetry] 崩溃日志 {} 上报发生网络异常: {}, 中断本轮重试",
                    report.report_id, e
                );
                // 网络断开或不可达，保留本地所有待上报文件，中断当前轮次循环
                break;
            }
        }
    }

    Ok(success_count)
}
