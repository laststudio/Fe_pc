use std::io::Read;
use std::time::Duration;

use crate::workflow::tts::settings::CloudflareConfig;
use crate::workflow::tts::types::{AudioFormat, TtsEngine, TtsRequest, TtsResponse};

/// Cloudflare Workers AI TTS 适配器
#[derive(Clone, Debug)]
pub struct CloudflareAdapter {
    pub config: CloudflareConfig,
}

impl CloudflareAdapter {
    pub fn new(config: CloudflareConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: CloudflareConfig::default(),
        }
    }

    /// 获取或构建 Cloudflare Workers AI 执行端点 URL
    pub fn resolve_endpoint(&self) -> Result<String, String> {
        let ep = self.config.endpoint_url.trim();
        if !ep.is_empty() {
            return Ok(ep.to_string());
        }

        let account_id = self.config.account_id.trim();
        if account_id.is_empty() {
            return Err("Cloudflare Workers AI 需要配置 account_id 或完整的 endpoint_url".to_string());
        }

        let model = if !self.config.model.trim().is_empty() {
            self.config.model.trim()
        } else {
            "@cf/deepgram/aura-1"
        };

        Ok(format!(
            "https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/run/{model}"
        ))
    }
}

impl Default for CloudflareAdapter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl TtsEngine for CloudflareAdapter {
    fn provider_id(&self) -> &'static str {
        "cloudflare"
    }

    fn display_name(&self) -> &'static str {
        "Cloudflare Workers AI"
    }

    fn supports_voice_clone(&self) -> bool {
        false
    }

    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        let clean_text = req.text.trim();
        if clean_text.is_empty() {
            return Err("Cloudflare AI TTS 待朗读文本为空".to_string());
        }

        let token = self.config.api_token.trim();
        if token.is_empty() {
            return Err("Cloudflare Workers AI 需要配置 API Token".to_string());
        }

        let endpoint_url = self.resolve_endpoint()?;

        let mut payload = serde_json::json!({
            "text": clean_text
        });

        if let Some(voice) = req.voice.as_deref().filter(|s| !s.trim().is_empty()) {
            payload["speaker"] = serde_json::Value::String(voice.to_string());
        }

        let resp = ureq::post(&endpoint_url)
            .timeout(Duration::from_secs(30))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_json(payload);

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("Cloudflare AI 请求失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("Cloudflare AI 网络连接失败: {e}"));
            }
        };

        let content_type = resp.header("content-type").unwrap_or("").to_lowercase();
        if content_type.contains("json") {
            let body: serde_json::Value = resp
                .into_json()
                .map_err(|e| format!("解析 Cloudflare 错误响应失败: {e}"))?;
            if let Some(errors) = body.get("errors") {
                return Err(format!("Cloudflare AI 返回错误: {errors}"));
            }
            if let Some(msg) = body.get("messages") {
                return Err(format!("Cloudflare AI 信息: {msg}"));
            }
            return Err(format!("Cloudflare AI 未返回二进制音频流: {body}"));
        }

        // 直接读取二进制音频流
        let mut audio_bytes = Vec::new();
        resp.into_reader()
            .read_to_end(&mut audio_bytes)
            .map_err(|e| format!("读取 Cloudflare 音频数据失败: {e}"))?;

        if audio_bytes.is_empty() {
            return Err("Cloudflare AI 返回空音频数据".to_string());
        }

        let format = if audio_bytes.starts_with(b"RIFF") {
            AudioFormat::Wav
        } else {
            AudioFormat::Mp3
        };

        Ok(TtsResponse::new(audio_bytes, format, "cloudflare"))
    }
}
