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

/// 客观选择题本地缓存记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalChoiceAnswer {
    pub paper_id: i64,
    pub sec_id: i64,
    pub item_id: i64,
    pub entity_id: String,
    pub order: String,
    pub answer: String,
    pub max_score: f64,
}

/// 读写填空题本地缓存记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFillBlankAnswer {
    pub ques_id: i64,
    pub comb_id: i64,
    pub content: String,
    pub answer: String,
    pub point: f64,
}

/// 本地试卷答案聚合容器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalPaperAnswers {
    pub paper_id: i64,
    pub choices: Vec<LocalChoiceAnswer>,
    pub blanks: Vec<LocalFillBlankAnswer>,
}

impl LocalPaperAnswers {
    pub fn is_empty(&self) -> bool {
        self.choices.is_empty() && self.blanks.is_empty()
    }

    pub fn total_answers(&self) -> usize {
        self.choices.len() + self.blanks.len()
    }

    /// 将本地答案融合到 HomeworkItem 中：若题目列表为空则自动生成 Sections，若已存在题目则填充空答案
    pub fn apply_to_homework(&self, hw: &mut crate::models::HomeworkItem) {
        if self.is_empty() {
            return;
        }

        if hw.sections.is_empty() {
            let mut sections = Vec::new();

            if !self.choices.is_empty() {
                let mut questions = Vec::new();
                for (idx, c) in self.choices.iter().enumerate() {
                    let num = if c.order.is_empty() {
                        (idx + 1).to_string()
                    } else {
                        c.order.clone()
                    };
                    questions.push(crate::models::ParsedQuestion {
                        index: idx,
                        number: num,
                        category: "collector.choose".to_string(),
                        prompt_text: format!("客观单选题 (Section {}, Item {})", c.sec_id, c.item_id),
                        answers: vec![c.answer.clone()],
                        important_points: vec![format!("满分: {:.1} 分", c.max_score)],
                        audio_file: None,
                        options: Vec::new(),
                        word_id: None,
                        question_id: Some(c.item_id),
                    });
                }
                sections.push(crate::models::ParsedSection {
                    section_type: "collector.choose".to_string(),
                    title: "客观选择题 (本地极速答案库)".to_string(),
                    questions,
                });
            }

            if !self.blanks.is_empty() {
                let mut questions = Vec::new();
                let base_idx = self.choices.len();
                for (idx, b) in self.blanks.iter().enumerate() {
                    questions.push(crate::models::ParsedQuestion {
                        index: base_idx + idx,
                        number: (idx + 1).to_string(),
                        category: "collector.fill".to_string(),
                        prompt_text: if b.content.is_empty() { "读写填空题".to_string() } else { b.content.clone() },
                        answers: vec![b.answer.clone()],
                        important_points: vec![format!("分值: {:.1} 分", b.point)],
                        audio_file: None,
                        options: Vec::new(),
                        word_id: None,
                        question_id: Some(b.ques_id),
                    });
                }
                sections.push(crate::models::ParsedSection {
                    section_type: "collector.fill".to_string(),
                    title: "读写填空题 (本地极速答案库)".to_string(),
                    questions,
                });
            }

            hw.sections = sections;
            hw.question_count = hw.total_questions();
            if !hw.status.contains("已完成") || hw.status.is_empty() || hw.status.contains("已保存") {
                hw.status = "已匹配本地答案".to_string();
            }
        } else {
            // 已存在题目，进行答案回填
            let mut choice_idx = 0;
            let mut blank_idx = 0;
            for section in &mut hw.sections {
                for q in &mut section.questions {
                    if q.answers.is_empty() {
                        if q.category.contains("choose") || q.category.contains("choice") {
                            if let Some(c) = self.choices.iter().find(|c| c.order == q.number) {
                                q.answers.push(c.answer.clone());
                            } else if choice_idx < self.choices.len() {
                                q.answers.push(self.choices[choice_idx].answer.clone());
                                choice_idx += 1;
                            }
                        } else if q.category.contains("fill") || q.category.contains("blank") {
                            if blank_idx < self.blanks.len() {
                                q.answers.push(self.blanks[blank_idx].answer.clone());
                                blank_idx += 1;
                            }
                        }
                    }
                }
            }
        }
    }
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
    let mut candidates = Vec::new();

