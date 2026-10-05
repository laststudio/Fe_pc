pub mod crash_handler;
pub mod reporter;

pub use crash_handler::{
    archive_crash, current_iso8601, epoch_seconds_to_iso8601, get_archived_crashes_dir,
    get_crashes_dir, install_panic_hook, list_pending_crashes, record_runtime_error,
    save_crash_to_disk, CrashReport,
};
pub use reporter::{
    report_pending_crashes_to, run_telemetry_pipeline, run_telemetry_pipeline_with_url,
    send_launch_telemetry_to, start_telemetry_pipeline,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn create_test_crashes_dir() -> (PathBuf, String) {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Uuid::new_v4();
        let path = std::env::temp_dir().join(format!("fepc_test_crashes_{}_{}", n, id));
        let _ = fs::create_dir_all(&path);
        let key = format!("FEPC_CRASHES_DIR_{}_{}", n, id);
        (path, key)
    }

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct TempEnvGuard {
        dir: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TempEnvGuard {
        fn new() -> Self {
            let lock = TEST_MUTEX.lock().unwrap();
            let (dir, _) = create_test_crashes_dir();
            unsafe {
                std::env::set_var("FEPC_CRASHES_DIR", &dir);
            }
            Self { dir, _lock: lock }
        }
    }

    impl Drop for TempEnvGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("FEPC_CRASHES_DIR");
            }
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn test_crash_report_serialization_and_deserialization() {
        let report = CrashReport {
            report_id: "test-report-uuid-1234".to_string(),
            occurred_at: "2026-10-05T06:00:00Z".to_string(),
            error_type: "panic".to_string(),
            error_message: "assertion failed: x == y".to_string(),
            error_location: "src/telemetry/tests.rs:10:5".to_string(),
            stack_trace: "backtrace line 1\nbacktrace line 2".to_string(),
            installation_id: "inst-999".to_string(),
            platform_device_id: "hw-guid-888".to_string(),
            client_key: "fepc-windows".to_string(),
            app_version: "0.1.6".to_string(),
            app_version_code: 106,
            os_platform: "windows".to_string(),
            os_version: "Windows 11 (Build 22631)".to_string(),
            device_name: "PC-DESKTOP-TEST".to_string(),
        };

        let json_str = serde_json::to_string_pretty(&report).expect("Serialize CrashReport failed");
        assert!(json_str.contains("\"report_id\": \"test-report-uuid-1234\""));
        assert!(json_str.contains("\"occurred_at\": \"2026-10-05T06:00:00Z\""));
        assert!(json_str.contains("\"error_type\": \"panic\""));
        assert!(json_str.contains("\"client_key\": \"fepc-windows\""));
        assert!(json_str.contains("\"app_version_code\": 106"));

        let deserialized: CrashReport =
            serde_json::from_str(&json_str).expect("Deserialize CrashReport failed");
        assert_eq!(report, deserialized);
    }

    #[test]
    fn test_iso8601_formatting() {
        let epoch_zero = epoch_seconds_to_iso8601(0);
        assert_eq!(epoch_zero, "1970-01-01T00:00:00Z");

        let current = current_iso8601();
        assert_eq!(current.len(), 20);
        assert!(current.ends_with('Z'));
        assert_eq!(&current[10..11], "T");
    }

    #[test]
    fn test_record_runtime_error_and_crashes_persistence() {
        let _guard = TempEnvGuard::new();

        assert_eq!(list_pending_crashes().len(), 0);

        record_runtime_error(
            "runtime_error",
            "failed to parse ets response",
            "src/cloud_client.rs:120:8",
            Some("fake stack frame 1\nfake stack frame 2"),
        );

        let pending = list_pending_crashes();
        assert_eq!(pending.len(), 1);

        let (path, report) = &pending[0];
        assert!(path.exists());
        assert_eq!(report.error_type, "runtime_error");
        assert_eq!(report.error_message, "failed to parse ets response");
        assert_eq!(report.error_location, "src/cloud_client.rs:120:8");
        assert_eq!(report.client_key, crate::la_client::LA_CLIENT_KEY);
        assert_eq!(report.app_version_code, crate::la_client::LA_APP_VERSION_CODE);

        // Archive crash
        archive_crash(path);

        // Should no longer be pending
        let pending_after = list_pending_crashes();
        assert_eq!(pending_after.len(), 0);

        // Should exist in archived
        let archived_file = get_archived_crashes_dir().join(path.file_name().unwrap());
        assert!(archived_file.exists());
    }

    #[test]
    fn test_archive_pruning_max_20() {
        let _guard = TempEnvGuard::new();

        // Create 25 crash files
        for i in 0..25 {
            let report = CrashReport {
                report_id: format!("test-prune-{i:02}"),
                occurred_at: format!("2026-10-05T06:{i:02}:00Z"),
                error_type: "panic".to_string(),
                error_message: format!("test message {i}"),
                error_location: "mod.rs:1:1".to_string(),
                stack_trace: String::new(),
                installation_id: "test".to_string(),
                platform_device_id: "test".to_string(),
                client_key: "fepc-windows".to_string(),
                app_version: "0.1.6".to_string(),
                app_version_code: 106,
                os_platform: "windows".to_string(),
                os_version: "11".to_string(),
                device_name: "test".to_string(),
            };
            let path = save_crash_to_disk(&report).expect("save crash");
            // Archive immediately
            archive_crash(&path);
            // Slight delay so modified times differ slightly if needed
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let archived_dir = get_archived_crashes_dir();
        let entries: Vec<_> = fs::read_dir(&archived_dir)
            .expect("read archived dir")
            .flatten()
            .filter(|e| e.path().is_file())
            .collect();

        assert_eq!(entries.len(), 20, "Archived directory must retain at most 20 files");
    }

    #[test]
    fn test_telemetry_pipeline_mock_server() {
        let _guard = TempEnvGuard::new();

        // Create 1 pending crash
        record_runtime_error(
            "panic",
            "assertion failed",
            "src/main.rs:1:1",
            Some("stack..."),
        );
        assert_eq!(list_pending_crashes().len(), 1);

        // Bind mock TCP listener
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let local_addr = listener.local_addr().expect("local addr");
        let base_url = format!("http://{}", local_addr);

        fn read_http_request(stream: &mut std::net::TcpStream) -> String {
            let mut data = Vec::new();
            let mut buf = [0u8; 1024];
            let mut content_length = None;

            loop {
                let n = stream.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..n]);

                if content_length.is_none()
                    && let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n")
                {
                    let header_part = String::from_utf8_lossy(&data[..pos]);
                    for line in header_part.lines() {
                        if let Some(cl) = line.to_lowercase().strip_prefix("content-length:")
                            && let Ok(len) = cl.trim().parse::<usize>()
                        {
                            content_length = Some((pos + 4, len));
                        }
                    }
                    if content_length.is_none() {
                        break;
                    }
                }

                if let Some((body_start, body_len)) = content_length
                    && data.len() >= body_start + body_len
                {
                    break;
                }
            }

            String::from_utf8_lossy(&data).to_string()
        }

        let server_thread = std::thread::spawn(move || {
            // Request 1: Heartbeat /client-telemetry/launch
            if let Ok((mut stream, _)) = listener.accept() {
                let req_text = read_http_request(&mut stream);
                assert!(req_text.contains("/client-telemetry/launch"));
                assert!(req_text.contains("X-Client-Key: fepc-windows"));
                assert!(req_text.contains("X-App-Version-Code: 106"));

                let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"success\":true}";
                let _ = stream.write_all(response.as_bytes());
            }

            // Request 2: Crash report /client-telemetry/errors
            if let Ok((mut stream, _)) = listener.accept() {
                let req_text = read_http_request(&mut stream);
                assert!(req_text.contains("/client-telemetry/errors"));
                assert!(req_text.contains("assertion failed"));

                let response = "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"success\":true}";
                let _ = stream.write_all(response.as_bytes());
            }
        });

        // Run pipeline pointing to mock server
        run_telemetry_pipeline_with_url(&base_url);

        server_thread.join().expect("server thread join");

        // Crash should have been archived!
        assert_eq!(list_pending_crashes().len(), 0);
        let archived = fs::read_dir(get_archived_crashes_dir())
            .expect("read archived")
            .flatten()
            .count();
        assert_eq!(archived, 1);
    }
}
