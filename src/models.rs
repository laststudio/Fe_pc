use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomeworkSource {
    Cloud,
    Local,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    pub text: String,
    pub image_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedQuestion {
    pub index: usize,
    pub number: String,
    pub category: String,
    pub prompt_text: String,
    pub answers: Vec<String>,
    pub important_points: Vec<String>,
    pub audio_file: Option<String>,
    pub options: Vec<QuestionOption>,
}

impl ParsedQuestion {
    pub fn display_category(&self) -> &str {
        match self.category.as_str() {
            "collector.choose" => "听后选择",
            "collector.read" => "短文朗读",
            "collector.picture" => "看图说话",
            "collector.fill" => "听后填空",
            "collector.role" | "collector.role_play" => "角色扮演 / 口语问答",
            "collector.dialogue" => "听后回答",
            "collector.repeat" | "collector.repeat_essay" | "collector.repeat_dialogue" => "短文复述",
            "collector.3q5a" => "三问五答",
            "collector.word" => "单词辨音",
            "collector.essay" => "情景表达",
            other if other.starts_with("collector.") => &other[10..],
            other if !other.is_empty() => other,
            _ => "常规题型",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedSection {
    pub section_type: String,
    pub title: String,
    pub questions: Vec<ParsedQuestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HomeworkItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub deadline: String,
    pub total_point: f64,
    pub awarded_point: Option<f64>,
    pub question_count: usize,
    pub source: HomeworkSource,
    pub local_dir: Option<String>,
    pub resource_urls: Vec<String>,
    pub sections: Vec<ParsedSection>,
}

impl HomeworkItem {
    pub fn is_parsed(&self) -> bool {
        !self.sections.is_empty()
    }

    pub fn total_questions(&self) -> usize {
        if self.sections.is_empty() {
            self.question_count
        } else {
            self.sections.iter().map(|s| s.questions.len()).sum()
        }
    }

    pub fn get_concise_answers(&self) -> Vec<(String, String)> {
        let mut list = Vec::new();
        for section in &self.sections {
            for q in &section.questions {
                let ans = if q.answers.is_empty() {
                    "无".to_string()
                } else if q.answers.len() == 1 {
                    q.answers[0].clone()
                } else if q.answers.iter().all(|a| a.len() <= 6) {
                    q.answers.join(" / ")
                } else {
                    format!("{} (共{}种备选表述)", q.answers[0], q.answers.len())
                };
                let label = if q.number.is_empty() {
                    format!("第 {} 题", q.index + 1)
                } else {
                    format!("第 {} 题", q.number)
                };
                list.push((label, ans));
            }
        }
        list
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserSession {
    pub phone: String,
    pub token: String,
    pub device_code: String,
    pub parent_account_id: String,
    pub user_name: String,
    pub is_logged_in: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalAccountInfo {
    pub account: String,
    pub name: String,
    pub token: String,
    pub cur_res_id: String,
}
