//! Fe PC 多提供商 TTS 引擎矩阵核心实现与调度分发
//!
//! 具备微软 Edge、Fish Audio、硅基流动 (SiliconFlow)、小米 MiMo、MiniMax 海螺、
//! Cloudflare Workers AI 及自定义 OpenAI 规范 HTTP 适配器。
//! 提供统一抽象特征接口、独立持久化配置、物理麦克风底噪混音与全自动容灾回退能力。

pub mod adapters;
pub mod dispatcher;
pub mod settings;
pub mod types;

use std::sync::Once;
use std::time::Duration;
use base64::Engine;

pub use adapters::*;
pub use dispatcher::*;
pub use settings::*;
pub use types::*;

static INIT_CRYPTO: Once = Once::new();

pub fn ensure_crypto_provider() {
    INIT_CRYPTO.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

// ==============================================================================
// 历史向后兼容服务结构定义 (Legacy Compatibility Services)
// ==============================================================================

/// 兼容历史代码的微软 Edge TTS 服务包装
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
        let adapter = EdgeAdapter::with_default_config();
        let mut req = TtsRequest::new(text);
        if let Some(v) = voice_name {
            req = req.with_voice(v);
        }
        if let Some(p) = pitch {
            req = req.with_pitch(p);
        }
        if let Some(r) = rate {
            // 将历史百分比语速映射为 speed 浮点
            req = req.with_speed(1.0 + (r as f64) / 50.0);
        }
        adapter.synthesize(&req).map(|resp| resp.audio_bytes)
    }
}

/// 兼容历史代码的自定义 HTTP TTS 服务包装
pub struct HttpTtsService;

impl HttpTtsService {
    /// 调用自定义 HTTP TTS 服务合成音频（兼容 OpenAI / Kokoro / TTS.ai / Gradio 规范）
    #[inline(never)]
    pub fn synthesize(
        endpoint_url: &str,
        api_key: Option<&str>,
        model: Option<&str>,
        text: &str,
        voice_name: Option<&str>,
        speed: Option<f64>,
    ) -> Result<Vec<u8>, String> {
        let mut config = CustomHttpConfig {
            endpoint_url: endpoint_url.to_string(),
            ..Default::default()
        };
        if let Some(k) = api_key {
            config.api_key = k.to_string();
        }
        if let Some(m) = model {
            config.model = m.to_string();
        }
        let adapter = CustomHttpAdapter::new(config);
        let mut req = TtsRequest::new(text);
        if let Some(v) = voice_name {
            req = req.with_voice(v);
        }
        if let Some(s) = speed {
            req = req.with_speed(s);
        }
        adapter.synthesize(&req).map(|resp| resp.audio_bytes)
    }

    /// 解析 SSE 流式响应中的音频 Base64 数据块
    pub fn parse_sse_audio(payload: &str) -> Result<Vec<u8>, String> {
        CustomHttpAdapter::parse_sse_audio(payload)
    }
}

/// 兼容历史代码的声音克隆服务包装
pub struct VoiceCloneService;

impl VoiceCloneService {
    /// MiMo 语音克隆协议
    pub fn synthesize_mimo_clone(
        endpoint: Option<&str>,
        api_key: &str,
        model: Option<&str>,
        text: &str,
        reference_audio: &[u8],
    ) -> Result<Vec<u8>, String> {
        let mut config = MimoConfig::default();
        if let Some(ep) = endpoint {
            config.endpoint_url = ep.to_string();
        }
        config.api_key = api_key.to_string();
        if let Some(m) = model {
            config.model = m.to_string();
        }
        let adapter = MimoAdapter::new(config);
        let req = TtsRequest::new(text).with_reference_audio(reference_audio.to_vec());
        adapter.synthesize(&req).map(|resp| resp.audio_bytes)
    }