    // 1. 当前工程环境或相对路径
    candidates.push(PathBuf::from(r"F:\fepc\ETS\localdata\ETS.db"));
    candidates.push(PathBuf::from(r"ETS\localdata\ETS.db"));
    candidates.push(PathBuf::from(r"..\ETS\localdata\ETS.db"));

    // 2. 常见盘符安装路径
    for drive in &["C", "D", "E", "F", "G"] {
        candidates.push(PathBuf::from(format!(r"{drive}:\ETS\localdata\ETS.db")));
        candidates.push(PathBuf::from(format!(r"{drive}:\Program Files (x86)\ETS\localdata\ETS.db")));
        candidates.push(PathBuf::from(format!(r"{drive}:\Program Files\ETS\localdata\ETS.db")));
    }

    // 3. 用户 AppData 目录
    if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(app_data).join("ETS").join("localdata").join("ETS.db"));
    }
    if let Ok(app_data) = std::env::var("APPDATA") {
        candidates.push(PathBuf::from(app_data).join("ETS").join("localdata").join("ETS.db"));
    }

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

/// 查询本地缓存的客观选择题答案
pub fn query_choice_answers(db_path: &Path, paper_id: i64) -> Result<Vec<LocalChoiceAnswer>, String> {
    if !db_path.exists() {
        return Err("数据库不存在".to_string());
    }

    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT PaperID, SecID, ItemID, EntityID, [Order], Answer, MaxScore FROM [ScoreForChoose] WHERE PaperID = ?1"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([paper_id], |row| {
        let p_id: i64 = row.get(0).unwrap_or(0);
        let s_id: i64 = row.get(1).unwrap_or(0);
        let i_id: i64 = row.get(2).unwrap_or(0);
        let entity = decode_db_string(&get_cell_bytes(row, 3));
        let order = decode_db_string(&get_cell_bytes(row, 4));
        let ans = decode_db_string(&get_cell_bytes(row, 5));
        let max_score: f64 = row.get(6).unwrap_or(0.0);

        Ok(LocalChoiceAnswer {
            paper_id: p_id,
            sec_id: s_id,
            item_id: i_id,
            entity_id: entity,
            order,
            answer: ans,
            max_score,
        })
    }).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for r in rows.flatten() {
        results.push(r);
    }

    Ok(results)
}

/// 查询本地缓存的读写填空题答案
pub fn query_blank_answers(db_path: &Path, comb_id: i64) -> Result<Vec<LocalFillBlankAnswer>, String> {
    if !db_path.exists() {
        return Err("数据库不存在".to_string());
    }

    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT QuesID, CombID, Content, Answer, Point FROM [RW_Info] WHERE CombID = ?1"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([comb_id], |row| {
        let q_id: i64 = row.get(0).unwrap_or(0);
        let c_id: i64 = row.get(1).unwrap_or(0);
        let content = decode_db_string(&get_cell_bytes(row, 2));
        let ans = decode_db_string(&get_cell_bytes(row, 3));
        let pt: f64 = row.get(4).unwrap_or(0.0);

        Ok(LocalFillBlankAnswer {
            ques_id: q_id,
            comb_id: c_id,
            content,
            answer: ans,
            point: pt,
        })
    }).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for r in rows.flatten() {
        results.push(r);
    }

    Ok(results)
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

