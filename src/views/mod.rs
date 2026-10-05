// =========================================================================================
// ⚠️ UI 排版与组件规范说明（避免踩坑必读）：
// 
// 1. 【严禁使用上游 FluentButton 进行列表与操作栏排版】：
//    上游 `fluent_egui::winui::FluentButton`（widgets.rs:361）存在严重测宽缺陷：
//    `let width = if avail_w > 0.0 { raw_w.min(avail_w) } else { raw_w };`
//    当行尾剩余宽度不足时（如只剩 30~50px），FluentButton 会谎报自身需要 40px，
//    导致 egui 认为装得下而不换行，最终按钮被强行压缩压扁，文字截断为“背景 (低”。
//    👉 必须统一使用本模块封装的 `choice_btn`、`action_btn`、`accent_action_btn`、`danger_action_btn`！
//    它们基于原生 `egui::Button` 严格按实际文字像素测宽，放不下时会作为独立个体【整按钮平滑换行】。
//
// 2. 【严禁滥用 ui.row 刚性单行排版】：
//    `ui.row` 底层直接调用刚性不可折行的 `ui.horizontal`。如果行内包含多个按钮或长文本，
//    会强行向外申请宽度并撑爆父容器，导致视口右侧卡片边距（24px）被挤出屏幕裁切。
//    👉 必须使用 `responsive_flow(ui, ...)` 或 `ui.horizontal_wrapped(...)` 进行流式排版！
//
// 3. 【长文本与本地路径必须做防护截断】：
//    长路径或长文件名请使用 `truncate_middle(s, max_len)`，并通过 `.on_hover_text(s)` 显示完整路径，
//    杜绝长字符串撑大布局。
// =========================================================================================

use eframe::egui::{Color32, RichText};
use fluent_egui::winui::FluentUi;

pub mod announcement_modal;
pub mod answer_view;
pub mod auth_gate_modal;
pub mod auto_complete_view;
pub mod cloud_account;
pub mod homework_list;
pub mod local_scan;
pub mod settings;
pub mod credits;
pub mod eula_modal;

pub use announcement_modal::render_announcement_modal;
pub use auth_gate_modal::render_auth_gate_modal;
pub use eula_modal::render_eula_modal;

/// 优雅渲染页面标准标题与副标题，彻底移除 GalleryPageHeader 包含的无用 demo 按钮与图标
pub fn render_header(ui: &mut FluentUi, title: &str, subtitle: &str) {
    ui.heading(RichText::new(title).size(22.0).strong());
    if !subtitle.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(13.0).color(Color32::from_gray(160)));
    }
    ui.add_space(14.0);
}

/// 自适应流式排版容器（带标准 8px 间距，支持子组件整行无缝折行）
pub fn responsive_flow<R>(
    ui: &mut eframe::egui::Ui,
    add_contents: impl FnOnce(&mut eframe::egui::Ui) -> R,
) -> eframe::egui::InnerResponse<R> {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = eframe::egui::vec2(8.0, 8.0);
        add_contents(ui)
    })
}

/// 自适应动态测宽选择按钮（支持选中高亮、整按钮自动换行，杜绝上游 FluentButton 压扁截断与横向溢出）
pub fn choice_btn(
    ui: &mut eframe::egui::Ui,
    text: &str,
    is_selected: bool,
    accent: Option<Color32>,
) -> eframe::egui::Response {
    let display_text = if is_selected {
        format!("✔ {}", text)
    } else {
        text.to_string()
    };
    let mut btn = eframe::egui::Button::new(
        RichText::new(&display_text)
            .size(13.0)
            .color(if is_selected {
                Color32::WHITE
            } else {
                ui.visuals().widgets.inactive.text_color()
            }),
    )
    .min_size(eframe::egui::vec2(0.0, 30.0))
    .corner_radius(6.0);

    if is_selected {
        let fill = accent.unwrap_or(Color32::from_rgb(0, 120, 215));
        btn = btn.fill(fill);
    }

    ui.add(btn)
}

/// 自适应标准操作按钮（支持整按钮自动换行，杜绝上游 FluentButton 压扁截断与横向溢出）
pub fn action_btn(ui: &mut eframe::egui::Ui, text: &str) -> eframe::egui::Response {
    let btn = eframe::egui::Button::new(RichText::new(text).size(13.0))
        .min_size(eframe::egui::vec2(0.0, 30.0))
        .corner_radius(6.0);
    ui.add(btn)
}

/// 自适应强调色主操作按钮（如“开始做题”、“查看完整答案”）
pub fn accent_action_btn(
    ui: &mut eframe::egui::Ui,
    text: &str,
    accent: Option<Color32>,
) -> eframe::egui::Response {
    let fill = accent.unwrap_or(Color32::from_rgb(0, 120, 215));
    let btn = eframe::egui::Button::new(
        RichText::new(text)
            .size(13.0)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(fill)
    .min_size(eframe::egui::vec2(0.0, 30.0))
    .corner_radius(6.0);
    ui.add(btn)
}

/// 自适应警示/危险操作按钮（如“退出登录”、“解除绑定”）
pub fn danger_action_btn(ui: &mut eframe::egui::Ui, text: &str) -> eframe::egui::Response {
    let btn = eframe::egui::Button::new(
        RichText::new(text)
            .size(13.0)
            .color(Color32::from_rgb(255, 99, 99)),
    )
    .min_size(eframe::egui::vec2(0.0, 30.0))
    .corner_radius(6.0);
    ui.add(btn)
}

/// 自适应微弱/次要操作按钮（如“复制到剪贴板”）
pub fn subtle_action_btn(ui: &mut eframe::egui::Ui, text: &str) -> eframe::egui::Response {
    let btn = eframe::egui::Button::new(
        RichText::new(text)
            .size(13.0)
            .color(ui.visuals().widgets.inactive.text_color()),
    )
    .frame(false)
    .min_size(eframe::egui::vec2(0.0, 30.0));
    ui.add(btn)
}

/// 字符串中间省略算法（用于长路径、长文件名，防止撑爆容器，保留头尾可读性）
pub fn truncate_middle(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len || max_len < 7 {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let head_len = (max_len - 3) / 2;
    let tail_len = max_len - 3 - head_len;
    let head: String = chars[..head_len].iter().collect();
    let tail: String = chars[chars.len() - tail_len..].iter().collect();
    format!("{}...{}", head, tail)
}

/// 紧凑型现代 Fluent 2 滑块控件：限制滑块轨宽与手柄尺寸，杜绝超大圆球手柄与视口横向溢出
pub fn compact_slider<'a>(
    ui: &mut eframe::egui::Ui,
    slider: eframe::egui::Slider<'a>,
) -> eframe::egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 16.0; // 手柄直径精准控制在 14px 现代细腻尺寸
        ui.spacing_mut().slider_width = 110.0;    // 轨宽自适应收紧为 110px
        ui.add(slider)
    })
    .inner
}
