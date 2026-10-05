use std::fs;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::Value;

use crate::models::{
    HomeworkItem, HomeworkSource, LocalAccountInfo, ParsedQuestion, ParsedSection, QuestionOption,
};

pub struct LocalScanner;

impl LocalScanner {
    /// 查找系统 AppData 下所有匹配 74656D705F74656D705F74656D705F* 的目录以及 ETS 目录
    pub fn find_ets_local_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(appdata) = std::env::var("APPDATA") {
            let appdata_path = PathBuf::from(appdata);

            // 1. 显式检测 %APPDATA%/ETS 与 %APPDATA%/74656D705F 目录
            let ets_dir = appdata_path.join("ETS");
            if ets_dir.is_dir() {
                dirs.push(ets_dir);
            }
            let hex_temp_dir = appdata_path.join("74656D705F");
            if hex_temp_dir.is_dir() && !dirs.iter().any(|d| d == &hex_temp_dir) {
                dirs.push(hex_temp_dir);
            }

            // 2. 遍历查找匹配 74656D705F* 或大小写不敏感 ETS 的目录
            if let Ok(entries) = fs::read_dir(&appdata_path) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_match = file_name.starts_with("74656D705F")
                        || file_name.eq_ignore_ascii_case("ETS");
                    if is_match && entry.path().is_dir() {
                        let path = entry.path();
                        if !dirs.iter().any(|d| d == &path) {
                            dirs.push(path);
                        }
                    }
                }
            }
        }

        // 优先将包含实际作业数据的目录排在前面，其次按修改时间从新到旧排序
        dirs.sort_by(|a, b| {
            let has_hw_a = fs::read_dir(a)
                .map(|mut it| it.any(|e| e.map(|entry| entry.path().is_dir() && !matches!(entry.file_name().to_str(), Some("common" | "system" | "clear_cache" | "pc_xst_dict"))).unwrap_or(false)))
                .unwrap_or(false);
            let has_hw_b = fs::read_dir(b)
                .map(|mut it| it.any(|e| e.map(|entry| entry.path().is_dir() && !matches!(entry.file_name().to_str(), Some("common" | "system" | "clear_cache" | "pc_xst_dict"))).unwrap_or(false)))
                .unwrap_or(false);

            match has_hw_b.cmp(&has_hw_a) {
                std::cmp::Ordering::Equal => {
                    let time_a = fs::metadata(a).and_then(|m| m.modified()).ok();
                    let time_b = fs::metadata(b).and_then(|m| m.modified()).ok();
                    time_b.cmp(&time_a)
                }
                other => other,
            }
        });
        dirs
    }

    /// 从指定的本地 ETS 目录中解析 userinfo 保存的账号信息
    #[inline(never)]
    pub fn read_local_accounts(ets_dir: &Path) -> Vec<LocalAccountInfo> {
        let mut accounts = Vec::new();
        let userinfo_path = ets_dir.join("userinfo");
        let b64 = match fs::read_to_string(&userinfo_path) {
            Ok(s) => s,
            Err(_) => return accounts,
        };

        crate::themida_tiger_white!({
            if let Ok(bytes) = BASE64.decode(b64.trim()) {
                // ETS PC 客户端以 GBK/CP936 编码写入 userinfo，直接按 UTF-8 解码会导致中文学生姓名乱码
                let (cow, _, _) = encoding_rs::GBK.decode(&bytes);
                let json_str = cow.as_ref();
                if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(json_str) {
                    for item in items {
                        let account = item.get("Account").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let token = item.get("Token").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let cur_res_id = item.get("CurResItemID").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if !account.is_empty() {
                            accounts.push(LocalAccountInfo {
                                account,
                                name,
                                token,
                                cur_res_id,
                            });
                        }
                    }
                }
            }
        });
        accounts
    }

    /// 从指定的本地 ETS 目录中解析设备码 ProductID
    #[inline(never)]
    pub fn read_local_device_code(ets_dir: &Path) -> Option<String> {
        let config_path = ets_dir.join("configinfo");
        let b64 = fs::read_to_string(&config_path).ok()?;
        let bytes = BASE64.decode(b64.trim()).ok()?;
        let entries: Vec<Value> = serde_json::from_slice(&bytes).ok()?;

        let mut product_id = None;
        crate::themida_tiger_white!({
            for entry in &entries {
                let entry_name = entry.get("Entry").and_then(|v| v.as_str()).unwrap_or("");
                if entry_name == "ProductID"
                    && let Some(val) = entry.get("Value").and_then(|v| v.as_str())
                    && !val.is_empty()
                {
                    product_id = Some(val.to_string());
                    break;
                }
            }
        });
        product_id
    }

    /// 扫描 ETS 目录中所有作业文件夹并调用 resolver 引擎解析
    pub fn scan_homeworks(ets_dir: &Path) -> Vec<HomeworkItem> {
        let mut homeworks = Vec::new();
        if let Ok(entries) = fs::read_dir(ets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                // 过滤已知非作业目录
                if name == "common"
                    || name == "system"
                    || name == "clear_cache"
                    || name == "pc_xst_dict"
                    || name.starts_with('.')
                {
                    continue;
                }

                if let Some(hw) = Self::parse_homework_folder(&path) {
                    homeworks.push(hw);
                }
            }
        }

        // 按修改时间降序排序
        homeworks.sort_by(|a, b| b.id.cmp(&a.id));
        homeworks
    }

    /// 解析单个本地作业目录 (如 42236 目录)
    pub fn parse_homework_folder(folder: &Path) -> Option<HomeworkItem> {
        let folder_name = folder.file_name()?.to_string_lossy().to_string();
        let mut sections = Vec::new();
        let (id, mut title) = if let Some((id_part, name_part)) = folder_name.split_once('-') {
            (id_part.to_string(), name_part.to_string())
        } else {
            (folder_name.clone(), format!("本地试卷 {}", folder_name))
        };

        // 尝试从 template/res.json 提取题型名称与试卷名
        let mut template_section_titles = Vec::new();
        if let Ok(entries) = fs::read_dir(folder) {
            for entry in entries.flatten() {
                let sub_path = entry.path();
                if sub_path.is_dir() && sub_path.file_name().unwrap_or_default().to_string_lossy().starts_with("template") {
                    let res_json = sub_path.join("res.json");
                    if res_json.exists()
                        && let Ok(content) = fs::read_to_string(&res_json)
                            && let Ok(val) = serde_json::from_str::<Value>(&content) {
                                if let Some(paper_name) = val.get("name").and_then(|v| v.as_str())
                                    && !paper_name.is_empty() {
                                        title = paper_name.to_string();
                                    }
                                if let Some(list) = val.get("exam_type_list").and_then(|v| v.as_array()) {
                                    for item in list {
                                        if let Some(t_name) = item.get("exam_type_name").and_then(|v| v.as_str()) {
                                            template_section_titles.push(t_name.to_string());
                                        }
                                    }
                                }
                            }
                }
            }
        }

        // 遍历所有包含 content.json 的子目录 (兼容 content0001000X 与 MD5 哈希子目录等)
        let mut content_dirs = Vec::new();
        if let Ok(entries) = fs::read_dir(folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("content.json").exists() {
                    content_dirs.push(path);
                }
            }
        }

        // 排序确保题型顺序
        content_dirs.sort();

        let mut question_counter = 0usize;

        for dir in content_dirs {
            let content_file = dir.join("content.json");
            if !content_file.exists() {
                continue;
            }

            let content_str = fs::read_to_string(&content_file).unwrap_or_default();
            let content_val: Value = match serde_json::from_str(&content_str) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let content2_val: Option<Value> = {
                let p = dir.join("content2.json");
                if p.exists() {
                    fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str(&s).ok())
                } else {
                    None
                }
            };

            let info_val: Option<Value> = {
                let p = dir.join("info.json");
                if p.exists() {
                    fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str(&s).ok())
                } else {
                    None
                }
            };

            let structure_type = content_val
                .get("structure_type")
                .and_then(|v| v.as_str())
                .unwrap_or("未知题型")
                .to_string();

            let raw_questions = resolver::api::parse_task(
                &content_val,
                content2_val.as_ref(),
                info_val.as_ref(),
            );

            let mut questions = Vec::new();
            for raw_q in raw_questions {
                question_counter += 1;

                let no = raw_q
                    .get("no")
                    .map(|v| match v {
                        Value::Number(n) => n.to_string(),
                        Value::String(s) => s.clone(),
                        _ => question_counter.to_string(),
                    })
                    .unwrap_or_else(|| question_counter.to_string());

                let cat = raw_q
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&structure_type)
                    .to_string();

                let prompt = raw_q
                    .get("question")
                    .or_else(|| raw_q.get("text"))
                    .or_else(|| raw_q.get("prompt"))
                    .or_else(|| raw_q.get("ask"))
                    .or_else(|| raw_q.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let mut answers = Vec::new();
                if let Some(ans_val) = raw_q.get("answers").or_else(|| raw_q.get("answer")) {
                    match ans_val {
                        Value::Array(arr) => {
                            for a in arr {
                                if let Some(s) = a.as_str() {
                                    let s_trim = s.trim();
                                    if !s_trim.is_empty() {
                                        answers.push(s_trim.to_string());
                                    }
                                }
                            }
                        }
                        Value::String(s) => {
                            let s_trim = s.trim();
                            if !s_trim.is_empty() {
                                answers.push(s_trim.to_string());
                            }
                        }
                        _ => {}
                    }
                }

                let mut important_points = Vec::new();
                if let Some(imp_val) = raw_q.get("important")
                    && let Value::Array(arr) = imp_val {
                        for item in arr {
                            if let Some(s) = item.as_str()
                                && !s.is_empty() {
                                    important_points.push(s.to_string());
                                }
                        }
                    }

                let mut options = Vec::new();
                if let Some(opts) = raw_q.get("options").and_then(|v| v.as_array()) {
                    for opt in opts {
                        match opt {
                            Value::String(s) => {
                                let trimmed = s.trim();
                                let (label, text) = if let Some((l, r)) = trimmed.split_once('.') {
                                    if l.chars().count() == 1 {
                                        (l.trim().to_string(), r.trim().to_string())
                                    } else {
                                        ("".to_string(), trimmed.to_string())
                                    }
                                } else {
                                    ("".to_string(), trimmed.to_string())
                                };
                                options.push(QuestionOption {
                                    label,
                                    text,
                                    image_path: None,
                                });
                            }
                            Value::Object(obj) => {
                                let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let img = obj.get("image").or_else(|| obj.get("image_path")).and_then(|v| v.as_str()).map(|s| s.to_string());
                                options.push(QuestionOption {
                                    label,
                                    text,
                                    image_path: img,
                                });
                            }
                            _ => {}
                        }
                    }
                }

                let audio_file = raw_q
                    .get("audio")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let word_id = raw_q.get("word_id").and_then(|v| v.as_i64());
                let question_id = raw_q.get("question_id").and_then(|v| v.as_i64());

                questions.push(ParsedQuestion {
                    index: question_counter - 1,
                    number: no,
                    category: cat,
                    prompt_text: prompt,
                    answers,
                    important_points,
                    audio_file,
                    options,
                    word_id,
                    question_id,
                });
            }

            if !questions.is_empty() {
                let friendly_title = if let Some(t_name) = template_section_titles.get(sections.len()) {
                    t_name.clone()
                } else {
                    match structure_type.as_str() {
                        "collector.choose" => "听后选择".to_string(),
                        "collector.read" => "短文朗读".to_string(),
                        "collector.picture" => "看图说话".to_string(),
                        "collector.fill" => "听后填空".to_string(),
                        "collector.role" | "collector.role_play" => "角色扮演".to_string(),
                        "collector.dialogue" => "听后回答 (简答)".to_string(),
                        "collector.repeat" | "collector.repeat_essay" | "collector.repeat_dialogue" => "短文复述".to_string(),
                        "collector.3q5a" => "三问五答".to_string(),
                        "collector.word" => "单词辨音".to_string(),
                        "collector.essay" => "情景表达".to_string(),
                        other => other.to_string(),
                    }
                };

                sections.push(ParsedSection {
                    section_type: structure_type,
                    title: friendly_title,
                    questions,
                });
            }
        }

        if sections.is_empty() {
            return None;
        }

        // 根据各章节第一小题题号排序，确保按试卷自然顺序呈现 (1, 2, 3...)
        sections.sort_by_key(|sec| {
            sec.questions.first()
                .and_then(|q| q.number.parse::<i64>().ok())
                .unwrap_or(9999)
        });

        let total_q = sections.iter().map(|s| s.questions.len()).sum();

        Some(HomeworkItem {
            id,
            name: title,
            status: "已保存在本地".to_string(),
            deadline: "".to_string(),
            total_point: 30.0,
            awarded_point: None,
            question_count: total_q,
            source: HomeworkSource::Local,
            local_dir: Some(folder.to_string_lossy().to_string()),
            resource_urls: Vec::new(),
            sections,
            set_type: 0,
            pk: 0,
            resource_id: String::new(),
        })
    }
}