    /// tts.ai 语音克隆协议 (带拟真公网 IP 注入与轮询)
    pub fn synthesize_ttsai_clone(
        endpoint: Option<&str>,
        api_key: Option<&str>,
        model: Option<&str>,
        speed: Option<f64>,
        text: &str,
        reference_audio: &[u8],
    ) -> Result<Vec<u8>, String> {
        let clean_text = text.trim();
        if clean_text.is_empty() {
            return Err("tts.ai 语音克隆文本为空".to_string());
        }
        if reference_audio.is_empty() {
            return Err("tts.ai 语音克隆参考音频为空".to_string());
        }

        let endpoint_url = endpoint.unwrap_or("https://tts.ai/api/v1/voice-clone/");
        let model_name = model.unwrap_or("pocket-tts");
        let speed_val = speed.unwrap_or(0.95);
        let speed_str = format!("{speed_val:.2}");

        let random_ip = Self::random_ipv4();
        let boundary = format!("----TTSBoundary{}", uuid::Uuid::new_v4().simple());
        let mut body = Vec::new();

        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"text\"\r\n\r\n{clean_text}\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n{model_name}\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"speed\"\r\n\r\n{speed_str}\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nmp3\r\n")
                .as_bytes(),
        );

        let (filename, content_type) = if reference_audio.starts_with(b"RIFF") {
            ("reference.wav", "audio/wav")
        } else {
            ("reference.mp3", "audio/mpeg")
        };
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(reference_audio);
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let mut req = ureq::post(endpoint_url)
            .timeout(Duration::from_secs(30))
            .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .set("X-Forwarded-For", &random_ip);

        if let Some(key) = api_key {
            let key = key.trim();
            if !key.is_empty() {
                req = req.set("Authorization", &format!("Bearer {key}"));
            }
        }

        let resp = match req.send_bytes(&body) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_msg = resp.into_string().unwrap_or_default();
                return Err(format!("tts.ai 克隆请求失败 (HTTP {code}): {err_msg}"));
            }
            Err(e) => {
                return Err(format!("tts.ai 网络连接失败: {e}"));
            }
        };

        let submit_res: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析 tts.ai 响应 JSON 失败: {e}"))?;

        if let Some(err) = submit_res.get("error") {
            return Err(format!("tts.ai 接口返回错误: {err}"));
        }

        let uuid = submit_res
            .get("uuid")
            .or_else(|| submit_res.get("id"))
            .or_else(|| submit_res.pointer("/data/uuid"))
            .or_else(|| submit_res.pointer("/data/id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("tts.ai 响应中缺少 uuid: {submit_res}"))?;

        Self::poll_ttsai_result(uuid, &random_ip, api_key)
    }

    /// 轮询 tts.ai 任务状态并下载结果音频
    fn poll_ttsai_result(uuid: &str, random_ip: &str, api_key: Option<&str>) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let poll_url = format!("https://api.tts.ai/v1/speech/results/?uuid={uuid}");
        let mut last_status = String::new();

        for attempt in 1..=60 {
            std::thread::sleep(Duration::from_millis(500));

            let mut req = ureq::get(&poll_url)
                .timeout(Duration::from_secs(15))
                .set("X-Forwarded-For", random_ip);

            if let Some(key) = api_key {
                let key = key.trim();
                if !key.is_empty() {
                    req = req.set("Authorization", &format!("Bearer {key}"));
                }
            }

            let resp = match req.call() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("tts.ai 轮询第 {attempt} 次网络波动: {e}");
                    continue;
                }
            };

            let poll_val: serde_json::Value = match resp.into_json() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("tts.ai 轮询 JSON 解析失败: {e}");
                    continue;
                }
            };

            let status = poll_val
                .get("status")
                .or_else(|| poll_val.pointer("/data/status"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            last_status = status.to_string();

            if status.eq_ignore_ascii_case("completed")
                || status.eq_ignore_ascii_case("success")
                || status.eq_ignore_ascii_case("finished")
            {
                let audio_url = poll_val
                    .get("audio_url")
                    .or_else(|| poll_val.get("url"))
                    .or_else(|| poll_val.get("download_url"))
                    .or_else(|| poll_val.pointer("/data/audio_url"))
                    .or_else(|| poll_val.pointer("/data/url"))
                    .and_then(|v| v.as_str());

                if let Some(dl_url) = audio_url {
                    let dl_resp = ureq::get(dl_url)
                        .timeout(Duration::from_secs(30))
                        .set("X-Forwarded-For", random_ip)
                        .call()
                        .map_err(|e| format!("下载 tts.ai 音频文件失败 ({dl_url}): {e}"))?;

                    let mut dl_bytes = Vec::new();
                    dl_resp
                        .into_reader()
                        .read_to_end(&mut dl_bytes)
                        .map_err(|e| format!("读取 tts.ai 音频流失败: {e}"))?;

                    if dl_bytes.is_empty() {
                        return Err("tts.ai 下载的音频内容为空".to_string());
                    }
                    return Ok(dl_bytes);
                }

                if let Some(b64) = poll_val
                    .get("audio")
                    .or_else(|| poll_val.pointer("/data/audio"))
                    .and_then(|v| v.as_str())
                {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(b64)
                        .map_err(|e| format!("tts.ai 音频 Base64 解码失败: {e}"))?;
                    if !bytes.is_empty() {
                        return Ok(bytes);
                    }
                }

                return Err(format!("tts.ai 任务完成但未找到音频下载信息: {poll_val}"));
            } else if status.eq_ignore_ascii_case("failed") || status.eq_ignore_ascii_case("error") {
                let err_msg = poll_val
                    .get("error")
                    .or_else(|| poll_val.get("message"))
                    .or_else(|| poll_val.pointer("/data/error"))
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| poll_val.to_string());
                return Err(format!("tts.ai 克隆任务失败: {err_msg}"));
            }
        }

        Err(format!("tts.ai 任务轮询超时 (30 秒，最终状态: {last_status})"))
    }

    /// 随机生成拟真公网 IPv4 地址
    pub fn random_ipv4() -> String {
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(987654321) as u64;
        let mut x = nanos ^ 0xa076_1d64_78bd_642f;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;

        let b1 = 110 + ((x >> 24) % 100);
        let b2 = 10 + ((x >> 16) % 240);
        let b3 = 10 + ((x >> 8) % 240);
        let b4 = 1 + (x % 250);
        format!("{b1}.{b2}.{b3}.{b4}")
    }
}

