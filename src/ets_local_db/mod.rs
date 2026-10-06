use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use base64::Engine;

/// 本地 ETS.db 账号信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EtsLocalAccount {
    pub account: String,               // 手机号或卡号 (User.Account)
    pub name: String,                  // 真实姓名 (User.Name)
    pub cover_url: String,             // 头像 URL
    pub token: String,                 // 会话 Token (JWT)
    pub password_plain: Option<String>,// 解密后的明文登录密码
    pub cur_res_item_id: String,       // 当前选中的资源包 ID (对应 ResourceList.ID / parent_account_id)
    
    // 关联的 ResourceList 学生卡与学籍信息
    pub student_card_no: String,       // 学生学号/卡号 (如 BJ170163553G)
    pub student_card_pwd: String,      // 学生卡直连明文密码 (如 77982088)
    pub school_name: String,           // 学校名称 (如 北京市十一学校顺义学校)
    pub class_name: String,            // 班级 (如 5)
    pub teacher_name: String,          // 老师姓名
    pub expiry_time: String,           // 服务有效期 (如 2027-07-31 21:14:16)
    pub grade_name: String,            // 学段年级 (如 高一)
}

/// 核心逆向算法：解密 Ets.exe 保存的密码密文
///
/// 官方采用固化密钥 `Vm0xMFlWbFdWWGhVYmxKWFltdHdVRlpzV21GWFJscHlWbTVLVUZWVU1Eaz0=`，
/// 密文以 `YFgES393` 开头，截取第 8 字符后进行 Base64 解码并与固化密钥逐字节异或。
pub fn decrypt_ets_password(cipher: &str) -> Option<String> {
    const XOR_KEY: &[u8] = b"Vm0xMFlWbFdWWGhVYmxKWFltdHdVRlpzV21GWFJscHlWbTVLVUZWVU1Eaz0=";
    const PREFIX: &str = "YFgES393";

    let trimmed = cipher.trim();
    if !trimmed.starts_with(PREFIX) {
        return None;
    }

    let sub = &trimmed[PREFIX.len()..];
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(sub)
        .ok()?;

    let mut plaintext = Vec::with_capacity(decoded_bytes.len());
    for (i, &b) in decoded_bytes.iter().enumerate() {
        let k = XOR_KEY[i % XOR_KEY.len()];
        plaintext.push(b ^ k);
    }

    String::from_utf8(plaintext).ok()
}

/// 多路径层叠自动探测本地 ETS.db 路径
pub fn detect_db_path() -> Option<PathBuf> {
    // 0. 支持环境变量显式指定 (用于自定义测试或排查)
    if let Ok(p) = std::env::var("FEPC_ETS_DB_PATH") {
        if !p.trim().is_empty() {
            let pb = PathBuf::from(p.trim());
            if pb.exists() && pb.is_file() {
                return Some(pb);
            }
        }
    }

    let mut candidates = Vec::new();

    // 1. 优先检测当前运行程序同级或上级目录的 ETS 安装目录
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("localdata").join("ETS.db"));
            candidates.push(exe_dir.join("ETS").join("localdata").join("ETS.db"));
            if let Some(parent) = exe_dir.parent() {
                candidates.push(parent.join("ETS").join("localdata").join("ETS.db"));
                candidates.push(parent.join("localdata").join("ETS.db"));
            }
        }
    }

    // 2. 当前工作目录及相对路径
    candidates.push(PathBuf::from(r"ETS\localdata\ETS.db"));
    candidates.push(PathBuf::from(r"..\ETS\localdata\ETS.db"));
    candidates.push(PathBuf::from(r"localdata\ETS.db"));

    // 3. 用户 AppData 与 LocalAppData 目录 (官方 e听说 实际持久化数据目录)
    if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(app_data).join("ETS").join("localdata").join("ETS.db"));
    }
    if let Ok(app_data) = std::env::var("APPDATA") {
        candidates.push(PathBuf::from(app_data).join("ETS").join("localdata").join("ETS.db"));
    }

    // 4. 常见盘符安装路径 (C/D/E/F/G)
    for drive in &["C", "D", "E", "F", "G"] {
        candidates.push(PathBuf::from(format!(r"{drive}:\ETS\localdata\ETS.db")));
        candidates.push(PathBuf::from(format!(r"{drive}:\Program Files (x86)\ETS\localdata\ETS.db")));
        candidates.push(PathBuf::from(format!(r"{drive}:\Program Files\ETS\localdata\ETS.db")));
    }

    #[cfg(debug_assertions)]
    candidates.push(PathBuf::from(r"F:\fepc\ETS\localdata\ETS.db"));

    for p in candidates {
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }

    None
}

