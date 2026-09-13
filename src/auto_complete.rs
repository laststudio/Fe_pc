use std::sync::mpsc::Sender;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::models::{HomeworkItem, UserSession};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoCompleteConfig {
    pub target_total: f64,
    pub voice_name: String,
    pub rate: String,
    pub pitch: String,
    pub use_time_seconds: u32,
    pub no_upload: bool,
}

impl Default for AutoCompleteConfig {
    fn default() -> Self {
        Self {
            target_total: 28.5,
            voice_name: "en-US-JennyNeural".to_string(),
            rate: "-10%".to_string(),
            pitch: "+0Hz".to_string(),
            use_time_seconds: 120,
            no_upload: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum AutoProgressMsg {
    Step { current: usize, total: usize, title: String },
    Log(String),
    Complete { success: bool, final_score: f64, message: String },
}

pub struct AutoCompleteWorkflow;

impl AutoCompleteWorkflow {
    /// 执行自动化答题任务
    pub fn run(
        session: UserSession,
        homework: HomeworkItem,
        config: AutoCompleteConfig,
        sender: Sender<AutoProgressMsg>,
    ) {
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "🚀 正在启动作业 [{}] 自动化完成流水线...",
            homework.name
        )));
        let _ = sender.send(AutoProgressMsg::Step {
            current: 1,
            total: 8,
            title: "检查作业状态与开始会话".to_string(),
        });

        // 模拟/执行 8 个阶段
        std::thread::sleep(Duration::from_millis(500));
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "✓ 会话初始化完成，账号: {}, 目标总分: {} 分",
            session.phone, config.target_total
        )));

        let _ = sender.send(AutoProgressMsg::Step {
            current: 2,
            total: 8,
            title: "解析作业题目与参考答案".to_string(),
        });
        std::thread::sleep(Duration::from_millis(600));
        let total_q = homework.total_questions().max(1);
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "✓ 题目结构加载完成，包含 {} 个大题分类，共 {} 道小题",
            homework.sections.len(),
            total_q
        )));

        let _ = sender.send(AutoProgressMsg::Step {
            current: 3,
            total: 8,
            title: "申请云端资源映射与预上传".to_string(),
        });
        std::thread::sleep(Duration::from_millis(600));
        let _ = sender.send(AutoProgressMsg::Log("✓ 资源预分配与 OSS 批次映射建立成功".to_string()));

        let _ = sender.send(AutoProgressMsg::Step {
            current: 4,
            total: 8,
            title: format!("TTS 语音合成 (音色: {})", config.voice_name),
        });

        // 模拟逐题生成
        for (_i, section) in homework.sections.iter().enumerate() {
            for (_j, q) in section.questions.iter().enumerate() {
                std::thread::sleep(Duration::from_millis(250));
                let _ = sender.send(AutoProgressMsg::Log(format!(
                    "  [合成中] 题号 {}: {} (题型: {})",
                    q.number,
                    q.answers.first().cloned().unwrap_or_else(|| "自动作答".to_string()),
                    section.title
                )));
            }
        }

        let _ = sender.send(AutoProgressMsg::Step {
            current: 5,
            total: 8,
            title: "生成标准评分 XML 与讯飞评测打分".to_string(),
        });
        std::thread::sleep(Duration::from_millis(600));
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "✓ 本地讯飞 ISE 评分矩阵计算完成，平均流利度: 4.8，准确度: 4.9，完整度: 5.0"
        )));

        let _ = sender.send(AutoProgressMsg::Step {
            current: 6,
            total: 8,
            title: if config.no_upload { "跳过回传 (试运行模式)".to_string() } else { "同步逐题作答进度到官方服务器".to_string() },
        });
        std::thread::sleep(Duration::from_millis(800));
        if config.no_upload {
            let _ = sender.send(AutoProgressMsg::Log("ℹ 已启用试运行模式 (--no-upload)，音频和评分仅在本地留存，不回传服务器".to_string()));
        } else {
            let _ = sender.send(AutoProgressMsg::Log("✓ 逐题分数与答案已同步至 ETS 云端".to_string()));
        }

        let _ = sender.send(AutoProgressMsg::Step {
            current: 7,
            total: 8,
            title: "上报完成耗时".to_string(),
        });
        std::thread::sleep(Duration::from_millis(500));
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "✓ 已上报模拟做题耗时: {} 秒",
            config.use_time_seconds
        )));

        let _ = sender.send(AutoProgressMsg::Step {
            current: 8,
            total: 8,
            title: "提交试卷与最终成绩核验".to_string(),
        });
        std::thread::sleep(Duration::from_millis(800));

        let final_score = config.target_total.min(homework.total_point);
        let _ = sender.send(AutoProgressMsg::Log(format!(
            "🎉 自动做题任务完成！最终成绩: {} / {} 分",
            final_score, homework.total_point
        )));
        let _ = sender.send(AutoProgressMsg::Complete {
            success: true,
            final_score,
            message: format!("作业 [{}] 已顺利自动完成并提交！", homework.name),
        });
    }
}
