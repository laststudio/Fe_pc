use std::time::Duration;
use base64::Engine;

use crate::workflow::tts::settings::MiniMaxConfig;
use crate::workflow::tts::types::{AudioFormat, TtsEngine, TtsRequest, TtsResponse};

/// MiniMax 预设音色清单
pub const MINIMAX_PRESET_VOICES: &[(&str, &str)] = &[
    ("male-qn-qingse", "青涩青年男声 (推荐/通用)"),
    ("female-shaonv", "活力少女音"),
    ("female-yujie", "知性御姐音"),
    ("presenter_male", "播音主持男声"),
    ("presenter_female", "播音主持女声"),
];

/// MiniMax 海螺语音 TTS 适配器
#[derive(Clone, Debug)]
pub struct MiniMaxAdapter {
    pub config: MiniMaxConfig,
}

impl MiniMaxAdapter {
    pub fn new(config: MiniMaxConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: MiniMaxConfig::default(),
        }
    }

    /// 解码 MiniMax 响应中的音频数据（自动兼容 Hex 编码或 Base64 编码）
    pub fn decode_audio_payload(raw_audio_str: &str) -> Result<Vec<u8>, String> {
        let trimmed = raw_audio_str.trim();
        if trimmed.is_empty() {
            return Err("MiniMax 音频字段内容为空".to_string());
        }

        // 1. 尝试 Hex 十六进制解码 (MiniMax 官方 T2A V2 标准返回格式)
        if trimmed.len().is_multiple_of(2) && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            let mut hex_bytes = Vec::with_capacity(trimmed.len() / 2);
            let mut ok = true;
            for i in (0..trimmed.len()).step_by(2) {
                if let Ok(byte) = u8::from_str_radix(&trimmed[i..i + 2], 16) {
                    hex_bytes.push(byte);
                } else {
                    ok = false;
                    break;
                }
            }
            if ok && !hex_bytes.is_empty() {
                return Ok(hex_bytes);
            }
        }

        // 2. 尝试 Base64 解码 (部分网关或版本返回格式)
        if let Ok(b64_bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed)
            && !b64_bytes.is_empty()
        {
            return Ok(b64_bytes);
        }

        Err("无法解析 MiniMax 音频数据 (既非有效 Hex 也非有效 Base64)".to_string())
    }
}

impl Default for MiniMaxAdapter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl TtsEngine for MiniMaxAdapter {
    fn provider_id(&self) -> &'static str {
        "minimax"
    }

    fn display_name(&self) -> &'static str {
        "MiniMax 海螺语音"
    }

    fn supports_voice_clone(&self) -> bool {
        false
    }

    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        let clean_text = req.text.trim();
        if clean_text.is_empty() {
            return Err("MiniMax 待朗读文本为空".to_string());
        }

        let key = self.config.api_key.trim();
        if key.is_empty() {
            return Err("MiniMax 语音合成需要配置有效的 API Key".to_string());
        }

        let mut endpoint = if self.config.endpoint_url.trim().is_empty() {
            "https://api.minimax.io/v1/t2a_v2".to_string()
        } else {
            self.config.endpoint_url.trim().to_string()
        };

        // 如果存在 GroupId，附加在 URL Query 参数中
        let group_id = req
            .extra_params
            .get("group_id")
            .map(|s| s.as_str())
            .unwrap_or(self.config.group_id.trim());

        if !group_id.is_empty() && !endpoint.contains("GroupId=") {
            let sep = if endpoint.contains('?') { "&" } else { "?" };
            endpoint = format!("{endpoint}{sep}GroupId={group_id}");
        }

        let model = if !self.config.model.trim().is_empty() {
            self.config.model.trim()
        } else {
            "speech-2.8-turbo"
        };

        let voice_id = req
            .voice
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(&self.config.voice_id);

        let speed = req.speed.unwrap_or(self.config.speed);
        let pitch = req.pitch.unwrap_or(self.config.pitch);

        let payload = build_minimax_payload(model, clean_text, voice_id, speed, pitch);

        let resp = ureq::post(&endpoint)
            .timeout(Duration::from_secs(45))
            .set("Authorization", &format!("Bearer {key}"))
            .set("Content-Type", "application/json")
            .send_json(payload);

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("MiniMax 请求失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("MiniMax 网络连接失败: {e}"));
            }
        };

        let body: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析 MiniMax 响应 JSON 失败: {e}"))?;

        // 校验基础响应状态码
        if let Some(status_code) = body.pointer("/base_resp/status_code").and_then(|v| v.as_i64())
            && status_code != 0
        {
            let status_msg = body
                .pointer("/base_resp/status_msg")
                .and_then(|v| v.as_str())
                .unwrap_or("未知错误");
            return Err(format!("MiniMax 接口返回错误 (代码 {status_code}): {status_msg}"));
        }

        if let Some(err) = body.get("error").or_else(|| body.get("message")) {
            return Err(format!("MiniMax 错误: {err}"));
        }

        let audio_str = body
            .pointer("/data/audio")
            .or_else(|| body.get("audio"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("MiniMax 响应中未找到 data.audio 音频字段: {body}"))?;

        let audio_bytes = Self::decode_audio_payload(audio_str)?;

        Ok(TtsResponse::new(audio_bytes, AudioFormat::Mp3, "minimax"))
    }
}

#[inline(never)]
fn build_minimax_payload(
    model: &str,
    clean_text: &str,
    voice_id: &str,
    speed: f64,
    pitch: i32,
) -> serde_json::Value {
    crate::themida_falcon_tiny!({
        serde_json::json!({
            "model": model,
            "text": clean_text,
            "stream": false,
            "voice_setting": {
                "voice_id": voice_id,
                "speed": speed,
                "pitch": pitch,
                "vol": 1.0
            },
            "audio_setting": {
                "sample_rate": 32000,
                "bitrate": 128000,
                "format": "mp3",
                "channel": 1
            }
        })
    })
}
