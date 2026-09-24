use std::sync::Once;
use msedge_tts::tts::client::connect;
use msedge_tts::tts::SpeechConfig;

static INIT_CRYPTO: Once = Once::new();

pub fn ensure_crypto_provider() {
    INIT_CRYPTO.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

pub struct EdgeTtsService;

impl EdgeTtsService {
    /// 合成文本为 24kHz MP3 音频二进制字节流
    #[inline(never)]
    pub fn synthesize(
        text: &str,
        voice_name: Option<&str>,
        rate: Option<i32>,
        pitch: Option<i32>,
    ) -> Result<Vec<u8>, String> {
        ensure_crypto_provider();
        let clean_text = text.trim();
        if clean_text.is_empty() {
            return Err("TTS 待朗读文本为空".to_string());
        }

        let config = crate::themida_falcon_tiny!({
            let voice = voice_name.unwrap_or("en-US-JennyNeural").to_string();
            SpeechConfig {
                voice_name: voice,
                audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
                pitch: pitch.unwrap_or(0),
                rate: rate.unwrap_or(-5),
                volume: 0,
            }
        });

        let mut last_error = String::new();
        for attempt in 1..=3 {
            match connect() {
                Ok(mut client) => {
                    match client.synthesize(clean_text, &config) {
                        Ok(audio) => {
                            if !audio.audio_bytes.is_empty() {
                                return Ok(audio.audio_bytes.clone());
                            } else {
                                last_error = "Edge TTS 返回空音频数据".to_string();
                            }
                        }
                        Err(e) => {
                            last_error = format!("Edge TTS 合成错误: {e:?}");
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("Edge TTS 连接失败: {e:?}");
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(300 * attempt));
        }

        Err(format!("TTS 语音合成失败 (重试3次): {last_error}"))
    }
}
