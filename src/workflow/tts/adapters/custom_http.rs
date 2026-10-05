use std::io::Read;
use std::time::Duration;

use crate::workflow::tts::settings::CustomHttpConfig;
use crate::workflow::tts::types::{AudioFormat, TtsEngine, TtsRequest, TtsResponse};

/// 自定义 HTTP 适配器（兼容标准 OpenAI / Kokoro / TTS.ai / Gradio 规范）
#[derive(Clone, Debug)]
pub struct CustomHttpAdapter {
    pub config: CustomHttpConfig,
}

impl CustomHttpAdapter {
    pub fn new(config: CustomHttpConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: CustomHttpConfig::default(),
        }
    }

    /// 解析 SSE 流式响应中的音频 Base64 数据块
    pub fn parse_sse_audio(payload: &str) -> Result<Vec<u8>, String> {
        use base64::Engine;
        let mut audio = Vec::new();
        for line in payload.lines() {
            let line = line.trim();
            if !line.starts_with("data:") {
                continue;
            }
            let data = line[5..].trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(err) = event.get("error") {
                    return Err(format!("HTTP TTS 流式错误: {err}"));
                }
                if let Some(chunk) = event.get("chunk").and_then(|v| v.as_str())
                    && let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(chunk)
                {
                    audio.extend_from_slice(&bytes);
                }
            }
        }
        if audio.is_empty() {
            Err("SSE 流中未解析到有效音频数据".to_string())
        } else {
            Ok(audio)
        }
    }
}

impl Default for CustomHttpAdapter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl TtsEngine for CustomHttpAdapter {
    fn provider_id(&self) -> &'static str {
        "custom_http"
    }

    fn display_name(&self) -> &'static str {
        "自定义 HTTP (OpenAI 兼容)"
    }

    fn supports_voice_clone(&self) -> bool {
        false
    }

    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        let clean_text = req.text.trim();
        if clean_text.is_empty() {
            return Err("HTTP TTS 待朗读文本为空".to_string());
        }

        let endpoint_url = if self.config.endpoint_url.trim().is_empty() {
            "https://api.openai.com/v1/audio/speech"
        } else {
            self.config.endpoint_url.trim()
        };

        let voice = req
            .voice
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(&self.config.voice);

        let model = if !self.config.model.trim().is_empty() {
            self.config.model.trim()
        } else {
            "tts-1"
        };

        let speed = req.speed.unwrap_or(self.config.speed);

        let mut http_req = ureq::post(endpoint_url).timeout(Duration::from_secs(30));

        let key = self.config.api_key.trim();
        if !key.is_empty() {
            http_req = http_req.set("Authorization", &format!("Bearer {key}"));
        }

        let payload = serde_json::json!({
            "model": model,
            "input": clean_text,
            "text": clean_text,
            "voice": voice,
            "response_format": "mp3",
            "format": "mp3",
            "speed": speed
        });

        let resp = match http_req.send_json(payload) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("HTTP TTS 请求失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("HTTP TTS 网络连接失败: {e}"));
            }
        };

        let content_type = resp.header("content-type").unwrap_or("").to_lowercase();
        if content_type.contains("text/event-stream") {
            let body = resp.into_string().map_err(|e| format!("读取 SSE 响应失败: {e}"))?;
            let audio_bytes = Self::parse_sse_audio(&body)?;
            let format = if audio_bytes.starts_with(b"RIFF") { AudioFormat::Wav } else { AudioFormat::Mp3 };
            Ok(TtsResponse::new(audio_bytes, format, "custom_http"))
        } else if content_type.contains("json") {
            let body: serde_json::Value = resp.into_json().map_err(|e| format!("解析 JSON 响应失败: {e}"))?;
            if let Some(b64) = body.get("audio").or_else(|| body.get("chunk")).and_then(|v| v.as_str()) {
                use base64::Engine;
                let audio_bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| format!("Base64 解码音频失败: {e}"))?;
                let format = if audio_bytes.starts_with(b"RIFF") { AudioFormat::Wav } else { AudioFormat::Mp3 };
                Ok(TtsResponse::new(audio_bytes, format, "custom_http"))
            } else if let Some(err) = body.get("error") {
                Err(format!("HTTP TTS 接口返回错误: {err}"))
            } else {
                Err("HTTP TTS 返回的 JSON 中未包含 audio/chunk 音频字段".to_string())
            }
        } else {
            let mut audio_bytes = Vec::new();
            resp.into_reader()
                .read_to_end(&mut audio_bytes)
                .map_err(|e| format!("读取音频二进制流失败: {e}"))?;
            if audio_bytes.is_empty() {
                return Err("HTTP TTS 返回空音频数据".to_string());
            }
            let format = if audio_bytes.starts_with(b"RIFF") { AudioFormat::Wav } else { AudioFormat::Mp3 };
            Ok(TtsResponse::new(audio_bytes, format, "custom_http"))
        }
    }
}