/// 查询某套试卷的全量本地缓存答案 (客观选择题 + 读写填空题)
pub fn query_paper_all_answers(db_path: &Path, paper_id: i64) -> Option<LocalPaperAnswers> {
    if !db_path.exists() {
        return None;
    }

    let mut all_choices = query_choice_answers(db_path, paper_id).unwrap_or_default();
    let mut blanks = query_blank_answers(db_path, paper_id).unwrap_or_default();

    // 1. 若 choices 为空，尝试从 ScoreForChoose_HW 补充查询
    if all_choices.is_empty() {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT PaperID, SecID, ItemID, EntityID, [Order], Answer, MaxScore FROM [ScoreForChoose_HW] WHERE PaperID = ?1"
            ) {
                if let Ok(rows) = stmt.query_map([paper_id], |row| {
                    let p_id: i64 = row.get(0).unwrap_or(0);
                    let s_id: i64 = row.get(1).unwrap_or(0);
                    let i_id: i64 = row.get(2).unwrap_or(0);
                    let entity = decode_db_string(&get_cell_bytes(row, 3));
                    let order = decode_db_string(&get_cell_bytes(row, 4));
                    let ans = decode_db_string(&get_cell_bytes(row, 5));
                    let max_score: f64 = row.get(6).unwrap_or(0.0);
                    Ok(LocalChoiceAnswer {
                        paper_id: p_id,
                        sec_id: s_id,
                        item_id: i_id,
                        entity_id: entity,
                        order,
                        answer: ans,
                        max_score,
                    })
                }) {
                    for r in rows.flatten() {
                        all_choices.push(r);
                    }
                }
            }
        }
    }

    // 2. 若根据 paper_id 作为 comb_id 没找到填空题，尝试在 RW_Comb 中通过 ExamID 或 HomeworkID 关联查询
    if blanks.is_empty() {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            let pid_str = paper_id.to_string();
            if let Ok(mut stmt) = conn.prepare("SELECT ID FROM [RW_Comb] WHERE ExamID = ?1 OR HomeworkID = ?1") {
                if let Ok(rows) = stmt.query_map([&pid_str], |row| {
                    Ok(decode_db_string(&get_cell_bytes(row, 0)))
                }) {
                    for comb_str in rows.flatten() {
                        if let Ok(cid) = comb_str.parse::<i64>() {
                            if let Ok(b_list) = query_blank_answers(db_path, cid) {
                                blanks.extend(b_list);
                            }
                        }
                    }
                }
            }
        }
    }

    if all_choices.is_empty() && blanks.is_empty() {
        None
    } else {
        Some(LocalPaperAnswers {
            paper_id,
            choices: all_choices,
            blanks,
        })
    }
}

/// 根据试卷名称在 Paper 表中反向检索 PaperID
pub fn find_paper_id_by_name(db_path: &Path, name: &str) -> Option<i64> {
    if !db_path.exists() || name.trim().is_empty() {
        return None;
    }
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).ok()?;

    let mut stmt = conn.prepare("SELECT PaperID, Paper_mc, Paper_jc FROM [Paper]").ok()?;
    let rows = stmt.query_map([], |row| {
        let pid: i64 = row.get(0).unwrap_or(0);
        let mc = decode_db_string(&get_cell_bytes(row, 1));
        let jc = decode_db_string(&get_cell_bytes(row, 2));
        Ok((pid, mc, jc))
    }).ok()?;

    let target = name.trim().to_lowercase();
    let mut candidate = None;

    for r in rows.flatten() {
        let (pid, mc, jc) = r;
        let mc_clean = mc.trim().to_lowercase();
        let jc_clean = jc.trim().to_lowercase();

        if (!mc_clean.is_empty() && mc_clean == target) || (!jc_clean.is_empty() && jc_clean == target) {
            return Some(pid);
        }
        if (!mc_clean.is_empty() && (mc_clean.contains(&target) || target.contains(&mc_clean)))
            || (!jc_clean.is_empty() && (jc_clean.contains(&target) || target.contains(&jc_clean)))
        {
            candidate = Some(pid);
        }
    }

    candidate
}

