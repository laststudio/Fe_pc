use std::collections::HashMap;

use crate::workflow::tts::adapters::{
    CloudflareAdapter, CustomHttpAdapter, EdgeAdapter, FishAudioAdapter, MimoAdapter,
    MiniMaxAdapter, SiliconFlowAdapter,
};
use crate::workflow::tts::settings::TtsMatrixSettings;
use crate::workflow::tts::types::{TtsEngine, TtsRequest, TtsResponse};

/// 统一 TTS 调度分发中心
/// 负责生命周期管理、厂商引擎路由分发、自动容灾降级 (Auto Fallback) 与统一麦克风粉红底噪注入
pub struct TtsDispatcher {
    pub settings: TtsMatrixSettings,
    engines: HashMap<String, Box<dyn TtsEngine>>,
    edge_fallback: EdgeAdapter,
}

impl TtsDispatcher {
    /// 构造新的调度中心实例，依据传入设置实例化所有提供商适配器
    pub fn new(settings: TtsMatrixSettings) -> Self {
        let mut engines: HashMap<String, Box<dyn TtsEngine>> = HashMap::new();

        let edge = EdgeAdapter::new(settings.edge.clone());
        let siliconflow = SiliconFlowAdapter::new(settings.siliconflow.clone());
        let fish_audio = FishAudioAdapter::new(settings.fish_audio.clone());
        let mimo = MimoAdapter::new(settings.mimo.clone());
        let minimax = MiniMaxAdapter::new(settings.minimax.clone());
        let cloudflare = CloudflareAdapter::new(settings.cloudflare.clone());
        let custom_http = CustomHttpAdapter::new(settings.custom_http.clone());

        engines.insert("edge".to_string(), Box::new(edge.clone()));
        engines.insert("siliconflow".to_string(), Box::new(siliconflow));
        engines.insert("fish_audio".to_string(), Box::new(fish_audio));
        engines.insert("mimo".to_string(), Box::new(mimo));
        engines.insert("minimax".to_string(), Box::new(minimax));
        engines.insert("cloudflare".to_string(), Box::new(cloudflare));
        engines.insert("custom_http".to_string(), Box::new(custom_http));

        Self {
            settings,
            engines,
            edge_fallback: edge,
        }
    }

    /// 从磁盘读取持久化配置并初始化调度器
    pub fn from_saved_settings() -> Self {
        Self::new(TtsMatrixSettings::load())
    }

    /// 支持的所有提供商标识列表
    pub fn supported_providers() -> &'static [&'static str] {
        &[
            "edge",
            "siliconflow",
            "fish_audio",
            "mimo",
            "minimax",
            "cloudflare",
            "custom_http",
        ]
    }

    /// 获取特定提供商引擎引用
    pub fn get_engine(&self, provider_id: &str) -> Option<&dyn TtsEngine> {
        self.engines.get(provider_id).map(|b| b.as_ref())
    }

    /// 使用当前设置中激活的默认提供商进行合成
    pub fn synthesize(&self, req: &TtsRequest) -> Result<TtsResponse, String> {
        self.synthesize_with_provider(&self.settings.active_provider, req)
    }

    /// 指定特定提供商进行合成，支持自动容灾回退与统一麦克风底噪混音
    pub fn synthesize_with_provider(
        &self,
        provider_id: &str,
        req: &TtsRequest,
    ) -> Result<TtsResponse, String> {
        let norm_provider = provider_id.trim().to_lowercase();
        let target_provider = if norm_provider.is_empty() {
            "edge"
        } else {
            norm_provider.as_str()
        };

        let primary_result = if let Some(engine) = self.engines.get(target_provider) {
            engine.synthesize(req)
        } else if target_provider == "clone" {
            // 兼容历史 "clone" 模式标记
            if let Some(mimo) = self.engines.get("mimo") {
                mimo.synthesize(req)
            } else {
                Err(format!("未找到兼容的克隆提供商: {target_provider}"))
            }
        } else if target_provider == "http" {
            // 兼容历史 "http" 模式标记
            if let Some(http) = self.engines.get("custom_http") {
                http.synthesize(req)
            } else {
                Err(format!("未找到兼容的 HTTP 提供商: {target_provider}"))
            }
        } else {
            Err(format!("未知的 TTS 提供商标识符: '{target_provider}'"))
        };

        // 处理结果及自动容灾降级
        let mut response = match primary_result {
            Ok(resp) => resp,
            Err(primary_err) => {
                if self.settings.enable_auto_fallback && target_provider != "edge" {
                    eprintln!(
                        "⚠️ [TTS 自动容灾降级] 提供商 '{target_provider}' 发生故障 ({primary_err})，正在自动无感切换至微软 Edge 官方高清音色..."
                    );

                    let fallback_voice = if !self.settings.edge.voice.trim().is_empty() {
                        self.settings.edge.voice.clone()
                    } else {
                        "en-US-JennyNeural".to_string()
                    };

                    let fallback_req = TtsRequest {
                        text: req.text.clone(),
                        voice: Some(fallback_voice),
                        speed: req.speed,
                        pitch: Some(self.settings.edge.pitch),
                        reference_audio: None,
                        extra_params: HashMap::new(),
                    };

                    match self.edge_fallback.synthesize(&fallback_req) {
                        Ok(mut edge_resp) => {
                            edge_resp.engine_identifier =
                                format!("edge (fallback from {target_provider})");
                            edge_resp
                        }
                        Err(fallback_err) => {
                            return Err(format!(
                                "首选引擎 [{target_provider}] 失败: {primary_err}; 容灾降级 Edge 亦失败: {fallback_err}"
                            ));
                        }
                    }
                } else {
                    return Err(primary_err);
                }
            }
        };

        // 统一麦克风粉红底噪混音 (注入物理模拟麦克风底噪，增强防作弊通过率)
        if self.settings.enable_pink_noise {
            response.audio_bytes = Self::inject_mic_noise(
                &response.audio_bytes,
                self.settings.noise_level_db,
            );
            // 混音后重新核算时长
            if let Ok((new_dur, _)) =
                crate::workflow::audio_proc::check_audio_validity(&response.audio_bytes)
                && new_dur > 0.0
            {
                response.duration_secs = new_dur;
            }
        }

        Ok(response)
    }

    /// 麦克风粉红底噪注入辅助函数
    pub fn inject_mic_noise(audio_bytes: &[u8], noise_strength_db: f32) -> Vec<u8> {
        match crate::workflow::audio_proc::inject_mic_noise(audio_bytes, noise_strength_db) {
            Ok(noised) => noised,
            Err(_) => audio_bytes.to_vec(),
        }
    }
}

impl Default for TtsDispatcher {
    fn default() -> Self {
        Self::from_saved_settings()
    }
}
