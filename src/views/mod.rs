use eframe::egui::{Color32, RichText};
use fluent_egui::winui::FluentUi;

pub mod answer_view;
pub mod auto_complete_view;
pub mod cloud_account;
pub mod homework_list;
pub mod local_scan;
pub mod settings;
pub mod credits;

/// 优雅渲染页面标准标题与副标题，彻底移除 GalleryPageHeader 包含的无用 demo 按钮与图标
pub fn render_header(ui: &mut FluentUi, title: &str, subtitle: &str) {
    ui.heading(RichText::new(title).size(22.0).strong());
    if !subtitle.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(13.0).color(Color32::from_gray(160)));
    }
    ui.add_space(14.0);
}