/// 综合匹配方法：支持按 PaperID (数字)、Publish_no / ResourceID、HomeworkID 或作业试卷名称多维度极速匹配答案
pub fn match_paper_answers(db_path: &Path, hw_id: &str, hw_name: &str, resource_id: &str) -> Option<LocalPaperAnswers> {
    // 1. 若 hw_id 可直接解析为数字 PaperID
    if let Ok(pid) = hw_id.parse::<i64>() {
        if let Some(ans) = query_paper_all_answers(db_path, pid) {
            return Some(ans);
        }
    }

    // 2. 若 resource_id 可解析为数字 PaperID 或 Publish_no
    if let Ok(pid) = resource_id.parse::<i64>() {
        if let Some(ans) = query_paper_all_answers(db_path, pid) {
            return Some(ans);
        }
    }

    // 3. 尝试通过 Publish_no 在 Paper 表中查找 PaperID
    if !resource_id.trim().is_empty() {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            if let Ok(mut stmt) = conn.prepare("SELECT PaperID FROM [Paper] WHERE Publish_no = ?1") {
                if let Ok(mut rows) = stmt.query([resource_id]) {
                    if let Ok(Some(row)) = rows.next() {
                        let pid: i64 = row.get(0).unwrap_or(0);
                        if let Some(ans) = query_paper_all_answers(db_path, pid) {
                            return Some(ans);
                        }
                    }
                }
            }
        }
    }

    // 4. 尝试通过试卷名称在 Paper 表中查找 PaperID
    if !hw_name.trim().is_empty() {
        if let Some(pid) = find_paper_id_by_name(db_path, hw_name) {
            if let Some(ans) = query_paper_all_answers(db_path, pid) {
                return Some(ans);
            }
        }
    }

    // 5. 尝试在 RW_Comb 中直接匹配 HomeworkID
    if let Ok(conn) = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        if let Ok(mut stmt) = conn.prepare("SELECT ID FROM [RW_Comb] WHERE HomeworkID = ?1") {
            if let Ok(rows) = stmt.query_map([hw_id], |row| Ok(decode_db_string(&get_cell_bytes(row, 0)))) {
                let mut blanks = Vec::new();
                for comb_str in rows.flatten() {
                    if let Ok(cid) = comb_str.parse::<i64>() {
                        if let Ok(b_list) = query_blank_answers(db_path, cid) {
                            blanks.extend(b_list);
                        }
                    }
                }
                if !blanks.is_empty() {
                    return Some(LocalPaperAnswers {
                        paper_id: 0,
                        choices: Vec::new(),
                        blanks,
                    });
                }
            }
        }
    }

    None
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

            // 测试客观题答案查询 (PaperID = 1)
            let choices = query_choice_answers(&path, 1).expect("查询选择题答案失败");
            assert!(!choices.is_empty(), "PaperID 1 的选择题答案不应为空");
            assert_eq!(choices[0].answer, "A");
            assert_eq!(choices[1].answer, "C");
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
    fn test_query_paper_all_answers_real() {
        let path = PathBuf::from(r"F:\fepc\ETS\localdata\ETS.db");
        if path.exists() {
            // 1. 查询 PaperID = 1 的全量答案 (包含选择题)
            let ans1 = query_paper_all_answers(&path, 1).expect("PaperID 1 应该命中答案");
            assert!(!ans1.choices.is_empty(), "PaperID 1 应该包含客观选择题");
            assert_eq!(ans1.choices[0].answer, "A");
            assert_eq!(ans1.choices[1].answer, "C");

            // 2. 查询 PaperID = 48 的全量答案 (包含 20 道选择题)
            let ans48 = query_paper_all_answers(&path, 48).expect("PaperID 48 应该命中答案");
            assert_eq!(ans48.choices.len(), 20, "PaperID 48 应包含 20 道选择题");
            assert_eq!(ans48.choices[0].answer, "B");
            assert_eq!(ans48.choices[1].answer, "A");

            // 3. 查询 CombID = 1301 的填空题
            let ans_blank = query_paper_all_answers(&path, 1301).expect("CombID 1301 应该命中填空题");
            assert!(!ans_blank.blanks.is_empty(), "CombID 1301 应该包含填空题答案");
            assert_eq!(ans_blank.blanks[0].answer, "meet");
            assert_eq!(ans_blank.blanks[1].answer, "too");

            // 4. 查询 ExamID = 251 (通过 RW_Comb 关联检索填空题)
            let ans_exam = query_paper_all_answers(&path, 251).expect("ExamID 251 应该关联命中填空题");
            assert_eq!(ans_exam.blanks.len(), 65, "ExamID 251 应该关联包含 65 道填空题");

            // 5. 测试 match_paper_answers 综合匹配与 apply_to_homework
            let matched = match_paper_answers(&path, "48", "人教版7年级(上)英语听力单元测试Unit 1", "42236")
                .expect("综合匹配 PaperID 48 应该成功");
            assert_eq!(matched.choices.len(), 20);

            let mut hw = crate::models::HomeworkItem {
                id: "48".to_string(),
                name: "人教版7年级(上)英语听力单元测试Unit 1".to_string(),
                status: "未完成".to_string(),
                deadline: "".to_string(),
                total_point: 20.0,
                awarded_point: None,
                question_count: 0,
                source: crate::models::HomeworkSource::Local,
                local_dir: None,
                resource_urls: Vec::new(),
                sections: Vec::new(),
                set_type: 0,
                pk: 0,
                resource_id: "42236".to_string(),
            };

            matched.apply_to_homework(&mut hw);
            assert_eq!(hw.total_questions(), 20, "融合答案后题目数量应该为 20");
            assert_eq!(hw.sections.len(), 1);
            assert_eq!(hw.sections[0].questions[0].answers, vec!["B".to_string()]);
            assert_eq!(hw.sections[0].questions[1].answers, vec!["A".to_string()]);
            assert_eq!(hw.status, "已匹配本地答案");
        }
    }

    #[test]
    fn test_mock_read_cached_homeworks_and_all_answers() {
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
                );
                CREATE TABLE [ScoreForChoose] (
                    PaperID INT, SecID INT, ItemID INT, EntityID VARCHAR, [Order] VARCHAR,
                    Answer VARCHAR, MaxScore DOUBLE
                );
                INSERT INTO [ScoreForChoose] VALUES (
                    888, 1, 10, 'ENT_1', '1', 'B', 2.0
                );
                INSERT INTO [ScoreForChoose] VALUES (
                    888, 1, 11, 'ENT_2', '2', 'D', 2.0
                );
                CREATE TABLE [RW_Info] (
                    QuesID INT, CombID INT, Content VARCHAR, Answer VARCHAR, Point DOUBLE
                );
                INSERT INTO [RW_Info] VALUES (
                    1001, 888, 'Hello {{blank}}', 'world', 1.0
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

        // 验证 query_paper_all_answers
        let all_ans = query_paper_all_answers(&db_file, 888).expect("查询 Mock 答案失败");
        assert_eq!(all_ans.choices.len(), 2);
        assert_eq!(all_ans.choices[0].answer, "B");
        assert_eq!(all_ans.choices[1].answer, "D");
        assert_eq!(all_ans.blanks.len(), 1);
        assert_eq!(all_ans.blanks[0].answer, "world");

        let mut hw = crate::models::HomeworkItem {
            id: "HW_TEST_1".to_string(),
            name: "Unit 1 Mock Homework".to_string(),
            status: "未完成".to_string(),
            deadline: "".to_string(),
            total_point: 100.0,
            awarded_point: None,
            question_count: 0,
            source: crate::models::HomeworkSource::Local,
            local_dir: None,
            resource_urls: Vec::new(),
            sections: Vec::new(),
            set_type: 0,
            pk: 0,
            resource_id: "".to_string(),
        };

        all_ans.apply_to_homework(&mut hw);
        assert_eq!(hw.sections.len(), 2);
        assert_eq!(hw.total_questions(), 3);
        assert_eq!(hw.sections[0].questions[0].answers, vec!["B".to_string()]);
        assert_eq!(hw.sections[1].questions[0].answers, vec!["world".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