/// 辅助函数：尝试将 SQLite 中的 BLOB/TEXT 字节安全转换为 UTF-8（支持 UTF-8 与 GBK/GB18030 回退）
fn decode_db_string(raw: &[u8]) -> String {
    if raw.is_empty() {
        return String::new();
    }
    if let Ok(s) = std::str::from_utf8(raw) {
        return s.to_string();
    }
    let (cow, _, _) = encoding_rs::GB18030.decode(raw);
    cow.to_string()
}

fn get_cell_bytes(row: &rusqlite::Row, idx: usize) -> Vec<u8> {
    use rusqlite::types::ValueRef;
    match row.get_ref(idx) {
        Ok(ValueRef::Text(s)) => s.to_vec(),
        Ok(ValueRef::Blob(b)) => b.to_vec(),
        Ok(ValueRef::Integer(i)) => i.to_string().into_bytes(),
        Ok(ValueRef::Real(f)) => f.to_string().into_bytes(),
        _ => Vec::new(),
    }
}

/// 从 ETS.db 读取所有历史已存账号并关联学籍信息
pub fn read_local_accounts(db_path: &Path) -> Result<Vec<EtsLocalAccount>, String> {
    if !db_path.exists() {
        return Err(format!("数据库文件不存在: {}", db_path.display()));
    }

    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).map_err(|e| format!("打开数据库失败: {e}"))?;

    // 1. 读取 User 表
    let mut stmt = conn.prepare(
        "SELECT Account, Name, Cover, Token, Password, CurResItemID FROM [User]"
    ).map_err(|e| format!("查询 User 表失败: {e}"))?;

    let user_rows = stmt.query_map([], |row| {
        Ok((
            decode_db_string(&get_cell_bytes(row, 0)),
            decode_db_string(&get_cell_bytes(row, 1)),
            decode_db_string(&get_cell_bytes(row, 2)),
            decode_db_string(&get_cell_bytes(row, 3)),
            decode_db_string(&get_cell_bytes(row, 4)),
            decode_db_string(&get_cell_bytes(row, 5)),
        ))
    }).map_err(|e| format!("执行 User 查询失败: {e}"))?;

    let mut accounts = Vec::new();

    // 2. 读取 ResourceList 表构建索引
    let mut res_map = std::collections::HashMap::new();
    if let Ok(mut res_stmt) = conn.prepare(
        "SELECT ID, Account, Password, StudentName, SchoolName, ClassName, TeacherFirstName, ExpiryTime, Grade FROM [ResourceList]"
    ) {
        if let Ok(res_rows) = res_stmt.query_map([], |row| {
            Ok((
                decode_db_string(&get_cell_bytes(row, 0)),
                decode_db_string(&get_cell_bytes(row, 1)),
                decode_db_string(&get_cell_bytes(row, 2)),
                decode_db_string(&get_cell_bytes(row, 3)),
                decode_db_string(&get_cell_bytes(row, 4)),
                decode_db_string(&get_cell_bytes(row, 5)),
                decode_db_string(&get_cell_bytes(row, 6)),
                decode_db_string(&get_cell_bytes(row, 7)),
                decode_db_string(&get_cell_bytes(row, 8)),
            ))
        }) {
            for r in res_rows.flatten() {
                res_map.insert(r.0.clone(), r);
            }
        }
    }

    for u in user_rows.flatten() {
        let (account, name, cover, token, pwd, cur_res) = u;
        if account.trim().is_empty() && token.trim().is_empty() {
            continue;
        }

        let plain_pwd = decrypt_ets_password(&pwd);

        let mut acc = EtsLocalAccount {
            account: account.clone(),
            name,
            cover_url: cover,
            token,
            password_plain: plain_pwd,
            cur_res_item_id: cur_res.clone(),
            ..Default::default()
        };

        // 匹配关联的学生卡与学籍信息 (严格通过 CurResItemID 关联，未匹配绝不臆造或随意跨账号挪用)
        if !cur_res.trim().is_empty() {
            if let Some(res) = res_map.get(&cur_res) {
                acc.student_card_no = res.1.clone();
                acc.student_card_pwd = res.2.clone();
                if acc.name.is_empty() {
                    acc.name = res.3.clone();
                }
                acc.school_name = res.4.clone();
                acc.class_name = res.5.clone();
                acc.teacher_name = res.6.clone();
                acc.expiry_time = res.7.clone();
                acc.grade_name = res.8.clone();
            }
        }

        accounts.push(acc);
    }

    Ok(accounts)
}

