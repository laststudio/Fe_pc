// =========================================================================================
// ⚠️ 设置视图排版规范：
// 1. 设置项中的单选按钮组、材质选择与主题切换统一使用 `ui.horizontal_wrapped` + `choice_btn`。
// 2. 避免使用不可折行的 `ui.row`，输入框使用 `horizontal_wrapped` 并指定合理宽度。
// =========================================================================================

use std::fs;

use eframe::egui::{self, Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::{BackdropKind, FluentTheme, InfoBarKind};

use crate::state::AppState;
use crate::views::{action_btn, choice_btn, compact_slider, render_header, responsive_flow};

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(ui, "设置与偏好", "自定义界面外观、系统材质与缓存管理");

    ui.card("界面外观与材质", "个性化 Windows 11 Fluent 视觉效果", |ui| {
        let accent = state.theme_config.accent;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            ui.label(RichText::new("主题模式:").size(13.0).strong());
            let is_dark = state.theme_config.theme.is_dark();
            if choice_btn(ui, "深色模式", is_dark, accent).clicked() {
                state.theme_config.theme = FluentTheme::Dark;
            }
            if choice_btn(ui, "浅色模式", !is_dark, accent).clicked() {
                state.theme_config.theme = FluentTheme::Light;
            }
        });

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            ui.label(RichText::new("窗口背景材质:").size(13.0).strong());
            let backdrops = [
                (BackdropKind::Mica, "Mica (云母磨砂·Win11)"),
                (BackdropKind::Acrylic, "Acrylic (亚克力透光)"),
                (BackdropKind::Tabbed, "Tabbed (云母双层·标签页材质)"),
                (BackdropKind::Auto, "自动适配"),
                (BackdropKind::None, "纯色背景 (低资源)"),
            ];

            for (bk, label) in backdrops {
                let is_sel = state.backdrop_kind == bk;
                if choice_btn(ui, label, is_sel, accent).clicked() {
                    state.backdrop_kind = bk;
                    state.theme_config = state.theme_config.with_backdrop(bk);
                    fluent_egui::winui::apply_theme(ui.ctx(), state.theme_config);
                    ui.ctx().request_repaint();
                    state.notification = Some((format!("已切换背景材质为: {}", label), InfoBarKind::Success));
                }
            }
        });

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            ui.label(RichText::new("主题强调色:").size(13.0).strong());
            let colors = [
                ("默认蓝", Color32::from_rgb(0, 120, 215)),
                ("紫罗兰", Color32::from_rgb(138, 43, 226)),
                ("薄荷绿", Color32::from_rgb(46, 204, 113)),
                ("落霞橙", Color32::from_rgb(230, 126, 34)),
            ];

            for (name, col) in colors {
                let is_sel = state.theme_config.accent == Some(col);
                let mut btn = egui::Button::new(
                    RichText::new(if is_sel { format!("✔ {}", name) } else { name.to_string() })
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .fill(col)
                .min_size(egui::vec2(0.0, 30.0))
                .corner_radius(6.0);
                if is_sel {
                    btn = btn.stroke(egui::Stroke::new(2.0, Color32::WHITE));
                }
                if ui.add(btn).clicked() {
                    state.theme_config = state.theme_config.with_accent(col);
                }
            }
        });

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            ui.label(RichText::new("动效体验:").size(13.0).strong());
            let mut anim = state.theme_config.animations;
            if ui.add(fluent_egui::winui::ToggleSwitch::new(&mut anim)).changed() {
                state.theme_config = state.theme_config.with_animations(anim);
                if anim {
                    state.nav_revision = state.nav_revision.wrapping_add(1);
                }
                fluent_egui::winui::apply_theme(ui.ctx(), state.theme_config);
                ui.ctx().request_repaint();
                state.notification = Some((
                    if anim {
                        "已开启 Fluent 页面平滑过渡与实时动效".to_string()
                    } else {
                        "已关闭界面动效 (节能模式)".to_string()
                    },
                    InfoBarKind::Success,
                ));
            }
            ui.label(RichText::new("启用 Fluent 2 页面平滑过渡、呼吸灯与音波可视化动效").size(12.5));
        });
    });

    ui.add_space(8.0);

    ui.card("数据与缓存管理", "管理本地下载的试题资源和临时文件", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            if action_btn(ui, "🗑 清理试卷临时下载缓存").clicked() {
                let cache_dir = std::env::temp_dir().join("fepc_cache");
                if cache_dir.exists() {
                    let _ = fs::remove_dir_all(&cache_dir);
                }
                state.notification = Some(("已成功清空本地试卷下载缓存".to_string(), InfoBarKind::Success));
            }

            if action_btn(ui, "📂 打开缓存文件夹").clicked() {
                let cache_dir = std::env::temp_dir().join("fepc_cache");
                let _ = fs::create_dir_all(&cache_dir);
                let _ = open::that(&cache_dir);
            }
        });
    });

    ui.add_space(8.0);

    ui.card("TTS 语音合成与声音克隆引擎矩阵", "管理英文朗读、零样本声音克隆、多云服务与麦克风物理底噪", |ui| {
        let accent = state.theme_config.accent;
        let mut settings_changed = false;

        responsive_flow(ui, |ui| {
            ui.label(RichText::new("引擎预设:").size(13.0).strong());
            let current = state.tts_settings.active_provider.clone();

            let providers = [
                ("edge", "🌐 微软 Edge (免密·免费)"),
                ("fish_audio", "🐟 Fish Audio (免密模型·极速克隆)"),
                ("siliconflow", "⚡ 硅基流动 (低价·CosyVoice2)"),
                ("mimo", "📲 小米 MiMo (v2.5 官方克隆)"),
                ("minimax", "🎤 MiniMax 海螺 (极致拟真)"),
                ("cloudflare", "☁ Cloudflare (每日免费)"),
                ("custom_http", "⚙ 自定义 HTTP (兼容 OpenAI)"),
            ];

            for (pid, label) in providers {
                let is_sel = current == pid;
                if choice_btn(ui, label, is_sel, accent).clicked() {
                    state.tts_settings.active_provider = pid.to_string();
                    state.auto_config.tts_mode = match pid {
                        "edge" => "edge".to_string(),
                        "fish_audio" | "mimo" => "clone".to_string(),
                        _ => "http".to_string(),
                    };
                    settings_changed = true;
                    state.notification = Some((format!("已切换为 {label}"), InfoBarKind::Success));
                }
            }
        });

        ui.add_space(6.0);
        ui.separator();

        // 根据当前提供商动态展示配置抽屉
        match state.tts_settings.active_provider.as_str() {
            "edge" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("官方推荐高保真音色:").strong());
                    let edge_voices = [
                        ("en-US-AvaNeural", "Ava (自然柔美)"),
                        ("en-US-AndrewNeural", "Andrew (温暖自然)"),
                        ("en-US-AriaNeural", "Aria (抑扬丰富)"),
                        ("en-US-JennyNeural", "Jenny (标准女音)"),
                        ("en-US-GuyNeural", "Guy (标准男音)"),
                        ("zh-CN-YunxiNeural", "云希 (中英双语)"),
                    ];
                    for (vid, vlabel) in edge_voices {
                        let is_active = state.tts_settings.edge.voice == vid;
                        if choice_btn(ui, vlabel, is_active, accent).clicked() {
                            state.tts_settings.edge.voice = vid.to_string();
                            state.auto_config.voice_name = vid.to_string();
                            settings_changed = true;
                        }
                    }
                });

                responsive_flow(ui, |ui| {
                    ui.label("语速偏置:");
                    if compact_slider(ui, egui::Slider::new(&mut state.tts_settings.edge.rate, -30..=30).suffix("%")).changed() {
                        settings_changed = true;
                    }
                    ui.label("音调偏置:");
                    if compact_slider(ui, egui::Slider::new(&mut state.tts_settings.edge.pitch, -20..=20).suffix("半音")).changed() {
                        settings_changed = true;
                    }
                });
                ui.label(RichText::new("💡 微软官方神经语音，完全免费无额度限制，首包响应极快，官方默认推荐。").size(11.0).color(Color32::from_gray(160)));
            }
            "fish_audio" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("模型版本:").strong());
                    let is_free = state.tts_settings.fish_audio.model == "s2.1-pro-free";
                    if choice_btn(ui, "s2.1-pro-free (免密免费体验模型)", is_free, accent).clicked() {
                        state.tts_settings.fish_audio.model = "s2.1-pro-free".to_string();
                        settings_changed = true;
                    }
                    if choice_btn(ui, "s2.1-pro (高清商业版)", !is_free, accent).clicked() {
                        state.tts_settings.fish_audio.model = "s2.1-pro".to_string();
                        settings_changed = true;
                    }
                });

                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("API Key:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.fish_audio.api_key).hint_text("使用免费模型可留空").desired_width(260.0)).changed() {
                        settings_changed = true;
                    }
                    ui.label("端点 URL:");
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.fish_audio.endpoint_url).desired_width(240.0)).changed() {
                        settings_changed = true;
                    }
                });
                responsive_flow(ui, |ui| {
                    if action_btn(ui, "⚡ 恢复 Fish Audio 推荐配置").clicked() {
                        state.tts_settings.fish_audio = crate::workflow::tts::FishAudioConfig::default();
                        settings_changed = true;
                    }
                });
                ui.label(RichText::new("💡 Fish Audio 原生支持单次请求直接上传 5 秒参考录音实现 0-shot 极速声音克隆，情绪灵动自然。").size(11.0).color(Color32::from_gray(160)));
            }
            "siliconflow" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("模型选择:").strong());
                    let is_cosy = state.tts_settings.siliconflow.model.contains("CosyVoice");
                    if choice_btn(ui, "CosyVoice2-0.5B (极低价·韵律极佳)", is_cosy, accent).clicked() {
                        state.tts_settings.siliconflow.model = "FunAudioLLM/CosyVoice2-0.5B".to_string();
                        settings_changed = true;
                    }
                    if choice_btn(ui, "Fish-Speech-1.5 (自然多语种)", !is_cosy, accent).clicked() {
                        state.tts_settings.siliconflow.model = "fishaudio/fish-speech-1.5".to_string();
                        settings_changed = true;
                    }
                });

                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("API Key:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.siliconflow.api_key).hint_text("sk-xxxxxxxx").desired_width(260.0)).changed() {
                        settings_changed = true;
                    }
                    ui.label("端点 URL:");
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.siliconflow.endpoint_url).desired_width(240.0)).changed() {
                        settings_changed = true;
                    }
                });
                responsive_flow(ui, |ui| {
                    if action_btn(ui, "⚡ 恢复硅基流动推荐配置").clicked() {
                        state.tts_settings.siliconflow = crate::workflow::tts::SiliconFlowConfig::default();
                        settings_changed = true;
                    }
                });
                ui.label(RichText::new("💡 硅基流动注册实名即送 14 元体验金（可合成超 200 万字），单价低至 0.06 元/万字，100% 兼容 OpenAI 协议。").size(11.0).color(Color32::from_gray(160)));
            }
            "mimo" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("API Key:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.mimo.api_key).hint_text("小米开放平台 API Key").desired_width(260.0)).changed() {
                        settings_changed = true;
                    }
                    ui.label("端点 URL:");
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.mimo.endpoint_url).desired_width(240.0)).changed() {
                        settings_changed = true;
                    }
                });
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("模型名称:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.mimo.model).desired_width(220.0)).changed() {
                        settings_changed = true;
                    }
                    if action_btn(ui, "⚡ 恢复 MiMo 推荐配置").clicked() {
                        state.tts_settings.mimo = crate::workflow::tts::MimoConfig::default();
                        settings_changed = true;
                    }
                });
                ui.label(RichText::new("💡 小米官方 MiMo 大模型语音克隆协议，支持 DataURL 参考音频直接注入合成，与后台完美协同。").size(11.0).color(Color32::from_gray(160)));
            }
            "minimax" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("模型版本:").strong());
                    let is_turbo = state.tts_settings.minimax.model.contains("turbo");
                    if choice_btn(ui, "speech-2.8-turbo (极速响应)", is_turbo, accent).clicked() {
                        state.tts_settings.minimax.model = "speech-2.8-turbo".to_string();
                        settings_changed = true;
                    }
                    if choice_btn(ui, "speech-2.8-hd (高清拟真天花板)", !is_turbo, accent).clicked() {
                        state.tts_settings.minimax.model = "speech-2.8-hd".to_string();
                        settings_changed = true;
                    }
                });
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("API Key:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.minimax.api_key).hint_text("MiniMax API Key").desired_width(240.0)).changed() {
                        settings_changed = true;
                    }
                    ui.label("Group ID (可选):");
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.minimax.group_id).hint_text("国内账户可选填").desired_width(160.0)).changed() {
                        settings_changed = true;
                    }
                });
                responsive_flow(ui, |ui| {
                    if action_btn(ui, "⚡ 恢复 MiniMax 推荐配置").clicked() {
                        state.tts_settings.minimax = crate::workflow::tts::MiniMaxConfig::default();
                        settings_changed = true;
                    }
                });
                ui.label(RichText::new("💡 MiniMax 海螺语音拥有顶尖的英语生活化发音、真实呼吸感与叹息连读，新用户赠送 15~30 元体验金。").size(11.0).color(Color32::from_gray(160)));
            }
            "cloudflare" => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("Account ID:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.cloudflare.account_id).hint_text("Cloudflare 账户 ID").desired_width(220.0)).changed() {
                        settings_changed = true;
                    }
                    ui.label("API Token:");
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.cloudflare.api_token).hint_text("Workers AI 专属 Token").desired_width(220.0)).changed() {
                        settings_changed = true;
                    }
                });
                responsive_flow(ui, |ui| {
                    ui.label("模型标识:");
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.cloudflare.model).desired_width(200.0)).changed() {
                        settings_changed = true;
                    }
                    if action_btn(ui, "⚡ 恢复 Cloudflare 推荐配置").clicked() {
                        state.tts_settings.cloudflare = crate::workflow::tts::CloudflareConfig::default();
                        settings_changed = true;
                    }
                });
                ui.label(RichText::new("💡 Cloudflare Workers AI 每日免费赠送 10,000 神经元（折合约 7,300 字/天），集成 Deepgram Aura 高清音质。").size(11.0).color(Color32::from_gray(160)));
            }
            _ => {
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("接口 URL:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.custom_http.endpoint_url).desired_width(280.0)).changed() {
                        settings_changed = true;
                    }
                });
                responsive_flow(ui, |ui| {
                    ui.label(RichText::new("API Key:").strong());
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.custom_http.api_key).desired_width(220.0)).changed() {
                        settings_changed = true;
                    }
                    ui.label("模型名:");
                    if ui.add(egui::TextEdit::singleline(&mut state.tts_settings.custom_http.model).desired_width(160.0)).changed() {
                        settings_changed = true;
                    }
                });
                ui.label(RichText::new("💡 兼容标准 OpenAI /v1/audio/speech 接口规范，可对接自建反代、Kokoro、CosyVoice 等任意私有服务。").size(11.0).color(Color32::from_gray(160)));
            }
        }

        ui.add_space(6.0);
        ui.separator();

        // 全局容灾与物理底噪选项
        responsive_flow(ui, |ui| {
            let mut auto_fallback = state.tts_settings.enable_auto_fallback;
            if ui.checkbox(&mut auto_fallback, "").changed() {
                state.tts_settings.enable_auto_fallback = auto_fallback;
                settings_changed = true;
            }
            ui.label(RichText::new("🛡 开启故障自动平滑降级（第三方接口欠费、超时或异常时无感回退至 Edge 官方音色）").size(12.5));
        });

        responsive_flow(ui, |ui| {
            let mut pink_noise = state.tts_settings.enable_pink_noise;
            if ui.checkbox(&mut pink_noise, "").changed() {
                state.tts_settings.enable_pink_noise = pink_noise;
                settings_changed = true;
            }
            ui.label(RichText::new("混入真实驻极体麦克风粉红底噪 (Pink Noise，破除机械纯净音)").size(12.5));
        });

        if state.tts_settings.enable_pink_noise {
            responsive_flow(ui, |ui| {
                ui.add_space(20.0);
                ui.label(RichText::new("衰减强度:").size(12.0));
                if compact_slider(ui, egui::Slider::new(&mut state.tts_settings.noise_level_db, -45.0..=-25.0).suffix(" dB")).changed() {
                    settings_changed = true;
                }
                ui.label(RichText::new("(推荐 -35.0 dB，自然模拟物理麦克风真实底噪)").size(11.0).color(Color32::from_gray(160)));
            });
        }

        if settings_changed {
            let _ = state.tts_settings.save();
        }

        ui.add_space(6.0);
        ui.separator();

        // 连通性测试与诊断看板
        responsive_flow(ui, |ui| {
            let btn_text = if state.tts_testing { "⏳ 正在合成并诊断..." } else { "🔊 测试当前引擎并诊断连通性" };
            if action_btn(ui, btn_text).clicked() && !state.tts_testing {
                state.report_feature_usage("voice_clone");
                state.tts_testing = true;
                state.tts_test_result = Some("正在向所选提供商发送测试请求...".to_string());
                let settings_clone = state.tts_settings.clone();

                std::thread::spawn(move || {
                    let start = std::time::Instant::now();
                    let test_text = "Good morning! Welcome to the English listening and speaking examination.";
                    let dispatcher = crate::workflow::tts::TtsDispatcher::new(settings_clone);
                    let req = crate::workflow::tts::TtsRequest::new(test_text);
                    let res = dispatcher.synthesize(&req);
                    let elapsed_ms = start.elapsed().as_millis();

                    let result_msg = match res {
                        Ok(resp) => {
                            let size_kb = resp.audio_bytes.len() as f64 / 1024.0;
                            format!(
                                "✔ 合成成功！[{}] 耗时: {} ms | 音频大小: {:.1} KB | 格式: {:?}",
                                resp.engine_identifier, elapsed_ms, size_kb, resp.format
                            )
                        }
                        Err(e) => {
                            format!("❌ 合成失败: {e} (耗时: {} ms)", elapsed_ms)
                        }
                    };
                    eprintln!("[TTS Test] {result_msg}");
                });
            }

            if let Some(msg) = &state.tts_test_result {
                let color = if msg.starts_with('✔') {
                    Color32::from_rgb(76, 175, 80)
                } else if msg.starts_with('❌') {
                    Color32::from_rgb(255, 82, 82)
                } else {
                    Color32::from_rgb(0, 180, 255)
                };
                ui.label(RichText::new(msg).color(color).size(11.5).strong());
            }
        });
    });

    ui.add_space(8.0);

    ui.card("关于 Fe PC", "软件信息与致谢", |ui| {
        let accent = state.theme_config.accent;
        ui.label(RichText::new("Fuck ets100 Desktop (Fe PC) v0.1.6").strong().size(15.0));
        ui.label("基于 Rust + fluent-egui + ets_resolver 构建的原生 Windows 11 现代化客户端。");
        ui.label("提供 e听说本地免权限试卷读取、云端作业拉取、全题型精准解析与全自动答题完成。");

        if let Some(ref policy) = state.software_policy {
            ui.add_space(4.0);
            ui.separator();
            ui.label(RichText::new("📢 服务端公告与版本发布说明").strong().color(accent.unwrap_or(Color32::from_rgb(0, 120, 215))));
            if let Some(ref title) = policy.announcement_title
                && !title.trim().is_empty() {
                ui.label(RichText::new(format!("📌 当前公告: {}", title)).strong());
            }
            let display_msg = policy.notice_message.as_deref()
                .or(policy.announcement_message.as_deref())
                .unwrap_or("");
            if !display_msg.trim().is_empty() {
                ui.label(RichText::new(display_msg).size(12.0));
            }
            // 版本更新提示与最新正式版本徽标判断 (严格按 policy.latest_version_code > LA_APP_VERSION_CODE)
            let is_new_version = policy.latest_version_code > crate::la_client::LA_APP_VERSION_CODE;
            if is_new_version {
                let update_msg = policy.update_message.as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("发现新版本，建议及时更新以获取最新功能与体验。");
                ui.label(RichText::new(format!("🚀 更新提示: {}", update_msg)).size(12.0).color(Color32::from_rgb(0, 150, 255)));
                if let Some(ref changelog) = policy.changelog_summary
                    && !changelog.trim().is_empty() {
                    ui.label(RichText::new(format!("📝 版本日志: {}", changelog)).size(11.5).color(Color32::from_gray(160)));
                }
            } else {
                ui.label(
                    RichText::new(format!("✔ 当前已是最新正式版本 (v{})", env!("CARGO_PKG_VERSION")))
                        .size(12.5)
                        .strong()
                        .color(Color32::from_rgb(76, 175, 80)),
                );
                if let Some(ref changelog) = policy.changelog_summary
                    && !changelog.trim().is_empty() {
                    ui.label(RichText::new(format!("📝 版本日志: {}", changelog)).size(11.5).color(Color32::from_gray(160)));
                }
            }

            responsive_flow(ui, |ui| {
                if is_new_version {
                    let dl_url = if !policy.update_url.trim().is_empty() {
                        Some(policy.update_url.as_str())
                    } else {
                        policy.announcement_url.as_deref().filter(|u| !u.trim().is_empty())
                    };
                    if let Some(url) = dl_url {
                        if action_btn(ui, "⬇ 前往下载新版本").clicked() {
                            let _ = open::that(url);
                        }
                    }
                }
                if action_btn(ui, "📢 查看公告与版本详情弹窗").clicked() {
                    state.announcement_modal_open = true;
                }
                if let Some(ref url) = policy.announcement_url
                    && !url.trim().is_empty()
                    && action_btn(ui, "🌐 打开外部公告网页").clicked() {
                    let _ = open::that(url);
                }
            });
        }

        ui.separator();

        ui.label("👤 开发者: code-leitianshuo (leitianshuo1337)");
        ui.label(RichText::new("🏷 软件性质: 原创闭源软件制品 (非商用 · 作者保留版权)").color(Color32::from_rgb(255, 179, 0)));
        ui.label(RichText::new("⚠️ 免责声明: 本软件仅供学习交流使用，请勿用于考试作弊等违规用途。").color(Color32::from_gray(160)));

        ui.add_space(4.0);
        responsive_flow(ui, |ui| {
            if action_btn(ui, "📄 查看用户协议与服务免责声明 (EULA)").clicked() {
                state.open_eula_modal(false);
            }
            if action_btn(ui, "📜 查看第三方开源库与组件致谢").clicked() {
                state.current_tab = crate::state::NavTab::Credits;
            }
        });
    });
}
