use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub const CURRENT_EULA_VERSION: &str = "2026-10-06";
pub const CURRENT_EULA_TITLE: &str = "Fe PC 最终用户使用许可协议（EULA）、原创版权声明与服务免责声明";
pub const CURRENT_EULA_EFFECTIVE_DATE: &str = "2026 年 10 月 6 日";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EulaConsentRecord {
    pub accepted: bool,
    pub version: String,
    pub accepted_at: String, // ISO8601
    pub app_version: String,
    pub app_version_code: i64,
}

#[derive(Debug, Clone)]
pub struct EulaSection {
    pub id: &'static str,
    pub title: &'static str,
    pub badge: Option<&'static str>,
    pub is_warning: bool,
    pub items: Vec<(&'static str, &'static str)>,
}

/// 获取 eula_accepted.json 存储路径
/// 优先环境变量 FEPC_EULA_PATH（用于自动化测试），其次 %APPDATA%\fepc\eula_accepted.json，回退当前目录
pub fn get_eula_file_path() -> PathBuf {
    if let Ok(custom) = std::env::var("FEPC_EULA_PATH")
        && !custom.trim().is_empty()
    {
        return PathBuf::from(custom);
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        let dir = PathBuf::from(app_data).join("fepc");
        let _ = fs::create_dir_all(&dir);
        return dir.join("eula_accepted.json");
    }

    PathBuf::from("eula_accepted.json")
}

/// 检查当前用户是否已同意最新版 EULA
pub fn is_eula_accepted() -> bool {
    // 环境变量覆盖（用于集成测试或无头模式）
    if let Ok(val) = std::env::var("FEPC_FORCE_EULA_MODAL")
        && (val == "1" || val.eq_ignore_ascii_case("true"))
    {
        return false;
    }
    if let Ok(val) = std::env::var("FEPC_AUTO_ACCEPT_EULA")
        && (val == "1" || val.eq_ignore_ascii_case("true"))
    {
        return true;
    }

    let path = get_eula_file_path();
    if !path.exists() {
        return false;
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let record: EulaConsentRecord = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(_) => return false,
    };

    record.accepted && record.version == CURRENT_EULA_VERSION
}

/// 保存用户同意状态至本地磁盘
pub fn save_eula_accepted() -> Result<(), String> {
    let path = get_eula_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let record = EulaConsentRecord {
        accepted: true,
        version: CURRENT_EULA_VERSION.to_string(),
        accepted_at: crate::telemetry::crash_handler::current_iso8601(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        app_version_code: crate::la_client::LA_APP_VERSION_CODE,
    };

    let json_str = serde_json::to_string_pretty(&record)
        .map_err(|e| format!("序列化 EULA 记录失败: {e}"))?;

    fs::write(&path, json_str)
        .map_err(|e| format!("保存 EULA 同意文件失败 ({}): {e}", path.display()))?;

    Ok(())
}

/// 重置 EULA 状态（用于测试）
pub fn reset_eula_accepted() -> Result<(), String> {
    let path = get_eula_file_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("删除 EULA 文件失败: {e}"))?;
    }
    Ok(())
}

