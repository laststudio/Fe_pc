use std::io::Read;
use std::time::Duration;

use crate::workflow::tts::settings::SiliconFlowConfig;
use crate::workflow::tts::types::{AudioFormat, TtsEngine, TtsRequest, TtsResponse};

/// 硅基流动预设推荐模型
pub const SILICONFLOW_PRESET_MODELS: &[(&str, &str)] = &[
    ("FunAudioLLM/CosyVoice2-0.5B", "阿里开源 CosyVoice2 高表现力多语言模型 (推荐)"),
    ("fishaudio/fish-speech-1.5", "Fish Speech 1.5 极速流式零样本克隆模型"),
];

/// 硅基流动 (SiliconFlow) TTS 适配器
#[derive(Clone, Debug)]
pub struct SiliconFlowAdapter {
    pub config: SiliconFlowConfig,
}

impl SiliconFlowAdapter {
    pub fn new(config: SiliconFlowConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: SiliconFlowConfig::default(),
        }
    }

    /// 根据配置端点计算音频音色注册接口 URL
    pub fn get_voice_upload_url(&self) -> String {
        let ep = self.config.endpoint_url.trim();
        if ep.is_empty() {
            "https://api.siliconflow.cn/v1/uploads/audio/voice".to_string()
        } else if ep.ends_with("/audio/speech") {
            ep.replace("/audio/speech", "/uploads/audio/voice")
        } else if let Some(idx) = ep.rfind("/v1/") {
            format!("{}/v1/uploads/audio/voice", &ep[..idx])
        } else {
            "https://api.siliconflow.cn/v1/uploads/audio/voice".to_string()
        }
    }

    /// 将参考音频二进制上传至硅基流动注册为音色 URI
    pub fn upload_reference_voice(
        &self,
        reference_audio: &[u8],
        model: &str,
    ) -> Result<String, String> {
        let upload_url = self.get_voice_upload_url();
        let key = self.config.api_key.trim();
        if key.is_empty() {
            return Err("硅基流动声音克隆需要有效的 API Key".to_string());
        }

        let boundary = format!("----TTSBoundarySF{}", uuid::Uuid::new_v4().simple());
        let mut body = Vec::new();

        // 字段: model
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n{model}\r\n")
                .as_bytes(),
        );

        // 字段: customName
        let custom_name = format!("fe_clone_{}", uuid::Uuid::new_v4().simple());
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"customName\"\r\n\r\n{custom_name}\r\n")
                .as_bytes(),
        );

        // 字段: file
        let (filename, content_type) = if reference_audio.starts_with(b"RIFF") {
            ("voice.wav", "audio/wav")
        } else {
            ("voice.mp3", "audio/mpeg")
        };
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(reference_audio);
        body.extend_from_slice(b"\r\n");

        // 结束 boundary
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let resp = ureq::post(&upload_url)
            .timeout(Duration::from_secs(30))
            .set("Authorization", &format!("Bearer {key}"))
            .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .send_bytes(&body);

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("硅基流动音色上传失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("硅基流动音色上传网络错误: {e}"));
            }
        };

        let json_res: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析硅基流动音色响应失败: {e}"))?;

        if let Some(err) = json_res.get("error").or_else(|| json_res.get("message")) {
            return Err(format!("硅基流动注册音色返回错误: {err}"));
        }

        let uri = json_res
            .get("uri")
            .or_else(|| json_res.pointer("/data/uri"))
            .or_else(|| json_res.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("硅基流动上传响应缺少 uri: {json_res}"))?;

        Ok(uri.to_string())
    }
}

impl Default for SiliconFlowAdapter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl TtsEngine for SiliconFlowAdapter {
    fn provider_id(&self) -> &'static str {
        "siliconflow"
    }

    fn display_name(&self) -> &'static str {
        "硅基流动 (SiliconFlow)"
    }

    fn supports_voice_clone(&self) -> bool {
        true
    }

    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        let clean_text = req.text.trim();
        if clean_text.is_empty() {
            return Err("硅基流动 TTS 待朗读文本为空".to_string());
        }

        let key = self.config.api_key.trim();
        if key.is_empty() {
            return Err("硅基流动 TTS 需要配置 API Key".to_string());
        }

        let endpoint = if self.config.endpoint_url.trim().is_empty() {
            "https://api.siliconflow.cn/v1/audio/speech"
        } else {
            self.config.endpoint_url.trim()
        };

        let model = if !self.config.model.trim().is_empty() {
            self.config.model.trim()
        } else {
            "FunAudioLLM/CosyVoice2-0.5B"
        };

        // 如果提供了 reference_audio，先上传注册 voice URI 并使用
        let voice_to_use = if let Some(ref_bytes) = req.reference_audio.as_deref().filter(|b| !b.is_empty()) {
            self.upload_reference_voice(ref_bytes, model)?
        } else if let Some(v) = req.voice.as_deref().filter(|s| !s.trim().is_empty()) {
            v.to_string()
        } else if !self.config.voice.trim().is_empty() {
            self.config.voice.trim().to_string()
        } else {
            "alex".to_string()
        };

        let speed = req.speed.unwrap_or(self.config.speed);

        // OpenAI 兼容请求格式
        let payload = serde_json::json!({
            "model": model,
            "input": clean_text,
            "voice": voice_to_use,
            "response_format": "mp3",
            "speed": speed
        });

        let resp = ureq::post(endpoint)
            .timeout(Duration::from_secs(45))
            .set("Authorization", &format!("Bearer {key}"))
            .set("Content-Type", "application/json")
            .send_json(payload);

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("硅基流动合成请求失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("硅基流动合成网络连接失败: {e}"));
            }
        };

        let content_type = resp.header("content-type").unwrap_or("").to_lowercase();
        if content_type.contains("json") {
            let body: serde_json::Value = resp
                .into_json()
                .map_err(|e| format!("解析硅基流动响应 JSON 失败: {e}"))?;
            if let Some(err) = body.get("error").or_else(|| body.get("message")) {
                return Err(format!("硅基流动合成错误: {err}"));
            }
            if let Some(b64) = body.get("audio").and_then(|v| v.as_str()) {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| format!("Base64 解码硅基流动音频失败: {e}"))?;
                return Ok(TtsResponse::new(bytes, AudioFormat::Mp3, "siliconflow"));
            }
            return Err(format!("硅基流动响应缺少有效音频: {body}"));
        }

        let mut audio_bytes = Vec::new();
        resp.into_reader()
            .read_to_end(&mut audio_bytes)
            .map_err(|e| format!("读取硅基流动音频数据失败: {e}"))?;

        if audio_bytes.is_empty() {
            return Err("硅基流动返回空音频数据".to_string());
        }

        Ok(TtsResponse::new(audio_bytes, AudioFormat::Mp3, "siliconflow"))
    }
}
