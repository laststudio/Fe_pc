use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const APPLICATION_ID: &str = "cc.lastudio.fepc";
pub const SIGNING_CERT_SHA256: &str = "79fe6f92aa7f46084832cd62292c223101586b46e3cfaf39740fef1c6253aaca";
pub const VERSION_CODE: i64 = 105;

#[derive(Serialize, Deserialize, Default)]
struct SavedDeviceInfo {
    pub installation_id: Option<String>,
}

fn get_storage_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let dir = PathBuf::from(app_data).join("fepc");
        let _ = fs::create_dir_all(&dir);
        return dir.join("device_info.json");
    }
    PathBuf::from("device_info.json")
}

pub fn get_installation_id() -> String {
    let path = get_storage_path();
    if path.exists()
        && let Ok(content) = fs::read_to_string(&path)
            && let Ok(info) = serde_json::from_str::<SavedDeviceInfo>(&content)
                && let Some(id) = info.installation_id
                    && !id.trim().is_empty() {
                        return id;
                    }

    let new_id = Uuid::new_v4().to_string();
    let info = SavedDeviceInfo {
        installation_id: Some(new_id.clone()),
    };
    if let Ok(content) = serde_json::to_string_pretty(&info) {
        let _ = fs::write(path, content);
    }
    new_id
}

#[inline(never)]
pub fn get_platform_device_id() -> String {
    let raw_id = get_raw_machine_guid().unwrap_or_else(|| {
        let comp = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "DESKTOP".to_string());
        let domain = std::env::var("USERDOMAIN").unwrap_or_default();
        let proc = std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default();
        format!("{comp}_{domain}_{proc}")
    });

    crate::themida_lion_white!({
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", APPLICATION_ID, raw_id).as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    })
}

fn get_registry_value_sz(subkey_str: &str, value_name_str: &str) -> Option<String> {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
        };

        unsafe {
            let mut subkey: Vec<u16> = subkey_str.encode_utf16().collect();
            subkey.push(0);
            let mut val_name: Vec<u16> = value_name_str.encode_utf16().collect();
            val_name.push(0);
            let mut hkey = windows::Win32::System::Registry::HKEY::default();

            if RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(subkey.as_ptr()),
                Some(0),
                KEY_READ,
                &mut hkey,
            )
            .is_ok()
            {
                let mut buffer = [0u16; 128];
                let mut buf_len = (buffer.len() * std::mem::size_of::<u16>()) as u32;
                let mut val_type = REG_SZ;

                let res = RegQueryValueExW(
                    hkey,
                    PCWSTR(val_name.as_ptr()),
                    None,
                    Some(&mut val_type),
                    Some(buffer.as_mut_ptr() as *mut u8),
                    Some(&mut buf_len),
                );
                let _ = RegCloseKey(hkey);

                if res.is_ok() && buf_len > 0 {
                    let u16_len = (buf_len as usize) / std::mem::size_of::<u16>();
                    let mut s = String::from_utf16_lossy(&buffer[..u16_len]);
                    if let Some(null_idx) = s.find('\0') {
                        s.truncate(null_idx);
                    }
                    if !s.trim().is_empty() {
                        return Some(s.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

fn get_raw_machine_guid() -> Option<String> {
    get_registry_value_sz("SOFTWARE\\Microsoft\\Cryptography", "MachineGuid")
}

pub fn get_device_name() -> String {
    let comp = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Desktop".to_string());
    format!("PC-{comp}")
}

pub fn get_platform_version() -> String {
    #[cfg(windows)]
    {
        if let Some(build_str) = get_registry_value_sz("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", "CurrentBuild")
            && let Ok(build_num) = build_str.parse::<u32>()
        {
            if build_num >= 22000 {
                return format!("Windows 11 (Build {build_num})");
            } else {
                return format!("Windows 10 (Build {build_num})");
            }
        }
        "Windows 11 (x64)".to_string()
    }
    #[cfg(not(windows))]
    {
        "Windows 11 (x64)".to_string()
    }
}

pub fn get_device_payload() -> Value {
    json!({
        "installation_id": get_installation_id(),
        "platform_device_id": get_platform_device_id(),
        "application_id": APPLICATION_ID,
        "signing_certificate_sha256": SIGNING_CERT_SHA256,
        "version_code": VERSION_CODE,
        "device_name": get_device_name(),
        "platform": "windows",
        "platform_version": get_platform_version(),
    })
}
