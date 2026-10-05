use std::time::Duration;
use base64::Engine;

use crate::workflow::tts::settings::MimoConfig;
use crate::workflow::tts::types::{AudioFormat, TtsEngine, TtsRequest, TtsResponse};

/// 小米 MiMo 语音克隆 TTS 适配器
#[derive(Clone, Debug)]
pub struct MimoAdapter {
    pub config: MimoConfig,
}

impl MimoAdapter {
    pub fn new(config: MimoConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: MimoConfig::default(),
        }
    }
}

impl Default for MimoAdapter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl TtsEngine for MimoAdapter {
    fn provider_id(&self) -> &'static str {
        "mimo"
    }

    fn display_name(&self) -> &'static str {
        "小米 MiMo 语音克隆"
    }

    fn supports_voice_clone(&self) -> bool {
        true
    }

    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        let clean_text = req.text.trim();
        if clean_text.is_empty() {
            return Err("MiMo 语音克隆文本为空".to_string());
        }

        let key = self.config.api_key.trim();
        if key.is_empty() {
            return Err("MiMo 语音克隆需要有效的 API Key".to_string());
        }

        let endpoint_url = if self.config.endpoint_url.trim().is_empty() {
            "https://api.xiaomimimo.com/v1/chat/completions"
        } else {
            self.config.endpoint_url.trim()
        };

        let model_name = if !self.config.model.trim().is_empty() {
            self.config.model.trim()
        } else {
            "mimo-v2.5-tts-voiceclone"
        };

        // 获取或构建 DataURL
        let voice_uri = if let Some(ref_bytes) = req.reference_audio.as_deref().filter(|b| !b.is_empty()) {
            let ref_b64 = base64::engine::general_purpose::STANDARD.encode(ref_bytes);
            if ref_bytes.starts_with(b"RIFF") {
                format!("data:audio/wav;base64,{ref_b64}")
            } else if ref_bytes.starts_with(b"ID3")
                || (ref_bytes.len() > 2 && ref_bytes[0] == 0xFF && (ref_bytes[1] & 0xE0) == 0xE0)
            {
                format!("data:audio/mp3;base64,{ref_b64}")
            } else {
                format!("data:audio/wav;base64,{ref_b64}")
            }
        } else if let Some(v) = req.voice.as_deref().filter(|s| !s.trim().is_empty()) {
            v.to_string()
        } else if !self.config.voice.trim().is_empty() {
            self.config.voice.trim().to_string()
        } else {
            return Err("MiMo 语音克隆需要提供参考音频或有效 voice DataURL".to_string());
        };

        let payload = serde_json::json!({
            "model": model_name,
            "messages": [
                {
                    "role": "assistant",
                    "content": clean_text
                }
            ],
            "audio": {
                "format": "mp3",
                "voice": voice_uri
            },
            "stream": false
        });

        let resp = ureq::post(endpoint_url)
            .timeout(Duration::from_secs(45))
            .set("Authorization", &format!("Bearer {key}"))
            .set("api-key", key)
            .set("Content-Type", "application/json")
            .send_json(payload);

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("MiMo 克隆请求失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("MiMo 克隆网络连接失败: {e}"));
            }
        };

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析 MiMo 响应 JSON 失败: {e}"))?;

        if let Some(err) = body.get("error") {
            return Err(format!("MiMo 接口返回错误: {err}"));
        }

        let audio_b64 = body
            .pointer("/choices/0/message/audio/data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("MiMo 响应中缺少 choices[0].message.audio.data: {body}"))?;

        let audio_bytes = base64::engine::general_purpose::STANDARD
            .decode(audio_b64)
            .map_err(|e| format!("解码 MiMo 音频 Base64 失败: {e}"))?;

        if audio_bytes.is_empty() {
            return Err("MiMo 返回空音频数据".to_string());
        }

        Ok(TtsResponse::new(audio_bytes, AudioFormat::Mp3, "mimo"))
    }
}