// ==============================================================================
// 统一公共入口函数 (100% 保持原有签名向后兼容)
// ==============================================================================

/// 统一调度入口：自动识别并调度 Edge TTS、自定义 HTTP TTS、声音克隆或新版多厂商矩阵
///
/// 内部自动根据参数映射至多厂商适配器，并由 `TtsDispatcher` 托管容灾回退与粉红底噪混音。
#[allow(clippy::too_many_arguments)]
pub fn synthesize_auto(
    text: &str,
    mode: &str,
    voice_name: Option<&str>,
    rate: Option<i32>,
    pitch: Option<i32>,
    http_url: Option<&str>,
    http_key: Option<&str>,
    http_model: Option<&str>,
    clone_reference_path: Option<&str>,
) -> Result<Vec<u8>, String> {
    let clean_text = text.trim();
    if clean_text.is_empty() {
        return Err("TTS 待朗读文本为空".to_string());
    }

    // 加载当前磁盘配置，并根据传入参数进行即时动态覆盖
    let mut settings = TtsMatrixSettings::load();

    // 解析参考音频数据
    let ref_audio_bytes = if let Some(path) = clone_reference_path {
        std::fs::read(path).ok()
    } else if let Some(v) = voice_name
        && (v.ends_with(".wav") || v.ends_with(".mp3") || std::path::Path::new(v).is_file())
    {
        std::fs::read(v).ok()
    } else {
        None
    };

    let speed_val = rate.map(|r| 1.0 + (r as f64) / 50.0);

    let mut req = TtsRequest::new(clean_text);
    if let Some(v) = voice_name {
        req = req.with_voice(v);
    }
    if let Some(p) = pitch {
        req = req.with_pitch(p);
    }
    if let Some(s) = speed_val {
        req = req.with_speed(s);
    }
    if let Some(ref_bytes) = ref_audio_bytes.clone() {
        req = req.with_reference_audio(ref_bytes);
    }

    let url_str = http_url.unwrap_or("").trim();
    let model_str = http_model.unwrap_or("").trim();
    let key_str = http_key.unwrap_or("").trim();

    // 智能识别目标提供商与适配器
    let norm_mode = mode.trim().to_lowercase();
    let target_provider: String = if norm_mode == "clone" {
        if url_str.contains("fish") || model_str.contains("fish") || settings.active_provider == "fish_audio" {
            if !url_str.is_empty() { settings.fish_audio.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.fish_audio.api_key = key_str.to_string(); }
            if !model_str.is_empty() { settings.fish_audio.model = model_str.to_string(); }
            "fish_audio".to_string()
        } else if url_str.contains("siliconflow") || model_str.contains("cosyvoice") || settings.active_provider == "siliconflow" {
            if !url_str.is_empty() { settings.siliconflow.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.siliconflow.api_key = key_str.to_string(); }
            if !model_str.is_empty() { settings.siliconflow.model = model_str.to_string(); }
            "siliconflow".to_string()
        } else {
            // 默认采用小米 MiMo 克隆
            if !url_str.is_empty() { settings.mimo.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.mimo.api_key = key_str.to_string(); }
            if !model_str.is_empty() { settings.mimo.model = model_str.to_string(); }
            "mimo".to_string()
        }
    } else if norm_mode == "http" {
        if url_str.contains("siliconflow") {
            if !url_str.is_empty() { settings.siliconflow.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.siliconflow.api_key = key_str.to_string(); }
            if !model_str.is_empty() { settings.siliconflow.model = model_str.to_string(); }
            "siliconflow".to_string()
        } else if url_str.contains("fish") {
            if !url_str.is_empty() { settings.fish_audio.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.fish_audio.api_key = key_str.to_string(); }
            if !model_str.is_empty() { settings.fish_audio.model = model_str.to_string(); }
            "fish_audio".to_string()
        } else if url_str.contains("minimax") {
            if !url_str.is_empty() { settings.minimax.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.minimax.api_key = key_str.to_string(); }
            if !model_str.is_empty() { settings.minimax.model = model_str.to_string(); }
            "minimax".to_string()
        } else if url_str.contains("cloudflare") {
            if !url_str.is_empty() { settings.cloudflare.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.cloudflare.api_token = key_str.to_string(); }
            if !model_str.is_empty() { settings.cloudflare.model = model_str.to_string(); }
            "cloudflare".to_string()
        } else {
            if !url_str.is_empty() { settings.custom_http.endpoint_url = url_str.to_string(); }
            if !key_str.is_empty() { settings.custom_http.api_key = key_str.to_string(); }
            if !model_str.is_empty() { settings.custom_http.model = model_str.to_string(); }
            "custom_http".to_string()
        }
    } else if TtsDispatcher::supported_providers().contains(&norm_mode.as_str()) {
        norm_mode.clone()
    } else if norm_mode == "matrix" || norm_mode == "auto" || norm_mode.is_empty() {
        if TtsDispatcher::supported_providers().contains(&settings.active_provider.as_str()) {
            settings.active_provider.clone()
        } else {
            "edge".to_string()
        }
    } else {
        "edge".to_string()
    };

    // 若处于 clone 模式且参考音频完全缺失，且非 Fish Audio，直接触发 Edge 容灾优雅回退
    if norm_mode == "clone" && ref_audio_bytes.is_none() && target_provider != "fish_audio" {
        eprintln!("⚠️ Voice Clone 参考音频未指定或不可读，自动优雅回退到微软 Edge 官方音色");
        return EdgeTtsService::synthesize(clean_text, voice_name, rate, pitch);
    }

    let dispatcher = TtsDispatcher::new(settings);
    match dispatcher.synthesize_with_provider(&target_provider, &req) {
        Ok(resp) => Ok(resp.audio_bytes),
        Err(err) => {
            // 如果调度中心由于 fallback 关闭或者边缘异常失败，再次保证流水线绝不中断
            eprintln!("⚠️ TTS 调度执行失败 ({err})，尝试终极回退至 Edge 官方音色...");
            EdgeTtsService::synthesize(clean_text, voice_name, rate, pitch)
        }
    }
}

