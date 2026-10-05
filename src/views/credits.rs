use eframe::egui::{Color32, RichText};
use fluent_egui::winui::prelude::*;

use crate::state::AppState;
use crate::views::{accent_action_btn, action_btn, render_header, responsive_flow};

pub fn render(ui: &mut FluentUi, state: &mut AppState) {
    render_header(
        ui,
        "第三方开源库与组件致谢",
        "致敬开源：Fe PC 深度受益于全球优秀开发者与开源基础设施的支持，特此署名致谢",
    );

    // 软件许可与服务协议 (EULA) 专区
    ui.card("软件许可与服务协议 (EULA)", "专有商业软件许可 · 隐私政策 · 服务与安全免责声明", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Fe PC 最终用户许可协议与免责声明").strong().size(14.0));
            ui.label(RichText::new("(版本: 2026-10-05)").color(Color32::from_gray(140)).size(12.0));
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new("本软件为商业专有软件体系，受《中华人民共和国著作权法》与国际版权公约严格保护。\n\
            点击下方按钮可随时查阅完整的专有商业软件许可、第三方凭据隐私授权与服务免责声明。")
                .color(Color32::from_gray(180))
                .size(12.5),
        );
        ui.add_space(6.0);
        responsive_flow(ui, |ui| {
            if accent_action_btn(ui, "📄 查看完整用户协议与服务免责声明 (EULA)", Some(Color32::from_rgb(0, 120, 215))).clicked() {
                state.open_eula_modal(false);
            }
        });
    });

    ui.add_space(8.0);

    ui.card("开源致谢致辞 (Acknowledgments)", "致敬为生态无私奉献的开发者们", |ui| {
        ui.label(
            RichText::new(
                "Fe PC (Fuck ets100 Desktop) 是一款专为学生听力口语训练与复习设计的高效桌面辅助工具。\n\
                软件本身由 code-leitianshuo (leitianshuo1337) 独立开发并持有版权。\n\
                在构建过程中，我们深度使用了 Rust 语言生态中众多杰出的第三方开源项目与平台服务。\n\
                我们由衷感谢这些开源项目的作者、维护者和社区贡献者！",
            )
            .size(13.5)
            .color(Color32::from_gray(210)),
        );
    });

    ui.add_space(8.0);

    // 特别致谢 (fuck_ets100 - 帅丘)
    ui.card("特别致谢与灵感来源 (Special Thanks)", "致敬开源先锋与启发者", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("fuck_ets100").strong().color(Color32::from_rgb(0, 180, 255)).size(15.0));
            ui.label(RichText::new("(项目作者: 帅丘)").color(Color32::from_rgb(255, 200, 50)).strong().size(13.5));
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new(
                "特别鸣谢帅丘开发的 fuck_ets100 项目！该项目对 e听说 协议结构、本地缓存体系与试卷解析算法的前驱探索与技术分享，为本项目提供了极具价值的参考与灵感启发，特此深表谢意！",
            )
            .color(Color32::from_gray(190))
            .size(13.0),
        );
    });

    ui.add_space(8.0);

    // 核心图形与 UI 渲染引擎
    ui.card("图形界面与渲染体系 (UI & Graphics)", "Windows 11 Fluent 视觉交互与即时渲染", |ui| {
        let ui_crates = [
            (
                "fluent-egui",
                "v0.1.1",
                "MIT / Apache-2.0",
                "提供符合微软 Windows 11 WinUI 3 / Fluent Design 2 标准的卡片、导航栏、按钮、滑块与材质系统。",
            ),
            (
                "egui & eframe",
                "v0.35.0",
                "MIT / Apache-2.0",
                "Emilk 打造的高性能即时模式 (Immediate Mode) GUI 引擎与原生桌面窗口生命周期上下文。",
            ),
            (
                "glow",
                "v0.16",
                "MIT / Apache-2.0 / Zlib",
                "现代 OpenGL / OpenGL ES 跨平台强类型底层渲染绑定，支撑平滑的硬件加速与抗锯齿呈现。",
            ),
        ];

        for (name, ver, lic, desc) in ui_crates {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(name).strong().color(Color32::from_rgb(0, 180, 255)).size(14.0));
                ui.label(RichText::new(format!("({})", ver)).color(Color32::from_gray(140)).size(12.0));
                ui.label(RichText::new(format!("[{}]", lic)).color(Color32::from_rgb(76, 175, 80)).size(12.0));
            });
            ui.label(RichText::new(desc).color(Color32::from_gray(180)).size(12.5));
            ui.add_space(4.0);
        }
    });

    ui.add_space(8.0);

    // 系统底层交互与编码
    ui.card("系统底层集成与字符编码 (System & Encoding)", "Windows 原生 API、编码转换与本地交互", |ui| {
        let sys_crates = [
            (
                "encoding_rs",
                "v0.8.35",
                "Apache-2.0 / MIT",
                "Mozilla 研发的符合 WHATWG 规范的高性能字符集转换库，完美解决本地 GBK/CP936 中文解码问题。",
            ),
            (
                "windows-rs",
                "v0.62",
                "MIT / Apache-2.0",
                "微软官方提供的 Windows API 原生 Rust 强类型绑定，用于硬件识别、注册表与窗口材质同步。",
            ),
            (
                "rfd (Rusty File Dialog)",
                "v0.15",
                "MIT",
                "原生平台文件系统对话框封装，提供直观的目录选取与 TXT/Markdown 结果文件导出保存。",
            ),
            (
                "arboard",
                "v3.6",
                "MIT / Apache-2.0",
                "跨平台安全系统剪贴板读写库，提供毫秒级题目答案与速查清单复制能力。",
            ),
            (
                "open",
                "v5.3",
                "MIT",
                "安全调用操作系统默认程序拉起畅言登录网页、WebView2 及本地作业缓存目录。",
            ),
        ];

        for (name, ver, lic, desc) in sys_crates {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(name).strong().color(Color32::from_rgb(0, 180, 255)).size(14.0));
                ui.label(RichText::new(format!("({})", ver)).color(Color32::from_gray(140)).size(12.0));
                ui.label(RichText::new(format!("[{}]", lic)).color(Color32::from_rgb(76, 175, 80)).size(12.0));
            });
            ui.label(RichText::new(desc).color(Color32::from_gray(180)).size(12.5));
            ui.add_space(4.0);
        }
    });

    ui.add_space(8.0);

    // 网络通讯与数据解析
    ui.card("网络通讯与数据处理 (Network & Data Parsing)", "HTTP 通讯、密码学运算与流式处理", |ui| {
        let net_crates = [
            (
                "serde & serde_json",
                "v1.0",
                "MIT / Apache-2.0",
                "Rust 事实上的序列化/反序列化标准框架，支撑试卷题目结构与协议元数据的极速解析。",
            ),
            (
                "ureq",
                "v2.12",
                "MIT / Apache-2.0",
                "无依赖、轻量简洁的同步安全 HTTP 客户端，用于云端接口请求与 Token 会话同步。",
            ),
            (
                "tokio",
                "v1.43",
                "MIT",
                "异步运行时与任务调度库，为自动化答题流水线提供高效平稳的并发执行保障。",
            ),
            (
                "zip",
                "v2.2",
                "MIT",
                "纯 Rust 实现的 ZIP 归档读取引擎，与解密算法配合实现内存级极速解压。",
            ),
            (
                "md-5 & base64",
                "最新稳定版",
                "MIT / Apache-2.0",
                "提供基础数据安全哈希运算与二进制数据传输编码。",
            ),
        ];

        for (name, ver, lic, desc) in net_crates {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(name).strong().color(Color32::from_rgb(0, 180, 255)).size(14.0));
                ui.label(RichText::new(format!("({})", ver)).color(Color32::from_gray(140)).size(12.0));
                ui.label(RichText::new(format!("[{}]", lic)).color(Color32::from_rgb(76, 175, 80)).size(12.0));
            });
            ui.label(RichText::new(desc).color(Color32::from_gray(180)).size(12.5));
            ui.add_space(4.0);
        }
    });

    ui.add_space(8.0);

    // AI 与语音评测生态
    ui.card("AI 语音技术生态致谢 (AI & Speech Ecosystem)", "高质量自然人声合成与口语评分协议", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Microsoft Edge Neural TTS").strong().color(Color32::from_rgb(0, 180, 255)).size(14.0));
            ui.label(RichText::new("(Azure Cognitive Services)").color(Color32::from_gray(140)).size(12.0));
        });
        ui.label(RichText::new("感谢微软提供的优质神经网络自然语音技术，赋予口语作答自然逼真的流利人声。").color(Color32::from_gray(180)).size(12.5));

        ui.add_space(4.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("iFlytek ISE (科大讯飞智能评测协议)").strong().color(Color32::from_rgb(0, 180, 255)).size(14.0));
        });
        ui.label(RichText::new("提供官方口语评测标准的流式评分算法与准确度/完整度/流利度矩阵生成依据。").color(Color32::from_gray(180)).size(12.5));
    });

    ui.add_space(8.0);

    ui.card("开发者与软件版权信息", "关于开发者与使用规范", |ui| {
        ui.label(RichText::new("开发者: code-leitianshuo (leitianshuo1337)").strong().size(15.0));
        ui.label(RichText::new("软件性质: 专有软件 (Proprietary / Closed Source) · 保留所有权利").color(Color32::from_rgb(255, 179, 0)));
        ui.label("本软件所有代码与设计由作者独立开发，不采用开源协议发布，未经授权请勿逆向或二次分发。");
        ui.add_space(6.0);
        responsive_flow(ui, |ui| {
            if action_btn(ui, "📄 查看用户协议与服务免责声明 (EULA)").clicked() {
                state.open_eula_modal(false);
            }
        });
    });
}
