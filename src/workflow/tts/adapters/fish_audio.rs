use std::io::Read;
use std::time::Duration;

use crate::workflow::tts::settings::FishAudioConfig;
use crate::workflow::tts::types::{AudioFormat, TtsEngine, TtsRequest, TtsResponse};

/// Fish Audio (鱼声) TTS 适配器
#[derive(Clone, Debug)]
pub struct FishAudioAdapter {
    pub config: FishAudioConfig,
}

impl FishAudioAdapter {
    pub fn new(config: FishAudioConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: FishAudioConfig::default(),
        }
    }

    /// 构造标准 multipart/form-data 二进制负载（包含参考音频 0-shot 极速克隆）
    pub fn build_multipart_payload(
        text: &str,
        model: &str,
        reference_audio: &[u8],
        reference_id: Option<&str>,
        boundary: &str,
    ) -> Vec<u8> {
        let mut body = Vec::new();

        // 字段: text
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"text\"\r\n\r\n{text}\r\n")
                .as_bytes(),
        );

        // 字段: model
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n{model}\r\n")
                .as_bytes(),
        );

        // 字段: format
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nmp3\r\n")
                .as_bytes(),
        );

        // 字段: reference_id (可选音色模板 ID)
        if let Some(ref_id) = reference_id.filter(|s| !s.trim().is_empty()) {
            body.extend_from_slice(
                format!("--{boundary}\r\nContent-Disposition: form-data; name=\"reference_id\"\r\n\r\n{ref_id}\r\n")
                    .as_bytes(),
            );
        }

        // 字段: reference_audio (参考音频二进制附件)
        let (filename, content_type) = if reference_audio.starts_with(b"RIFF") {
            ("reference.wav", "audio/wav")
        } else {
            ("reference.mp3", "audio/mpeg")
        };

        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"reference_audio\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(reference_audio);
        body.extend_from_slice(b"\r\n");

        // 结束 boundary
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        body
    }
}

impl Default for FishAudioAdapter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl TtsEngine for FishAudioAdapter {
    fn provider_id(&self) -> &'static str {
        "fish_audio"
    }

    fn display_name(&self) -> &'static str {
        "Fish Audio (鱼声)"
    }

    fn supports_voice_clone(&self) -> bool {
        true
    }

    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        let clean_text = req.text.trim();
        if clean_text.is_empty() {
            return Err("Fish Audio 待朗读文本为空".to_string());
        }

        let endpoint = if self.config.endpoint_url.trim().is_empty() {
            "https://api.fish.audio/v1/tts"
        } else {
            self.config.endpoint_url.trim()
        };

        // 确定模型：未配置 key 时支持免 Key 免费模型 s2.1-pro-free；有 key 时默认 s2.1-pro
        let model = if !self.config.model.trim().is_empty() {
            self.config.model.trim()
        } else if self.config.api_key.trim().is_empty() {
            "s2.1-pro-free"
        } else {
            "s2.1-pro"
        };

        let voice_id = req
            .voice
            .as_deref()
            .or_else(|| {
                if !self.config.voice_id.trim().is_empty() {
                    Some(self.config.voice_id.as_str())
                } else {
                    None
                }
            });

        let mut http_req = ureq::post(endpoint).timeout(Duration::from_secs(45));

        if !self.config.api_key.trim().is_empty() {
            http_req = http_req.set("Authorization", &format!("Bearer {}", self.config.api_key.trim()));
        }

        let resp = if let Some(ref_audio) = req.reference_audio.as_deref().filter(|b| !b.is_empty()) {
            // 采用 Multipart 传递参考音频实现 0-shot 极速克隆
            let boundary = format!("----TTSFishBoundary{}", uuid::Uuid::new_v4().simple());
            let body = Self::build_multipart_payload(clean_text, model, ref_audio, voice_id, &boundary);
            http_req
                .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
                .send_bytes(&body)
        } else {
            // 常规纯文本合成（JSON 模式）
            let mut payload = serde_json::json!({
                "text": clean_text,
                "format": "mp3",
                "model": model,
            });
            if let Some(vid) = voice_id {
                payload["reference_id"] = serde_json::Value::String(vid.to_string());
            }
            if let Some(spd) = req.speed {
                payload["speed"] = serde_json::Value::from(spd);
            }
            http_req
                .set("Content-Type", "application/json")
                .send_json(payload)
        };

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("Fish Audio 请求失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("Fish Audio 网络连接失败: {e}"));
            }
        };

        let content_type = resp.header("content-type").unwrap_or("").to_lowercase();
        if content_type.contains("json") {
            let body: serde_json::Value = resp
                .into_json()
                .map_err(|e| format!("解析 Fish Audio 响应失败: {e}"))?;
            if let Some(err) = body.get("error").or_else(|| body.get("message")) {
                return Err(format!("Fish Audio 接口返回错误: {err}"));
            }
            if let Some(b64) = body.get("audio").and_then(|v| v.as_str()) {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| format!("Base64 解码 Fish Audio 音频失败: {e}"))?;
                return Ok(TtsResponse::new(bytes, AudioFormat::Mp3, "fish_audio"));
            }
            return Err(format!("Fish Audio 响应未包含音频数据: {body}"));
        }

        let mut audio_bytes = Vec::new();
        resp.into_reader()
            .read_to_end(&mut audio_bytes)
            .map_err(|e| format!("读取 Fish Audio 音频二进制流失败: {e}"))?;

        if audio_bytes.is_empty() {
            return Err("Fish Audio 返回空音频数据".to_string());
        }

        Ok(TtsResponse::new(audio_bytes, AudioFormat::Mp3, "fish_audio"))
    }
}
