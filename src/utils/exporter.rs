use std::fs;
use std::path::Path;

use arboard::Clipboard;

use crate::models::HomeworkItem;

pub struct Exporter;

impl Exporter {
    pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
        let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_text(text).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn format_full_answers(homework: &HomeworkItem) -> String {
        let mut s = String::new();
        s.push_str(&format!("========================================\n"));
        s.push_str(&format!("试卷名称: {}\n", homework.name));
        s.push_str(&format!("题目数量: {} 题 | 总分: {} 分\n", homework.total_questions(), homework.total_point));
        s.push_str(&format!("生成工具: Fe PC (Fuck ets100 Desktop)\n"));
        s.push_str(&format!("========================================\n\n"));

        for section in &homework.sections {
            s.push_str(&format!("【{}】(共 {} 题)\n", section.title, section.questions.len()));
            s.push_str(&format!("----------------------------------------\n"));

            for q in &section.questions {
                let no_str = if q.number.is_empty() {
                    format!("{}", q.index + 1)
                } else {
                    q.number.clone()
                };

                s.push_str(&format!("第 {} 题 [{}]\n", no_str, q.display_category()));
                if !q.prompt_text.is_empty() {
                    s.push_str(&format!("题干: {}\n", q.prompt_text));
                }

                if !q.options.is_empty() {
                    s.push_str("选项:\n");
                    for opt in &q.options {
                        s.push_str(&format!("  {}. {}\n", opt.label, opt.text));
                    }
                }

                if !q.answers.is_empty() {
                    s.push_str(&format!("标准答案: {}\n", q.answers.join(" / ")));
                }

                if !q.important_points.is_empty() {
                    s.push_str(&format!("关键要点: {}\n", q.important_points.join(", ")));
                }

                s.push_str("\n");
            }
            s.push_str("\n");
        }

        s
    }

    pub fn format_concise_answers(homework: &HomeworkItem) -> String {
        let mut s = String::new();
        s.push_str(&format!("{} - 极简答案速查\n", homework.name));
        s.push_str("----------------------------------------\n");

        for (label, ans) in homework.get_concise_answers() {
            s.push_str(&format!("{}: {}\n", label, ans));
        }

        s
    }

    pub fn format_markdown(homework: &HomeworkItem) -> String {
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n", homework.name));
        s.push_str(&format!("> **试卷状态**: {} | **总分**: {} 分 | **题数**: {} 题\n\n", homework.status, homework.total_point, homework.total_questions()));

        for section in &homework.sections {
            s.push_str(&format!("## {}\n\n", section.title));

            for q in &section.questions {
                let no_str = if q.number.is_empty() {
                    format!("{}", q.index + 1)
                } else {
                    q.number.clone()
                };

                s.push_str(&format!("### 第 {} 题 (`{}`)\n\n", no_str, q.display_category()));
                if !q.prompt_text.is_empty() {
                    s.push_str(&format!("**题目**: {}\n\n", q.prompt_text));
                }

                if !q.options.is_empty() {
                    s.push_str("**选项**:\n");
                    for opt in &q.options {
                        s.push_str(&format!("- **{}**. {}\n", opt.label, opt.text));
                    }
                    s.push_str("\n");
                }

                if !q.answers.is_empty() {
                    s.push_str(&format!("> **标准答案**: `{}`\n\n", q.answers.join("` / `")));
                }

                if !q.important_points.is_empty() {
                    s.push_str(&format!("- **关键得分点**: {}\n\n", q.important_points.join(", ")));
                }
            }
        }

        s
    }

    pub fn save_to_file(path: &Path, content: &str) -> Result<(), String> {
        fs::write(path, content).map_err(|e| e.to_string())
    }
}