/// 获取全部结构化协议章节数据（供 UI 高性能渲染）
pub fn get_eula_sections() -> Vec<EulaSection> {
    vec![
        EulaSection {
            id: "preface",
            title: "重要合规提示与使用前提",
            badge: Some("使用前提"),
            is_warning: true,
            items: vec![
                (
                    "软件属性与非商业性质声明",
                    "本软件（Fe PC / Fuck ETS100 Desktop）为作者个人独立研发并维护的原创闭源软件制品（Closed Source），属于个人学习研究辅助工具，非商业公司或法人机构运营的商业软件体系。本软件所有历史早期探索阶段的开源许可协议（包括但不限于 GPL 3.0 等历史协议）已全数终止并彻底废除，历史许可条款不再具有任何法律约束力。",
                ),
                (
                    "完全民事行为能力与自愿受约",
                    "在使用本软件或关联云端服务前，请你务必审慎阅读、充分理解各条款内容。使用本软件或关联云端服务即代表你已年满 18 周岁并具备完全民事行为能力，并自愿接受本协议全部条款约束。如你不同意本协议的任一内容，请立即停止使用并彻底退出卸载本软件。",
                ),
            ],
        },
        EulaSection {
            id: "section-1",
            title: "一、软件性质、原创版权与受限使用许可（EULA）",
            badge: Some("核心许可"),
            is_warning: false,
            items: vec![
                (
                    "原创版权归属与知识产权真实状态（如实告知）",
                    "【如实告知】本软件目前尚未向国家版权管理部门申请办理计算机软件著作权登记（无官方软著登记证书）；但依据《中华人民共和国著作权法》，中国公民的作品自创作完成之日起依法自动产生著作权与版权。本软件全部原创源码、二进制制品、底层解密引擎、UI 交互设计之原创版权独家归属于作者个人（code-leitianshuo / 帅丘）所有，作者依法保留全部权利。",
                ),
                (
                    "受限个人学习许可",
                    "作者仅授予用户一项个人的、非排他性、不可转让、不可分许可且可由作者随时撤销的有限客户端使用许可。该许可仅允许用户本人在合法的个人 Windows 桌面计算设备上，进行合规正当的个人课后英语口语发音辅助练习与本地试卷归档解析，严禁用于任何商业目的。",
                ),
                (
                    "严禁逆向工程与破解攻击",
                    "严禁任何个人、机构或商业组织对本软件客户端安装包、底层机器码、通信鉴权接口、中继网关及后端系统进行反编译、反汇编、动态调试、脱壳解密、中间人抓包分析、接口劫持、伪造服务端签名响应，或制作任何形式的外挂、注入插件、辅助补丁及衍生破解版本。一经发现，将直接封禁账号与设备硬件，并保留追究法律责任之权利。",
                ),
                (
                    "严禁未授权转售与商业牟利",
                    "严禁在未经作者官方书面许可的情况下，将本软件安装包、授权卡密、云端解析接口等资源进行二次打包分发、公开倒卖、收费转售、租借，或利用本软件对外提供任何形式的有偿代读、代刷作业商业服务。若发现此类行为，平台有权直接永久封禁关联账号及设备，并依法追究民事赔偿与法律责任。",
                ),
            ],
        },
        EulaSection {
            id: "section-2",
            title: "二、LA Studio 账号系统与会员权益规范",
            badge: Some("账号服务"),
            is_warning: false,
            items: vec![
                (
                    "账号归属权与安全保管",
                    "用户在平台注册的 LA Studio 账号所有权归平台所有，用户仅享有受限的使用权。用户须自行妥善保管账号密码、会话 Token 及安全凭证。因用户主动泄露、共享、出租、转借他人导致的一切账号损失、数据异常与安全事故，均由用户自行全权承担。",
                ),
                (
                    "设备绑定与防刷风控机制",
                    "为保障系统安全与防刷秩序，平台实行严格的设备绑定与防刷策略（包括一机一码、多端登录互斥检测、高频 API 并发限流、环境完整性度量）。若系统检测到多设备异常共享、批量刷取接口等违规行为，LA Studio 后端将自动触发风控拦截与账号冻结。",
                ),
                (
                    "虚拟增值服务与违规处置",
                    "用户通过卡密兑换或官方渠道获得的会员等级（Pro / Ultra 会员）、功能权益与解析额度均属于平台提供的虚拟增值技术服务。若用户违反法律法规、学术道德或本协议任一规定，服务方有权在无需事先通知且不予退款的前提下，独立作出限制部分或全部功能、重置会员权益或永久封禁账号的处罚措施。",
                ),
            ],
        },
        EulaSection {
            id: "section-3",
            title: "三、第三方凭据收集、云端存储与代登录专项隐私授权",
            badge: Some("隐私授权"),
            is_warning: false,
            items: vec![
                (
                    "凭据收集与加密存储告知",
                    "本软件的“云端模式”及远程作业拉取功能需与第三方在线教育平台（包括但不限于 ETS100 / e听说 / 畅言教育等官方系统）进行网络交互。用户在此充分知悉、完全理解并主动授权：为了免除繁琐的手动本地导题流程，实现云端自动化代登录、拉取作答列表及作业答案数据，LA Studio 后端服务器系统将收集、传输并在服务器端加密存储用户提交的第三方平台（ETS100/e听说）账号（手机号/学号）、密码凭据、Session Token 及服务端设备标识码。",
                ),
                (
                    "排他性合法用途承诺（严正承诺）",
                    "开发者与 LA Studio 服务平台作出严肃郑重承诺：服务端收集与存储的前述第三方账号密码凭据，仅严格、排他性地用于系统代用户向第三方平台接口发起合法的身份校验、状态维持与作业答案解析等本软件明示之必要技术功能；开发者绝不将用户第三方账号密码用于任何超出本软件功能之外的用途，绝不用于侵犯用户隐私、破坏数据完整性或其他违法违规用途，绝不向任何与服务无关的第三方出售、出租、共享或披露。",
                ),
                (
                    "传输与存储安全防护",
                    "平台采用行业标准的高强度传输层加密协议（TLS/HTTPS）及后端对称/非对称混合加密存储策略对用户凭据进行防护，采取严格访问控制与审计机制防范未授权访问或数据泄露。",
                ),
                (
                    "委托代理与用户自愿原则",
                    "用户明确知晓：提交第三方平台账号密码并在云端进行代登录，属于用户个人自主发起并全权委托服务端的代理行为。用户必须确保其提交的账号密码为其本人合法拥有，用户完全自愿承担向第三方平台发起网络请求的一切平台规则与法律风险。",
                ),
            ],
        },
        EulaSection {
            id: "section-4",
            title: "四、服务与安全免责声明（责任限制）",
            badge: Some("责任限制"),
            is_warning: true,
            items: vec![
                (
                    "服务范围与适用性覆盖",
                    "本免责条款不可分割地适用于本产品生态下的所有组成部分，包括但不限于：Fe PC 桌面客户端各版本、底层解密库、LA Studio 账号系统、云端鉴权中心、云端中继代理服务器、远程配置接口、题目解密分发系统以及所有关联的技术工具与服务节点（统称“关联服务”）。",
                ),
                (
                    "按“现状”（AS-IS）提供与无担保",
                    "软件及全部关联服务均严格按“现状”（AS IS）及“现有”（AS AVAILABLE）的基础提供。开发者及服务提供方在适用法律允许的最大范围内，明确否认并免除任何明示、默示或法定的保证与担保，包括但不限于适销性、特定用途的适用性、服务持续不中断、无差错、无漏洞或不受第三方网络干扰。",
                ),
                (
                    "第三方平台非官方与无背书声明",
                    "本软件及关联服务为独立技术研究与效率辅助工具，与科大讯飞、广东音像教材出版社、e听说（ETS100）、畅言、各级学校、教育管理部门等任何官方机构无任何隶属、投资、合作、代理或官方背书关系。",
                ),
                (
                    "第三方账号“顶号”与封号风险免责",
                    "用户知悉第三方在线教育平台实行单设备互斥登录原则。使用云端模式可能导致官方客户端被退出登录（即“顶号”）；第三方平台可能随时调整风控规则，对非官方接口调用判定为异常并采取异地警告、限制功能、甚至封禁第三方账号。因使用本软件或关联云端服务导致的任何第三方平台账号异常、被迫下线、无法提交答卷、账号被风控或封禁等一切损失，开发者及关联服务方概不承担任何责任与补偿。",
                ),
                (
                    "学业与考试风险绝对免责（严禁考试作弊）",
                    "⚠️ 严禁用户在正规考试、模拟测验、随堂测验、课堂实时练习或实时录音提交等关键场景中使用本软件或关联服务！本软件仅供课后个人技术研究、数据解析与作业备份。用户必须恪守学术诚信与校纪校规，严禁用于考试作弊、弄虚作假或规避教学监管。因用户违规使用导致考试中断、无法交卷、答案丢失、被判定作弊、成绩作废、受校纪处分或引发任何纠纷的，所有后果与法律责任均由用户个人全权承担，开发者及关联服务方概不承担任何连带法律责任。",
                ),
                (
                    "用户违规责任自负与追偿承诺",
                    "若用户利用本软件从事任何违法违规、侵犯第三方知识产权或妨害计算机信息系统安全的行为，一切民事、行政或刑事法律后果均由用户个人独立承担。若用户的违规行为导致开发者或 LA Studio 关联服务方遭受行政处罚、民事诉讼或商誉损失，用户承诺无条件全额赔偿开发者所承受的一切直接与间接经济损失（包括但不限于诉讼费、律师费、罚金与赔偿金）。",
                ),
                (
                    "间接损害与责任排除",
                    "在法律法规允许的最大限度内，无论是基于侵权、违约、过失还是任何其他法律理论，开发者、运营者及关联服务提供方均不对用户或任何第三方所遭受的任何间接、附带、惩罚性、特殊或后果性损害（包括学业受损、预期收益损失、数据损坏、硬件故障等）承担任何赔偿责任。",
                ),
            ],
        },
        EulaSection {
            id: "section-5",
            title: "五、服务的变更、中断、限制与终止",
            badge: Some("服务变更"),
            is_warning: false,
            items: vec![
                (
                    "服务调整与终止权",
                    "开发者及服务运营方保留根据业务运营调整、政策法规监管、第三方接口变动或服务器维护升级需要，在无需向用户发出个别事先通知的情况下，随时修改、限制、暂停、中断或永久终止部分或全部客户端功能与云端服务的权利。",
                ),
                (
                    "不可抗力免责",
                    "对于因第三方接口失效、网络攻击、通信故障、机房停电、政策调整等不可抗力或常规维护导致的服务中断、延迟或数据无法读取，服务方无需对用户或任何第三方承担任何违约或补偿责任。",
                ),
            ],
        },
        EulaSection {
            id: "section-6",
            title: "六、适用法律与争议解决",
            badge: Some("法律管辖"),
            is_warning: false,
            items: vec![
                (
                    "准据法",
                    "本协议之订立、生效、解释、履行及纠纷解决均适用中华人民共和国大陆地区之法律法规。",
                ),
                (
                    "条款可分割性",
                    "本协议任何条款若因违反强制性法律规范而被具有管辖权的法院认定为无效或不可执行，该条款在必要范围内予以剥离，不影响其余条款的法律效力与可执行性。",
                ),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eula_sections_integrity() {
        let sections = get_eula_sections();
        assert_eq!(sections.len(), 7); // preface + sections 1-6
        assert_eq!(sections[0].id, "preface");
        assert!(sections[0].is_warning);
        assert_eq!(sections[4].id, "section-4");
        assert!(sections[4].is_warning); // 免责声明带警告

        let total_items: usize = sections.iter().map(|s| s.items.len()).sum();
        assert!(total_items >= 15);
    }

    #[test]
    fn test_eula_persistence_and_version_check() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("test_eula_{}.json", uuid::Uuid::new_v4()));
        unsafe {
            std::env::set_var("FEPC_EULA_PATH", test_file.to_str().unwrap());
        }

        // 1. 初始状态：未同意
        assert!(!is_eula_accepted());

        // 2. 保存同意状态
        save_eula_accepted().expect("保存 EULA 失败");
        assert!(test_file.exists());

        // 3. 校验状态：已同意
        assert!(is_eula_accepted());

        // 4. 读取文件并验证 JSON 格式
        let content = fs::read_to_string(&test_file).expect("读取临时文件失败");
        let record: EulaConsentRecord = serde_json::from_str(&content).expect("反序列化失败");
        assert!(record.accepted);
        assert_eq!(record.version, CURRENT_EULA_VERSION);
        assert!(!record.accepted_at.is_empty());

        // 5. 模拟旧版本协议 (如 "2025-01-01")，应该失效
        let old_record = EulaConsentRecord {
            accepted: true,
            version: "2025-01-01".to_string(),
            accepted_at: record.accepted_at.clone(),
            app_version: "0.1.0".to_string(),
            app_version_code: 100,
        };
        fs::write(&test_file, serde_json::to_string(&old_record).unwrap()).unwrap();
        assert!(!is_eula_accepted(), "旧版本 EULA 不应被认定为有效");

        // 6. 重置并清理
        reset_eula_accepted().expect("重置失败");
        assert!(!test_file.exists());
        assert!(!is_eula_accepted());

        unsafe {
            std::env::remove_var("FEPC_EULA_PATH");
        }
    }
}
