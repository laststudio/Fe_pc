use std::fs;

use eframe::egui::{Color32, RichText};
use fluent_egui::winui::prelude::*;
use fluent_egui::winui::{BackdropKind, FluentTheme, InfoBarKind};

use crate::state::AppState;
use crate::views::render_header;

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(ui, "设置与偏好", "自定义界面外观、系统材质与缓存管理");

    ui.card("界面外观与材质", "个性化 Windows 11 Fluent 视觉效果", |ui| {
        ui.row(|ui| {
            ui.label("主题模式:");
            let is_dark = state.theme_config.theme.is_dark();
            if ui.button(if is_dark { "🌙 当前: 深色模式" } else { "切换为深色" }).clicked() {
                state.theme_config.theme = FluentTheme::Dark;
            }
            if ui.button(if !is_dark { "☀ 当前: 浅色模式" } else { "切换为浅色" }).clicked() {
                state.theme_config.theme = FluentTheme::Light;
            }
        });

        ui.add_space(4.0);

        ui.row(|ui| {
            ui.label("窗口背景材质:");
            let backdrops = [
                (BackdropKind::Mica, "Mica (云母磨砂·Win11)"),
                (BackdropKind::Acrylic, "Acrylic (亚克力透光)"),
                (BackdropKind::Auto, "自动适配"),
                (BackdropKind::None, "纯色背景 (低资源)"),
            ];

            for (bk, label) in backdrops {
                let is_sel = state.backdrop_kind == bk;
                if ui.button(if is_sel { format!("✔ {}", label) } else { label.to_string() }).clicked() {
                    state.backdrop_kind = bk;
                    state.notification = Some((format!("已切换背景材质为: {}", label), InfoBarKind::Success));
                }
            }
        });

        ui.add_space(4.0);

        ui.row(|ui| {
            ui.label("主题强调色:");
            let colors = [
                ("默认蓝", Color32::from_rgb(0, 120, 215)),
                ("紫罗兰", Color32::from_rgb(138, 43, 226)),
                ("薄荷绿", Color32::from_rgb(46, 204, 113)),
                ("落霞橙", Color32::from_rgb(230, 126, 34)),
            ];

            for (name, col) in colors {
                if ui.button(name).clicked() {
                    state.theme_config = state.theme_config.with_accent(col);
                }
            }
        });
    });

    ui.add_space(8.0);

    ui.card("数据与缓存管理", "管理本地下载的试题资源和临时文件", |ui| {
        ui.row(|ui| {
            if ui.button("🗑 清理试卷临时下载缓存").clicked() {
                let cache_dir = std::env::temp_dir().join("fepc_cache");
                if cache_dir.exists() {
                    let _ = fs::remove_dir_all(&cache_dir);
                }
                state.notification = Some(("已成功清空本地试卷下载缓存".to_string(), InfoBarKind::Success));
            }

            if ui.button("📂 打开缓存文件夹").clicked() {
                let cache_dir = std::env::temp_dir().join("fepc_cache");
                let _ = fs::create_dir_all(&cache_dir);
                let _ = open::that(&cache_dir);
            }
        });
    });

    ui.add_space(8.0);

    ui.card("关于 Fe PC", "软件信息与致谢", |ui| {
        ui.label(RichText::new("Fuck ets100 Desktop (Fe PC) v0.1.0").strong().size(15.0));
        ui.label("基于 Rust + fluent-egui + ets_resolver 构建的原生 Windows 11 现代化客户端。");
        ui.label("提供 e听说本地免权限试卷读取、云端作业拉取、全题型精准解析与全自动答题完成。");

        ui.separator();

        ui.label("👨‍💻 开发者: code-leitianshuo (leitianshuo1337)");
        ui.label(RichText::new("🔒 软件性质: 专有商业软件 (非开源 / 保留所有权利)").color(Color32::from_rgb(255, 179, 0)));
        ui.label(RichText::new("⚠️ 免责声明: 本软件仅供学习交流使用，请勿用于考试作弊等违规用途。").color(Color32::from_gray(160)));

        ui.add_space(4.0);
        if ui.accent_button("📜 查看第三方开源库与组件致谢").clicked() {
            state.current_tab = crate::state::NavTab::Credits;
        }
    });
}
