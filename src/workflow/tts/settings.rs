use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 微软 Edge TTS 专属配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeConfig {
    pub voice: String,
    pub rate: i32,
    pub pitch: i32,
    pub timeout_secs: u64,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            voice: "en-US-JennyNeural".to_string(),
            rate: -5,
            pitch: 0,
            timeout_secs: 30,
        }
    }
}

/// 硅基流动 (SiliconFlow) 专属配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiliconFlowConfig {
    pub api_key: String,
    pub endpoint_url: String,
    pub model: String,
    pub voice: String,
    pub speed: f64,
}

impl Default for SiliconFlowConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint_url: "https://api.siliconflow.cn/v1/audio/speech".to_string(),
            model: "FunAudioLLM/CosyVoice2-0.5B".to_string(),
            voice: "alex".to_string(),
            speed: 1.0,
        }
    }
}

/// Fish Audio (鱼声) 专属配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FishAudioConfig {
    pub api_key: String,
    pub endpoint_url: String,
    pub model: String,
    pub voice_id: String,
    pub latency: String,
}

impl Default for FishAudioConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint_url: "https://api.fish.audio/v1/tts".to_string(),
            model: "s2.1-pro-free".to_string(),
            voice_id: String::new(),
            latency: "normal".to_string(),
        }
    }
}

/// 小米 MiMo 语音克隆专属配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MimoConfig {
    pub api_key: String,
    pub endpoint_url: String,
    pub model: String,
    pub voice: String,
}

impl Default for MimoConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint_url: "https://api.xiaomimimo.com/v1/chat/completions".to_string(),
            model: "mimo-v2.5-tts-voiceclone".to_string(),
            voice: String::new(),
        }
    }
}

/// MiniMax 海螺语音专属配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiniMaxConfig {
    pub api_key: String,
    pub group_id: String,
    pub endpoint_url: String,
    pub model: String,
    pub voice_id: String,
    pub speed: f64,
    pub pitch: i32,
}

impl Default for MiniMaxConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            group_id: String::new(),
            endpoint_url: "https://api.minimax.io/v1/t2a_v2".to_string(),
            model: "speech-2.8-turbo".to_string(),
            voice_id: "male-qn-qingse".to_string(),
            speed: 1.0,
            pitch: 0,
        }
    }
}

/// Cloudflare Workers AI 专属配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudflareConfig {
    pub api_token: String,
    pub account_id: String,
    pub endpoint_url: String,
    pub model: String,
}

impl Default for CloudflareConfig {
    fn default() -> Self {
        Self {
            api_token: String::new(),
            account_id: String::new(),
            endpoint_url: String::new(),
            model: "@cf/deepgram/aura-1".to_string(),
        }
    }
}

/// 自定义 HTTP (兼容 OpenAI / Kokoro / TTS.ai) 专属配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomHttpConfig {
    pub endpoint_url: String,
    pub api_key: String,
    pub model: String,
    pub voice: String,
    pub speed: f64,
}

impl Default for CustomHttpConfig {
    fn default() -> Self {
        Self {
            endpoint_url: "https://api.openai.com/v1/audio/speech".to_string(),
            api_key: String::new(),
            model: "tts-1".to_string(),
            voice: "alloy".to_string(),
            speed: 1.0,
        }
    }
}

/// 多厂商 TTS 矩阵全局设置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TtsMatrixSettings {
    /// 当前启用的首选提供商标识 ("edge", "siliconflow", "fish_audio", "mimo", "minimax", "cloudflare", "custom_http")
    #[serde(default = "default_active_provider")]
    pub active_provider: String,

    /// 是否开启故障自动容灾降级（当第三方提供商异常时无感回退到微软 Edge）
    #[serde(default = "default_enable_fallback")]
    pub enable_auto_fallback: bool,

    /// 是否混入真实麦克风粉红底噪 (Pink Noise)
    #[serde(default = "default_enable_noise")]
    pub enable_pink_noise: bool,

    /// 麦克风粉红底噪注入分贝强度（dB，默认 -35.0）
    #[serde(default = "default_noise_level")]
    pub noise_level_db: f32,

    /// 全局克隆参考音频本地文件路径
    #[serde(default)]
    pub clone_reference_path: String,

    /// 各提供商专属独立配置对象（切换预设时各参数独立保存，绝不互相覆盖！）
    #[serde(default)]
    pub edge: EdgeConfig,
    #[serde(default)]
    pub siliconflow: SiliconFlowConfig,
    #[serde(default)]
    pub fish_audio: FishAudioConfig,
    #[serde(default)]
    pub mimo: MimoConfig,
    #[serde(default)]
    pub minimax: MiniMaxConfig,
    #[serde(default)]
    pub cloudflare: CloudflareConfig,
    #[serde(default)]
    pub custom_http: CustomHttpConfig,
}

fn default_active_provider() -> String {
    "edge".to_string()
}

fn default_enable_fallback() -> bool {
    true
}

fn default_enable_noise() -> bool {
    true
}

fn default_noise_level() -> f32 {
    -35.0
}

impl Default for TtsMatrixSettings {
    fn default() -> Self {
        Self {
            active_provider: default_active_provider(),
            enable_auto_fallback: default_enable_fallback(),
            enable_pink_noise: default_enable_noise(),
            noise_level_db: default_noise_level(),
            clone_reference_path: String::new(),
            edge: EdgeConfig::default(),
            siliconflow: SiliconFlowConfig::default(),
            fish_audio: FishAudioConfig::default(),
            mimo: MimoConfig::default(),
            minimax: MiniMaxConfig::default(),
            cloudflare: CloudflareConfig::default(),
            custom_http: CustomHttpConfig::default(),
        }
    }
}

impl TtsMatrixSettings {
    /// 获取持久化 JSON 配置文件完整路径
    /// 优先使用 `%LOCALAPPDATA%\fepc\fepc_tts_settings.json`，缺失时使用当前目录
    pub fn config_file_path() -> PathBuf {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let dir = PathBuf::from(local_app_data).join("fepc");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("fepc_tts_settings.json")
        } else {
            PathBuf::from("fepc_tts_settings.json")
        }
    }

    /// 从本地磁盘加载设置；文件不存在或损坏时自动返回安全默认配置
    pub fn load() -> Self {
        let path = Self::config_file_path();
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(settings) = serde_json::from_str::<Self>(&content)
        {
            return settings;
        }
        Self::default()
    }

    /// 将当前设置保存持久化至磁盘
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化 TTS 矩阵配置失败: {e}"))?;
        std::fs::write(&path, json_str)
            .map_err(|e| format!("写入 TTS 矩阵配置文件失败 ({path:?}): {e}"))
    }
}
