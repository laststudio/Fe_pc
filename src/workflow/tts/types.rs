use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 支持的音频编码格式
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AudioFormat {
    #[default]
    Mp3,
    Wav,
    Pcm16k,
}

impl AudioFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Pcm16k => "pcm",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Pcm16k => "audio/L16;rate=16000",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Pcm16k => "pcm",
        }
    }
}

/// 统一 TTS 合成请求结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TtsRequest {
    /// 待朗读文本内容
    pub text: String,
    /// 目标发音人标识 / 音色名称
    pub voice: Option<String>,
    /// 朗读语速（相对倍率，如 0.8 - 1.5，基准为 1.0）
    pub speed: Option<f64>,
    /// 朗读音调（半音或百分比偏移）
    pub pitch: Option<i32>,
    /// 0-Shot 克隆参考音频二进制数据（MP3 或 WAV）
    pub reference_audio: Option<Vec<u8>>,
    /// 各提供商特定扩展参数字典
    #[serde(default)]
    pub extra_params: HashMap<String, String>,
}

impl TtsRequest {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            voice: None,
            speed: None,
            pitch: None,
            reference_audio: None,
            extra_params: HashMap::new(),
        }
    }

    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.voice = Some(voice.into());
        self
    }

    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = Some(speed);
        self
    }

    pub fn with_pitch(mut self, pitch: i32) -> Self {
        self.pitch = Some(pitch);
        self
    }

    pub fn with_reference_audio(mut self, audio: Vec<u8>) -> Self {
        self.reference_audio = Some(audio);
        self
    }

    pub fn with_extra(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.extra_params.insert(key.into(), val.into());
        self
    }
}

/// 统一 TTS 合成响应结果
#[derive(Clone, Debug)]
pub struct TtsResponse {
    /// 合成后的原始音频二进制数据
    pub audio_bytes: Vec<u8>,
    /// 音频编码格式
    pub format: AudioFormat,
    /// 音频估算时长（秒）
    pub duration_secs: f64,
    /// 执行合成的底层引擎标识（如 "edge", "siliconflow", "fish_audio" 等）
    pub engine_identifier: String,
}

impl TtsResponse {
    pub fn new(audio_bytes: Vec<u8>, format: AudioFormat, engine_identifier: impl Into<String>) -> Self {
        let duration_secs = crate::workflow::audio_proc::check_audio_validity(&audio_bytes)
            .map(|(d, _)| d)
            .unwrap_or(0.0);
        Self {
            audio_bytes,
            format,
            duration_secs,
            engine_identifier: engine_identifier.into(),
        }
    }

    pub fn with_duration(mut self, duration_secs: f64) -> Self {
        self.duration_secs = duration_secs;
        self
    }
}

/// 多厂商 TTS 引擎统一抽象特征接口
pub trait TtsEngine: Send + Sync {
    /// 提供商唯一标识符（如 "edge", "siliconflow", "fish_audio", "mimo", "minimax", "cloudflare", "custom_http"）
    fn provider_id(&self) -> &'static str;

    /// 人类可读的厂商中文显示名称
    fn display_name(&self) -> &'static str;

    /// 是否原生支持参考音频 0-shot 声音克隆
    fn supports_voice_clone(&self) -> bool;

    /// 执行文本合成
    fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String>;
}
