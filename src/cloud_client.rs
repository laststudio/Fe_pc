use std::fs::{self, File};
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::models::{
    HomeworkItem, HomeworkSource, ParsedQuestion, ParsedSection, QuestionOption, UserSession,
};

pub struct CloudClient;

impl CloudClient {
    pub fn session_file_path() -> PathBuf {
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("fepc_session.json");
        path
    }

    pub fn load_saved_session() -> Option<UserSession> {
        let path = Self::session_file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<UserSession>(&content) {
                    return Some(session);
                }
            }
        }
        None
    }

    pub fn save_session(session: &UserSession) {
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

    /// 生成或获取现有设备码
    pub fn get_or_generate_device_code(existing: Option<&str>) -> String {
        if let Some(code) = existing {
            if !code.trim().is_empty() {
                return code.trim().to_string();
            }
        }
        let res = cloud::cloud::cloud_generate_device_code_json();
        if let Some(code) = res.get("result").and_then(|r| r.get("device_code")).and_then(|v| v.as_str()) {
            return code.to_string();
        }
        "BF1CB2218D9A88B9|0C072A134F008B9F".to_string()
    }

    /// 执行官方 ETS100 云端登录
    pub fn login(phone: &str, password: &str, device_code: &str) -> Result<UserSession, String> {
        let res = cloud::cloud::cloud_login_json(phone, password, device_code);
        if res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let result = res.get("result").ok_or("未返回结果数据")?;
            let token = result.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let parent_id = result.get("parent_account_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let user_name = result.get("user_name").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let session = UserSession {
                phone: phone.to_string(),
                token,
                device_code: device_code.to_string(),
                parent_account_id: parent_id,
                user_name,
                is_logged_in: true,
            };
            Self::save_session(&session);
            Ok(session)
        } else {
            let error = res.get("error").cloned().unwrap_or(json!({}));
            let code = error.get("code").and_then(|v| v.as_str()).unwrap_or("unknown");
            let msg = error.get("message").and_then(|v| v.as_str()).unwrap_or("登录失败");
            let api_code = error.get("api_code").and_then(|v| v.as_i64()).unwrap_or(0);

            if code == "device_bind_required" || api_code == 30014 {
                Err("device_bind_required".to_string())
            } else {
                Err(format!("{}: {}", code, msg))
            }
        }
    }

    /// 发送换绑验证码
    pub fn send_rebind_code(phone: &str) -> Result<(), String> {
        let res = cloud::cloud::cloud_send_device_binding_code_json(phone);
        if res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let msg = res.get("error").and_then(|e| e.get("message")).and_then(|v| v.as_str()).unwrap_or("发送验证码失败");
            Err(msg.to_string())
        }
    }

    /// 验证码换绑并登录
    pub fn bind_device(phone: &str, password: &str, device_code: &str, code: &str) -> Result<UserSession, String> {
        let res = cloud::cloud::cloud_bind_device_json(phone, password, device_code, code);
        if res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let result = res.get("result").ok_or("换绑未返回结果")?;
            let token = result.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let parent_id = result.get("parent_account_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let user_name = result.get("user_name").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let session = UserSession {
                phone: phone.to_string(),
                token,
                device_code: device_code.to_string(),
                parent_account_id: parent_id,
                user_name,
                is_logged_in: true,
            };
            Self::save_session(&session);
            Ok(session)
        } else {
            let msg = res.get("error").and_then(|e| e.get("message")).and_then(|v| v.as_str()).unwrap_or("设备换绑失败");
            Err(msg.to_string())
        }
    }

    /// 获取云端作业列表
    pub fn get_homework_list(session: &UserSession, status: &str) -> Result<Vec<HomeworkItem>, String> {
        let res = cloud::cloud::cloud_get_homework_list_json(
            &session.token,
            &session.parent_account_id,
            status,
        );

        if !res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let msg = res.get("error").and_then(|e| e.get("message")).and_then(|v| v.as_str()).unwrap_or("获取作业列表失败");
            return Err(msg.to_string());
        }

        let mut items = Vec::new();
        if let Some(list) = res.get("result").and_then(|r| r.get("homeworks")).and_then(|v| v.as_array()) {
            for hw in list {
                let id = hw.get("id").map(|v| match v {
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    _ => "".to_string(),
                }).unwrap_or_default();

                let name = hw.get("name").and_then(|v| v.as_str()).unwrap_or("未命名作业").to_string();
                let deadline = hw.get("deadline").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let status_str = hw.get("status_name").and_then(|v| v.as_str()).unwrap_or(status).to_string();
                let total_point = hw.get("total_point").and_then(|v| v.as_f64()).unwrap_or(30.0);
                let awarded_point = hw.get("real_score").and_then(|v| v.as_f64());
                let question_count = hw.get("question_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                let mut resource_urls = Vec::new();
                if let Some(contents) = hw.get("contents").and_then(|v| v.as_array()) {
                    for c in contents {
                        if let Some(dl_url) = c.get("download_url").and_then(|v| v.as_str()) {
                            if !dl_url.is_empty() {
                                resource_urls.push(dl_url.to_string());
                            }
                        }
                    }
                }

                items.push(HomeworkItem {
                    id,
                    name,
                    status: status_str,
                    deadline,
                    total_point,
                    awarded_point,
                    question_count,
                    source: HomeworkSource::Cloud,
                    local_dir: None,
                    resource_urls,
                    sections: Vec::new(),
                });
            }
        }
        Ok(items)
    }

    /// 下载并解析云端试卷
    pub fn download_and_parse_homework(_session: &UserSession, homework: &HomeworkItem) -> Result<HomeworkItem, String> {
        let temp_dir = std::env::temp_dir().join("fepc_cache").join(&homework.id);
        let _ = fs::create_dir_all(&temp_dir);

        let mut sections = Vec::new();
        let mut question_counter = 0usize;

        for (idx, url) in homework.resource_urls.iter().enumerate() {

            let zip_dest = temp_dir.join(format!("part_{idx}.zip"));
            let zip_dest_str = zip_dest.to_string_lossy().to_string();

            // 下载 ZIP 文件
            let dl_res = cloud::cloud::cloud_download_file_json(url, &zip_dest_str);
            if !dl_res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }

            // 计算解压密码
            let pwd_res = cloud::cloud::cloud_zip_password_json(&zip_dest_str);
            let password = pwd_res.get("result")
                .and_then(|r| r.get("password"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // 解压 ZIP
            let part_dir = temp_dir.join(format!("part_{idx}"));
            let _ = fs::create_dir_all(&part_dir);

            if let Ok(file) = File::open(&zip_dest) {
                let mut archive = match zip::ZipArchive::new(file) {
                    Ok(a) => a,
                    Err(_) => continue,
                };

                for i in 0..archive.len() {
                    let zip_res = if password.is_empty() {
                        archive.by_index(i)
                    } else {
                        archive.by_index_decrypt(i, password.as_bytes())
                    };
                    if let Ok(mut zip_file) = zip_res {
                        let out_path = match zip_file.enclosed_name() {
                            Some(p) => part_dir.join(p),
                            None => continue,
                        };
                        if zip_file.is_dir() {
                            let _ = fs::create_dir_all(&out_path);
                        } else {
                            if let Some(p) = out_path.parent() {
                                let _ = fs::create_dir_all(p);
                            }
                            if let Ok(mut outfile) = File::create(&out_path) {
                                let _ = std::io::copy(&mut zip_file, &mut outfile);
                            }
                        }
                    }
                }
            }

            // 解析 content.json
            let content_file = part_dir.join("content.json");
            if content_file.exists() {
                if let Ok(c_str) = fs::read_to_string(&content_file) {
                    if let Ok(content_val) = serde_json::from_str::<Value>(&c_str) {
                        let content2_val = part_dir.join("content2.json").exists().then(|| {
                            fs::read_to_string(part_dir.join("content2.json"))
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                        }).flatten();

                        let info_val = part_dir.join("info.json").exists().then(|| {
                            fs::read_to_string(part_dir.join("info.json"))
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                        }).flatten();

                        let structure_type = content_val.get("structure_type")
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
                            let no = raw_q.get("no").map(|v| match v {
                                Value::Number(n) => n.to_string(),
                                Value::String(s) => s.clone(),
                                _ => question_counter.to_string(),
                            }).unwrap_or_else(|| question_counter.to_string());

                            let cat = raw_q.get("category").and_then(|v| v.as_str()).unwrap_or(&structure_type).to_string();
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
                            if let Some(Value::Array(arr)) = raw_q.get("important") {
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        if !s.is_empty() { important_points.push(s.to_string()); }
                                    }
                                }
                            }

                            let mut options = Vec::new();
                            if let Some(opts) = raw_q.get("options").and_then(|v| v.as_array()) {
                                for opt in opts {
                                    match opt {
                                        Value::String(s) => {
                                            let trimmed = s.trim();
                                            let (label, text) = if trimmed.len() >= 2 && trimmed.chars().nth(1) == Some('.') {
                                                (trimmed[0..1].to_string(), trimmed[2..].trim().to_string())
                                            } else {
                                                ("".to_string(), trimmed.to_string())
                                            };
                                            options.push(QuestionOption { label, text, image_path: None });
                                        }
                                        Value::Object(obj) => {
                                            let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                            let img = obj.get("image").or_else(|| obj.get("image_path")).and_then(|v| v.as_str()).map(|s| s.to_string());
                                            options.push(QuestionOption { label, text, image_path: img });
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            let audio_file = raw_q.get("audio").and_then(|v| v.as_str()).map(|s| s.to_string());

                            questions.push(ParsedQuestion {
                                index: question_counter - 1,
                                number: no,
                                category: cat,
                                prompt_text: prompt,
                                answers,
                                important_points,
                                audio_file,
                                options,
                            });
                        }

                        if !questions.is_empty() {
                            let friendly_title = match structure_type.as_str() {
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
                            };

                            sections.push(ParsedSection {
                                section_type: structure_type,
                                title: friendly_title,
                                questions,
                            });
                        }
                    }
                }
            }
        }

        if sections.is_empty() {
            return Err("未能解析出有效的试卷题目".to_string());
        }

        let total_q = sections.iter().map(|s| s.questions.len()).sum();

        Ok(HomeworkItem {
            id: homework.id.clone(),
            name: homework.name.clone(),
            status: "已下载完成".to_string(),
            deadline: homework.deadline.clone(),
            total_point: homework.total_point,
            awarded_point: homework.awarded_point,
            question_count: total_q,
            source: HomeworkSource::Cloud,
            local_dir: Some(temp_dir.to_string_lossy().to_string()),
            resource_urls: homework.resource_urls.clone(),
            sections,
        })
    }
}