/// 辅助函数：将 Unix 时间戳格式化为可读时间字符串 (YYYY-MM-DD HH:MM:SS，UTC+8 北京时间)
pub fn format_timestamp(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let ts_cst = ts + 8 * 3600;
    let days = ts_cst / 86400;
    let rem_secs = (ts_cst % 86400) as u32;
    let hours = rem_secs / 3600;
    let minutes = (rem_secs % 3600) / 60;
    let seconds = rem_secs % 60;

    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, hours, minutes, seconds)
}

/// 从 ETS.db 的 HomeworkList 表读取所有离线作业记录
pub fn read_cached_homeworks(db_path: &Path) -> Result<Vec<crate::models::HomeworkItem>, String> {
    if !db_path.exists() {
        return Err(format!("数据库文件不存在: {}", db_path.display()));
    }

    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).map_err(|e| format!("打开数据库失败: {e}"))?;

    let mut stmt = conn.prepare(
        "SELECT HomeworkID, Name, Begin, End, TotalPoint, Point, IsFinished, Set_ID, SetType, pk FROM [HomeworkList]"
    ).map_err(|e| format!("查询 HomeworkList 表失败: {e}"))?;

    let rows = stmt.query_map([], |row| {
        let hw_id = decode_db_string(&get_cell_bytes(row, 0));
        let name = decode_db_string(&get_cell_bytes(row, 1));
        let _begin: i64 = row.get(2).unwrap_or(0);
        let end: i64 = row.get(3).unwrap_or(0);
        let total_point: f64 = row.get(4).unwrap_or(100.0);
        let point: f64 = row.get(5).unwrap_or(0.0);
        let is_finished: i64 = row.get(6).unwrap_or(0);
        let set_id = decode_db_string(&get_cell_bytes(row, 7));
        let set_type: i64 = row.get(8).unwrap_or(0);
        let pk: i64 = row.get(9).unwrap_or(0);

        let deadline = if end > 0 {
            format_timestamp(end)
        } else {
            String::new()
        };

        let status = if is_finished == 1 {
            "已完成 (离线缓存)".to_string()
        } else {
            "未完成 (离线缓存)".to_string()
        };

        let awarded_point = if is_finished == 1 || point > 0.0 {
            Some(point)
        } else {
            None
        };

        Ok(crate::models::HomeworkItem {
            id: hw_id,
            name,
            status,
            deadline,
            total_point,
            awarded_point,
            question_count: 0,
            source: crate::models::HomeworkSource::Local,
            local_dir: Some(db_path.to_string_lossy().to_string()),
            resource_urls: Vec::new(),
            sections: Vec::new(),
            set_type,
            pk,
            resource_id: set_id,
        })
    }).map_err(|e| format!("执行 HomeworkList 查询失败: {e}"))?;

    let mut items = Vec::new();
    for r in rows.flatten() {
        if !r.id.is_empty() {
            items.push(r);
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_decryption_real_samples() {
        assert_eq!(
            decrypt_ets_password("YFgES393ZF0BSHx0XGY=").as_deref(),
            Some("20101201")
        );
        assert_eq!(
            decrypt_ets_password("YFgES393MBhTEygyH2ZSdg==").as_deref(),
            Some("fuckets100")
        );
        assert_eq!(
            decrypt_ets_password("YFgES393LAVVFipwXmFb").as_deref(),
            Some("zheng6269")
        );
        assert_eq!(
            decrypt_ets_password("YFgES393EAFFHX90VGZRaA==").as_deref(),
            Some("Flue22813.")
        );
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "");
        assert_eq!(format_timestamp(-100), "");
        // 1790255700: 2026-09-24 13:15:00 UTC -> 2026-09-24 21:15:00 CST
        let formatted = format_timestamp(1790255700);
        assert_eq!(formatted, "2026-09-24 21:15:00");
    }

    #[test]
    fn test_read_real_ets_db() {
        let path = PathBuf::from(r"F:\fepc\ETS\localdata\ETS.db");
        if path.exists() {
            let accounts = read_local_accounts(&path).expect("读取 ETS.db 账号失败");
            assert!(!accounts.is_empty(), "账号列表不应为空");
            for a in &accounts {
                println!("Account: {} ({}) -> card: '{}', school: '{}'", a.account, a.name, a.student_card_no, a.school_name);
            }

            // 严格断言：未在 ResourceList 绑定学生卡的账号，卡号与学校必须为空，绝不能挪用他人学生卡
            if let Some(tang) = accounts.iter().find(|a| a.account.contains("1500000100220922169")) {
                assert!(tang.student_card_no.is_empty(), "唐浩泽未绑定当前学生卡，卡号必须为空！");
                assert!(tang.school_name.is_empty(), "唐浩泽学校信息必须为空！");
            }
            if let Some(lastudio) = accounts.iter().find(|a| a.account.contains("18518811167")) {
                assert!(lastudio.student_card_no.is_empty(), "lastudio 未绑定当前学生卡，卡号必须为空！");
            }
            if let Some(ruban) = accounts.iter().find(|a| a.account.contains("13377673189")) {
                assert!(ruban.student_card_no.is_empty(), "鲁班奖未绑定当前学生卡，卡号必须为空！");
            }
            // 真实命中 272200541 的账号必须包含正确的学生卡信息
            if let Some(lei) = accounts.iter().find(|a| a.account == "13552295672") {
                assert_eq!(lei.student_card_no, "BJ170163553G");
                assert_eq!(lei.school_name, "北京市十一学校顺义学校");
            }

        }
    }

    #[test]
    fn test_read_cached_homeworks_real() {
        let path = PathBuf::from(r"F:\fepc\ETS\localdata\ETS.db");
        if path.exists() {
            let homeworks = read_cached_homeworks(&path).expect("读取 ETS.db 离线作业失败");
            assert_eq!(homeworks.len(), 5, "ETS.db 中应该包含 5 项离线作业");
            for hw in &homeworks {
                assert!(!hw.id.is_empty(), "作业 ID 不应为空");
                assert!(!hw.name.is_empty(), "作业名称不应为空");
                assert!(hw.status.contains("离线缓存"), "状态应标记为离线缓存");
                assert_eq!(hw.source, crate::models::HomeworkSource::Local);
            }
            assert_eq!(homeworks[0].id, "106834354");
            assert!(homeworks[0].name.contains("校园生活"));
        }
    }

    #[test]
    fn test_mock_read_cached_homeworks() {
        // 使用临时内存 SQLite 数据库验证无真实 ETS.db 场景下的纯逻辑准确性
        let dir = std::env::temp_dir().join(format!("test_ets_db_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&dir);
        let db_file = dir.join("ETS.db");

        {
            let conn = rusqlite::Connection::open(&db_file).unwrap();
            conn.execute_batch(
                "CREATE TABLE [HomeworkList] (
                    UserNo VARCHAR, HomeworkID VARCHAR, Name VARCHAR, Begin INT, End INT,
                    TotalPoint DOUBLE, Point DOUBLE, IsFinished INT, Set_ID VARCHAR,
                    SetType INT, pk INT
                );
                INSERT INTO [HomeworkList] VALUES (
                    'U1001', 'HW_TEST_1', 'Unit 1 Mock Homework', 1700000000, 1700086400,
                    100.0, 95.5, 1, 'SET_999', 1, 0
                );"
            ).unwrap();
        }

        // 验证 read_cached_homeworks
        let hws = read_cached_homeworks(&db_file).expect("读取 Mock 离线作业失败");
        assert_eq!(hws.len(), 1);
        assert_eq!(hws[0].id, "HW_TEST_1");
        assert_eq!(hws[0].name, "Unit 1 Mock Homework");
        assert_eq!(hws[0].status, "已完成 (离线缓存)");
        assert_eq!(hws[0].awarded_point, Some(95.5));
        assert_eq!(hws[0].resource_id, "SET_999");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

