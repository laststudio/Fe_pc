use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScoreProfile {
    pub total: f64,
    pub accuracy: f64,
    pub fluency: f64,
    pub integrity: f64,
    pub stress: f64,
}

impl Default for ScoreProfile {
    fn default() -> Self {
        Self {
            total: 5.0,
            accuracy: 4.95,
            fluency: 4.95,
            integrity: 4.95,
            stress: 4.90,
        }
    }
}

impl ScoreProfile {
    pub fn new(total: f64) -> Self {
        let t = total.clamp(0.0, 5.0);
        let factor = t / 5.0;
        Self {
            total: t,
            accuracy: (4.95 * factor).clamp(0.0, 5.0),
            fluency: (4.95 * factor).clamp(0.0, 5.0),
            integrity: (4.95 * factor).clamp(0.0, 5.0),
            stress: (4.90 * factor).clamp(0.0, 5.0),
        }
    }

    pub fn clamped(&self) -> Self {
        let clamp = |v: f64| v.clamp(0.0, 5.0);
        Self {
            total: clamp(self.total),
            accuracy: clamp(self.accuracy),
            fluency: clamp(self.fluency),
            integrity: clamp(self.integrity),
            stress: clamp(self.stress),
        }
    }
}

pub fn convert_score(score: f64, maximum: f64, graduation: f64) -> f64 {
    if maximum <= 0.0 {
        return 0.0;
    }
    let mut value = score * maximum / 5.0;
    if graduation > 0.0 {
        value = ((value - 1e-12) / graduation).ceil() * graduation;
    }
    (value.min(maximum) * 100.0).round() / 100.0
}

#[inline(never)]
pub fn build_score_detail(
    profile: &ScoreProfile,
    category: &str,
    choose: &str,
    repeat_mode: bool,
    graduation: f64,
) -> String {
    crate::themida_tiger_red!({
        let p = profile.clamped();
        let detail = json!({
            "total_score": format!("{:.2}", p.total),
            "choose": choose,
            "fluency_score": format!("{:.2}", p.fluency),
            "accuracy_score": format!("{:.2}", p.accuracy),
            "integrity_score": format!("{:.2}", p.integrity),
            "standard_score": format!("{:.2}", p.stress),
            "category": category,
            "repeat_mode": if repeat_mode { "0" } else { "1" },
            "version": "3",
            "rate_scale": if (graduation - 0.125).abs() < 1e-6 { 1 } else { 0 }
        });
        serde_json::to_string(&detail).unwrap_or_else(|_| "{}".to_string())
    })
}

fn phonemes(word: &str) -> Vec<String> {
    let clean: String = word.chars().filter(|c| c.is_alphabetic()).collect();
    let lower = clean.to_lowercase();
    if lower.is_empty() {
        return vec!["*".to_string()];
    }

    let pairs = [
        ("th", "θ"), ("sh", "ʃ"), ("ch", "tʃ"), ("wh", "w"), ("ph", "f"), ("ck", "k"),
        ("ng", "ŋ"), ("oo", "uː"), ("ee", "iː"), ("ea", "iː"), ("ou", "aʊ"),
        ("ow", "aʊ"), ("ai", "eɪ"), ("ay", "eɪ"), ("oi", "ɔɪ"), ("oy", "ɔɪ"),
    ];

    let mut result = Vec::new();
    let chars: Vec<char> = lower.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        if idx + 1 < chars.len() {
            let pair: String = chars[idx..=idx + 1].iter().collect();
            if let Some(&(_, phone)) = pairs.iter().find(|&&(p, _)| p == pair) {
                result.push(phone.to_string());
                idx += 2;
                continue;
            }
        }
        result.push(chars[idx].to_string());
        idx += 1;
    }
    result
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[inline(never)]
pub fn generate_score_xml(
    text: &str,
    profile: &ScoreProfile,
    category: &str,
    seed: u64,
) -> String {
    crate::themida_tiger_red!({
        let p = profile.clamped();
        let category_name = match category {
            "read_sentence" | "read_chapter" | "talk_recite" | "talk_free" => category,
            _ => "read_chapter",
        };

        // 伪随机微调波动（保持平稳逼真，不超出范围）
        let pseudo_noise = ((seed % 17) as f64 - 8.0) * 0.01;
        let standard = (p.total + pseudo_noise).clamp(0.0, 5.0);

        let sentences: Vec<&str> = text
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let valid_sentences = if sentences.is_empty() {
            vec![text.trim()]
        } else {
            sentences
        };

        let mut xml = String::with_capacity(1024);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        xml.push_str("<xml_result>");
        xml.push_str(&format!("<{category_name}>"));
        xml.push_str(&format!(
            "<rec_paper total_score=\"{:.2}\" accuracy_score=\"{:.2}\" fluency_score=\"{:.2}\" integrity_score=\"{:.2}\" standard_score=\"{:.2}\" stress_score=\"{:.2}\" is_rejected=\"false\" except_info=\"\" content=\"{}\">",
            p.total, p.accuracy, p.fluency, p.integrity, standard, p.stress, escape_xml(text)
        ));

        for (s_idx, sentence_str) in valid_sentences.iter().enumerate() {
            let s_noise = (((seed + s_idx as u64 * 7) % 13) as f64 - 6.0) * 0.01;
            let s_score = (p.total + s_noise).clamp(0.0, 5.0);

            xml.push_str(&format!(
                "<sentence content=\"{}\" total_score=\"{:.2}\">",
                escape_xml(sentence_str), s_score
            ));

            let words: Vec<&str> = sentence_str.split_whitespace().collect();
            for (w_idx, word_str) in words.iter().enumerate() {
                let w_noise = (((seed + w_idx as u64 * 11) % 19) as f64 - 9.0) * 0.015;
                let w_score = (s_score + w_noise).clamp(0.0, 5.0);

                xml.push_str(&format!(
                    "<word content=\"{}\" total_score=\"{:.2}\" dp_message=\"0\">",
                    escape_xml(word_str), w_score
                ));

                for ph in phonemes(word_str) {
                    xml.push_str(&format!("<phoneme content=\"{}\"/>", escape_xml(&ph)));
                }
                xml.push_str("</word>");
            }
            xml.push_str("</sentence>");
        }

        xml.push_str("</rec_paper>");
        xml.push_str(&format!("</{category_name}>"));
        xml.push_str("</xml_result>");

        xml
    })
}
