use std::sync::Once;
use std::time::Duration;
use msedge_tts::tts::client::connect;
use msedge_tts::tts::SpeechConfig;

use crate::workflow::tts::settings::EdgeConfig;
use crate::workflow::tts::types::{AudioFormat, TtsEngine, TtsRequest, TtsResponse};

static INIT_CRYPTO: Once = Once::new();

pub fn ensure_crypto_provider() {
    INIT_CRYPTO.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Edge 预设高质量自然神经网络音色清单
pub const EDGE_PRESET_VOICES: &[(&str, &str)] = &[
    ("en-US-JennyNeural", "美式英语 - Jenny (默认/推荐自然女声)"),
    ("en-US-GuyNeural", "美式英语 - Guy (标准清晰男声)"),
    ("en-US-AvaNeural", "美式英语 - Ava (高表现力自然女声)"),
    ("en-US-AndrewNeural", "美式英语 - Andrew (温和自然男声)"),
    ("en-US-AriaNeural", "美式英语 - Aria (标准播音女声)"),
    ("en-US-BrianNeural", "美式英语 - Brian (深沉自然男声)"),
    ("zh-CN-XiaoxiaoNeural", "中文普通话 - 晓晓 (自然温柔女声)"),
    ("zh-CN-YunxiNeural", "中文普通话 - 云希 (阳光自然男声)"),
];

/// 微软 Edge 官方 TTS 适配器
#[derive(Clone, Debug)]
pub struct EdgeAdapter {
    pub config: EdgeConfig,
}

impl EdgeAdapter {
    pub fn new(config: EdgeConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self {
            config: EdgeConfig::default(),
        }
    }
}

impl Default for EdgeAdapter {
    fn default() -> Self {
        Self::with_default_config()
    }
}

impl TtsEngine for EdgeAdapter {
    fn provider_id(&self) -> &'static str {
        "edge"
    }

    fn display_name(&self) -> &'static str {
        "微软 Edge 官方高清语音"
    }

    fn supports_voice_clone(&self) -> bool {
        false
    }

    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        ensure_crypto_provider();

        let clean_text = req.text.trim();
        if clean_text.is_empty() {
            return Err("Edge TTS 待朗读文本为空".to_string());
        }

        let voice = req
            .voice
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(&self.config.voice);

        let pitch = req.pitch.unwrap_or(self.config.pitch);
        let rate = if let Some(spd) = req.speed {
            // 将 1.0 比例语速转换为百分比 (-100% ~ +100%)
            ((spd - 1.0) * 50.0).round() as i32
        } else {
            self.config.rate
        };

        let speech_config = build_speech_config(voice, pitch, rate);

        let mut last_error = String::new();
        let max_retries = 3;

        for attempt in 1..=max_retries {
            match connect() {
                Ok(mut client) => match client.synthesize(clean_text, &speech_config) {
                    Ok(audio) => {
                        if !audio.audio_bytes.is_empty() {
                            return Ok(TtsResponse::new(
                                audio.audio_bytes,
                                AudioFormat::Mp3,
                                "edge",
                            ));
                        } else {
                            last_error = "Edge TTS 返回空音频数据".to_string();
                        }
                    }
                    Err(e) => {
                        last_error = format!("Edge TTS 合成错误: {e:?}");
                    }
                },
                Err(e) => {
                    last_error = format!("Edge TTS 连接失败: {e:?}");
                }
            }
            std::thread::sleep(Duration::from_millis(300 * attempt));
        }

        Err(format!("Edge TTS 语音合成失败 (已自动重试3次): {last_error}"))
    }
}

#[inline(never)]
fn build_speech_config(voice: &str, pitch: i32, rate: i32) -> SpeechConfig {
    crate::themida_falcon_tiny!({
        SpeechConfig {
            voice_name: voice.to_string(),
            audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
            pitch,
            rate,
            volume: 0,
        }
    })
}