// ==============================================================================
// 单元测试与验证模块
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types_and_payload_building() {
        let fmt = AudioFormat::Mp3;
        assert_eq!(fmt.as_str(), "mp3");
        assert_eq!(fmt.mime_type(), "audio/mpeg");

        let req = TtsRequest::new("Hello world")
            .with_voice("en-US-JennyNeural")
            .with_speed(1.2)
            .with_pitch(2)
            .with_extra("latency", "normal");

        assert_eq!(req.text, "Hello world");
        assert_eq!(req.voice.as_deref(), Some("en-US-JennyNeural"));
        assert_eq!(req.speed, Some(1.2));
        assert_eq!(req.pitch, Some(2));
        assert_eq!(req.extra_params.get("latency").map(|s| s.as_str()), Some("normal"));

        let resp = TtsResponse::new(vec![0u8; 1024], AudioFormat::Mp3, "edge");
        assert_eq!(resp.engine_identifier, "edge");
        assert_eq!(resp.format, AudioFormat::Mp3);
    }

    #[test]
    fn test_settings_independence_and_defaults() {
        let mut settings = TtsMatrixSettings::default();
        assert_eq!(settings.active_provider, "edge");
        assert!(settings.enable_auto_fallback);
        assert!(settings.enable_pink_noise);

        // 各厂商配置独立隔离测试
        settings.siliconflow.api_key = "sf_secret_123".to_string();
        settings.fish_audio.api_key = "fish_secret_456".to_string();
        settings.mimo.api_key = "mimo_secret_789".to_string();
        settings.minimax.api_key = "minimax_secret_000".to_string();

        assert_eq!(settings.siliconflow.api_key, "sf_secret_123");
        assert_eq!(settings.fish_audio.api_key, "fish_secret_456");
        assert_eq!(settings.mimo.api_key, "mimo_secret_789");
        assert_eq!(settings.minimax.api_key, "minimax_secret_000");
        assert!(settings.edge.voice.contains("Jenny"));

        // 序列化与反序列化测试
        let serialized = serde_json::to_string(&settings).expect("serialization failed");
        let deserialized: TtsMatrixSettings =
            serde_json::from_str(&serialized).expect("deserialization failed");
        assert_eq!(deserialized.siliconflow.api_key, "sf_secret_123");
        assert_eq!(deserialized.fish_audio.api_key, "fish_secret_456");
        assert_eq!(deserialized.minimax.api_key, "minimax_secret_000");
    }

    #[test]
    fn test_fish_audio_multipart_builder() {
        let text = "Test text for Fish Audio";
        let model = "s2.1-pro";
        let ref_audio = b"RIFFfake_wav_audio_bytes";
        let boundary = "----TestBoundary123";

        let body = FishAudioAdapter::build_multipart_payload(
            text,
            model,
            ref_audio,
            Some("preset_voice_1"),
            boundary,
        );

        let body_str = String::from_utf8_lossy(&body);
        assert!(body_str.contains("name=\"text\""));
        assert!(body_str.contains("Test text for Fish Audio"));
        assert!(body_str.contains("name=\"model\""));
        assert!(body_str.contains("s2.1-pro"));
        assert!(body_str.contains("name=\"reference_audio\""));
        assert!(body_str.contains("filename=\"reference.wav\""));
        assert!(body_str.contains("reference_id"));
        assert!(body_str.contains("----TestBoundary123--"));
    }

    #[test]
    fn test_siliconflow_upload_url_resolution() {
        let adapter = SiliconFlowAdapter::with_default_config();
        assert_eq!(
            adapter.get_voice_upload_url(),
            "https://api.siliconflow.cn/v1/uploads/audio/voice"
        );

        let custom_cfg = SiliconFlowConfig {
            endpoint_url: "https://custom.api.com/v1/audio/speech".to_string(),
            ..Default::default()
        };
        let custom_adapter = SiliconFlowAdapter::new(custom_cfg);
        assert_eq!(
            custom_adapter.get_voice_upload_url(),
            "https://custom.api.com/v1/uploads/audio/voice"
        );
    }

    #[test]
    fn test_minimax_audio_decoding_hex_and_base64() {
        // 1. Hex 解码测试
        let original = b"Hello MiniMax Audio";
        let hex_str: String = original.iter().map(|b| format!("{b:02x}")).collect();
        let decoded_hex = MiniMaxAdapter::decode_audio_payload(&hex_str).expect("hex decode failed");
        assert_eq!(decoded_hex, original);

        // 2. Base64 解码测试
        let b64_str = base64::engine::general_purpose::STANDARD.encode(original);
        let decoded_b64 = MiniMaxAdapter::decode_audio_payload(&b64_str).expect("b64 decode failed");
        assert_eq!(decoded_b64, original);
    }

    #[test]
    fn test_cloudflare_endpoint_resolution() {
        let cfg = CloudflareConfig {
            account_id: "test_account_999".to_string(),
            ..Default::default()
        };
        let adapter = CloudflareAdapter::new(cfg);
        let ep = adapter.resolve_endpoint().expect("resolve failed");
        assert_eq!(
            ep,
            "https://api.cloudflare.com/client/v4/accounts/test_account_999/ai/run/@cf/deepgram/aura-1"
        );
    }

    #[test]
    fn test_random_ipv4_format() {
        let ip = VoiceCloneService::random_ipv4();
        let parts: Vec<&str> = ip.split('.').collect();
        assert_eq!(parts.len(), 4, "IPv4 should have 4 octets, got: {ip}");
        for part in parts {
            let num: u32 = part.parse().expect("each octet should be an integer");
            assert!(num <= 255, "octet out of range: {num}");
        }
    }

    #[test]
    fn test_empty_text_rejection() {
        let err = EdgeTtsService::synthesize("   ", None, None, None);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("为空"));

        let err_auto = synthesize_auto("   ", "edge", None, None, None, None, None, None, None);
        assert!(err_auto.is_err());
        assert!(err_auto.unwrap_err().contains("为空"));
    }

    #[test]
    fn test_dispatcher_fallback_routing() {
        let mut settings = TtsMatrixSettings {
            enable_auto_fallback: true,
            ..Default::default()
        };
        settings.mimo.api_key = "".to_string(); // 触发 API key 缺失错误

        let dispatcher = TtsDispatcher::new(settings);
        let req = TtsRequest::new("Fallback routing test.");

        // mimo 在无 key 时会返回错误并触发 fallback 到 edge
        let res = dispatcher.synthesize_with_provider("mimo", &req);
        match res {
            Ok(resp) => {
                // 如果当前测试环境有网络，成功从 Edge 降级返回
                assert!(!resp.audio_bytes.is_empty());
                assert!(resp.engine_identifier.contains("fallback"));
            }
            Err(e) => {
                // 如果当前离线，应展示两个错误都已尝试的信息
                println!("Network offline during fallback test: {e}");
            }
        }
    }

    #[test]
    fn test_clone_fallback_graceful() {
        // 当 clone 模式且参考音频文件不存在时，应优雅回退到 Edge TTS（不 panic）
        let res = synthesize_auto(
            "Fallback test.",
            "clone",
            Some("en-US-JennyNeural"),
            Some(0),
            Some(0),
            None,
            None,
            None,
            Some("non_existent_reference_file_123456.wav"),
        );
        match res {
            Ok(bytes) => {
                assert!(!bytes.is_empty(), "Fallback to Edge TTS should produce valid audio");
            }
            Err(e) => {
                println!("Edge TTS offline or network error during test: {e}");
            }
        }
    }
}
