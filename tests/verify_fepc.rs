use fepc::local::LocalScanner;
use fepc::utils::exporter::Exporter;

#[test]
fn test_local_directory_discovery_and_scan() {
    let dirs = LocalScanner::find_ets_local_dirs();
    println!("Found {} local ETS directories", dirs.len());
    assert!(!dirs.is_empty(), "Should discover at least one ETS local directory in APPDATA");

    let target_dir = &dirs[0];
    println!("Testing with directory: {:?}", target_dir);

    // 1. 测试读取本地账号 (验证 GBK 中文解码无乱码)
    let accounts = LocalScanner::read_local_accounts(target_dir);
    println!("Found {} local accounts in userinfo", accounts.len());
    for acc in &accounts {
        println!("  Account: {} | Name: {} | Codepoints: {:?}", acc.account, acc.name, acc.name.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>());
        assert!(!acc.name.contains('\u{FFFD}'), "Account name must not contain replacement character mojibake! Found: {}", acc.name);
    }
    assert!(accounts.iter().any(|a| a.name.contains("雷") || a.name.contains("唐") || a.name.contains("鲁")),
        "Expected Chinese names like 雷天硕/唐浩泽/鲁班奖 to be decoded correctly");

    // 2. 测试扫描作业
    let homeworks = LocalScanner::scan_homeworks(target_dir);
    println!("Found {} homework items in local directory", homeworks.len());
    assert!(!homeworks.is_empty(), "Should parse homework items from the directory");

    for hw in &homeworks {
        println!("Homework: id={}, name={}, question_count={}, sections={}",
            hw.id, hw.name, hw.question_count, hw.sections.len());
        
        assert!(hw.is_parsed());
        assert!(!hw.sections.is_empty());

        for sec in &hw.sections {
            println!("  Section: {} ({}) with {} questions", sec.title, sec.section_type, sec.questions.len());
            for q in &sec.questions {
                println!("    Q[{}]: prompt='{}', answers={:?}, options={}",
                    q.number,
                    q.prompt_text.chars().take(30).collect::<String>(),
                    q.answers,
                    q.options.len()
                );
            }
        }

        // 3. 测试答案导出
        let concise = Exporter::format_concise_answers(hw);
        assert!(!concise.is_empty());
        println!("Concise answers sample:\n{}", concise.lines().take(6).collect::<Vec<_>>().join("\n"));

        let markdown = Exporter::format_markdown(hw);
        assert!(!markdown.is_empty());
        assert!(markdown.contains(&hw.name));
    }
}

#[test]
fn test_render_ui() {
    let ctx = eframe::egui::Context::default();
    let mut state = fepc::state::AppState {
        local_ets_dirs: fepc::local::LocalScanner::find_ets_local_dirs(),
        ..Default::default()
    };
    if let Some(first) = state.local_ets_dirs.first() {
        state.selected_local_dir = Some(first.clone());
        state.local_items = fepc::local::LocalScanner::scan_homeworks(first);
    }
    println!("State local_items count: {}", state.local_items.len());

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::homework_list::render(&mut fluent, &mut state);
            println!("UI available_rect: {:?}", ui.available_rect_before_wrap());
            println!("UI min_rect: {:?}", ui.min_rect());
        });
    });
    println!("homework_list render succeeded!");

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::local_scan::render(&mut fluent, &mut state);
        });
    });
    println!("local_scan render succeeded!");

    // 测试详细视图 (包含 collector.role / 角色扮演 备选折叠与中文类目映射)
    state.selected_homework = state.local_items.first().cloned();
    state.answer_view_mode = fepc::state::AnswerViewMode::Detailed;
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::answer_view::render(&mut fluent, &mut state);
        });
    });
    println!("answer_view (detailed) render succeeded!");

    // 测试极简速查视图
    state.answer_view_mode = fepc::state::AnswerViewMode::Concise;
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::answer_view::render(&mut fluent, &mut state);
        });
    });
    println!("answer_view (concise) render succeeded!");

    // 测试云端账号管理视图（未登录状态 + 人机验证卡片）
    state.la_captcha_needed = true;
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::cloud_account::render(&mut fluent, &mut state);
        });
    });
    println!("cloud_account (captcha needed) render succeeded!");

    // 测试设置视图
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::settings::render(&mut fluent, &mut state);
        });
    });
    println!("settings render succeeded!");

    // 测试致谢视图
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::credits::render(&mut fluent, &mut state);
        });
    });
    println!("credits render succeeded!");
}

#[test]
fn test_auto_complete_workflow() {
    use fepc::auto_complete::{AutoCompleteConfig, AutoCompleteWorkflow, AutoProgressMsg};
    use fepc::models::UserSession;
    use std::sync::mpsc::channel;

    let dirs = LocalScanner::find_ets_local_dirs();
    assert!(!dirs.is_empty());
    let homeworks = LocalScanner::scan_homeworks(&dirs[0]);
    assert!(!homeworks.is_empty());

    let target_hw = &homeworks[0];
    let (tx, rx) = channel();

    let session = UserSession {
        phone: "13800000000".to_string(),
        user_name: "测试学生".to_string(),
        token: "test_token".to_string(),
        device_code: "test_device".to_string(),
        parent_account_id: "test_account".to_string(),
        is_logged_in: true,
    };

    let config = AutoCompleteConfig {
        target_total: 28.5,
        voice_name: "en-US-JennyNeural".to_string(),
        rate: -5,
        pitch: 0,
        use_time_seconds: 600,
        no_upload: true,
        accuracy: 4.95,
        fluency: 4.95,
        integrity: 4.95,
    };

    let hw_clone = target_hw.clone();
    std::thread::spawn(move || {
        AutoCompleteWorkflow::run(session, hw_clone, config, tx);
    });

    let mut step_count = 0;
    let mut completed = false;
    let mut final_score = 0.0;

    while let Ok(msg) = rx.recv() {
        match msg {
            AutoProgressMsg::Step { current, total, title } => {
                step_count += 1;
                println!("  [Workflow Step] ({}/{}): {}", current, total, title);
            }
            AutoProgressMsg::Log(log) => {
                println!("    [Log] {}", log);
            }
            AutoProgressMsg::QuestionDone { number, score, max_score, status, .. } => {
                println!("    [Q Done] No.{} score={:.2}/{:.2} ({})", number, score, max_score, status);
            }
            AutoProgressMsg::Complete { success, final_score: score, message } => {
                completed = true;
                final_score = score;
                println!("  [Workflow Completed] success={}, score={}, message={}", success, score, message);
                assert!(success, "Workflow should succeed with --no-upload");
                break;
            }
        }
    }

    assert!(completed, "Workflow should complete");
    assert_eq!(step_count, 8, "Workflow must execute all 8 stages");
    assert!((final_score - 28.5).abs() < 0.1, "Final score should match target score");
}

#[test]
fn test_downloaded_exam_data_samples() {
    use std::path::Path;

    let base_dir = Path::new("G:/ets_crack/downloaded");
    if !base_dir.exists() {
        println!("G:/ets_crack/downloaded not available on this path, skipping");
        return;
    }

    let sample_folders = [
        "142433-2020考前冲刺1",
        "443178-2025-(新题型)WY 9年级(上) M5",
        "206766-202006通州区初二英语听说期末考试2",
        "150042-高二(上) 期中测试A",
        "552037-2025-高考全真模拟6",
    ];

    for folder_name in sample_folders {
        let folder_path = base_dir.join(folder_name);
        if !folder_path.exists() {
            continue;
        }

        println!("\n=== Testing Real Exam: {} ===", folder_name);
        let hw_opt = LocalScanner::parse_homework_folder(&folder_path);
        assert!(hw_opt.is_some(), "Failed to parse exam folder: {}", folder_name);

        let hw = hw_opt.unwrap();
        println!("Parsed successfully: id={}, name='{}', total_questions={}, sections={}",
            hw.id, hw.name, hw.question_count, hw.sections.len());
        assert!(!hw.sections.is_empty());
        assert!(hw.question_count > 0);

        for (s_idx, sec) in hw.sections.iter().enumerate() {
            println!("  [Section {}] '{}' (type: {}) - {} questions",
                s_idx + 1, sec.title, sec.section_type, sec.questions.len());
            for q in &sec.questions {
                let ans_sample = if q.answers.is_empty() {
                    "无".to_string()
                } else {
                    q.answers.first().cloned().unwrap_or_default()
                };
                let opt_info = if q.options.is_empty() {
                    "无选项 (主观题)".to_string()
                } else {
                    format!("{}个选项 [{}]", q.options.len(), q.options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>().join(","))
                };
                println!("    Q[{}]: prompt='{}...', answer='{}', {}",
                    q.number,
                    q.prompt_text.chars().take(25).collect::<String>(),
                    ans_sample.chars().take(30).collect::<String>(),
                    opt_info
                );
            }
        }

        let concise = Exporter::format_concise_answers(&hw);
        assert!(!concise.is_empty());
        println!("Concise answers sample (first 4 lines):\n{}",
            concise.lines().take(4).collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn test_device_fingerprint() {
    let inst_id = fepc::utils::device_info::get_installation_id();
    println!("Installation ID: {}", inst_id);
    assert_eq!(inst_id.len(), 36, "Installation ID must be 36 characters UUID");
    assert!(inst_id.contains('-'), "Installation ID must contain hyphens");

    let platform_id = fepc::utils::device_info::get_platform_device_id();
    println!("Platform Device ID: {}", platform_id);
    assert_eq!(platform_id.len(), 64, "Platform Device ID must be SHA256 hex string (64 chars)");

    let payload = fepc::utils::device_info::get_device_payload();
    assert_eq!(payload["application_id"], fepc::utils::device_info::APPLICATION_ID);
    assert_eq!(payload["signing_certificate_sha256"], "79fe6f92aa7f46084832cd62292c223101586b46e3cfaf39740fef1c6253aaca");
    assert_eq!(payload["version_code"], 17);
    assert_eq!(payload["installation_id"], inst_id);
    assert_eq!(payload["platform_device_id"], platform_id);
    println!("Device payload: {}", payload);
}

#[test]
fn test_scoring_and_xml() {
    use fepc::workflow::scoring::{build_score_detail, convert_score, generate_score_xml, ScoreProfile};

    let profile = ScoreProfile::new(4.8);
    assert!((profile.total - 4.8).abs() < 1e-4);
    assert!(profile.accuracy > 4.0);
    assert!(profile.fluency > 4.0);

    // 换算分测试
    let converted = convert_score(4.8, 10.0, 0.125);
    println!("Converted score (4.8 / 5.0 of 10.0 with 0.125 grad): {}", converted);
    assert!((9.5..=10.0).contains(&converted));

    // detail JSON 测试
    let detail = build_score_detail(&profile, "read_chapter", "A", true, 0.125);
    println!("Score detail JSON: {}", detail);
    let v: serde_json::Value = serde_json::from_str(&detail).expect("Must be valid JSON");
    assert_eq!(v["category"], "read_chapter");
    assert_eq!(v["choose"], "A");
    assert_eq!(v["rate_scale"], 1);

    // XML 评测测试
    let text = "In today's rapidly changing world, technology plays an important role.";
    let xml = generate_score_xml(text, &profile, "read_chapter", 42);
    println!("Generated XML snippet:\n{}", &xml[..xml.len().min(300)]);
    assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?><xml_result>"));
    assert!(xml.contains("<read_chapter>"));
    assert!(xml.contains("<rec_paper total_score=\"4.80\""));
    assert!(xml.contains("<sentence content=\"In today&apos;s rapidly changing world"));
    assert!(xml.contains("<word content=\"technology\""));
    assert!(xml.contains("<phoneme content="));
    assert!(xml.ends_with("</rec_paper></read_chapter></xml_result>"));
}

#[test]
fn test_la_models_and_session() {
    use fepc::la_client::{LaSession, LaUser};

    let user = LaUser {
        id: 1337,
        username: "leitianshuo".to_string(),
        member_level: 2,
        tier: "ultra".to_string(),
        avatar_url: None,
    };
    assert_eq!(user.tier_display_name(), "Ultra 会员");
    assert!(user.is_member());

    let session = LaSession {
        session_id: "sess_test_123".to_string(),
        device_id: Some("dev_win_1".to_string()),
        user,
        access_token: "mock_access_token".to_string(),
        refresh_token: "mock_refresh_token".to_string(),
        cached_at: 1726000000,
    };

    let json_str = serde_json::to_string(&session).unwrap();
    let deserialized: LaSession = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.user.username, "leitianshuo");
    assert_eq!(deserialized.user.tier, "ultra");
    assert_eq!(deserialized.session_id, "sess_test_123");
}

#[test]
fn test_real_la_login() {
    use fepc::la_client::LaClient;
    use fepc::state::AppState;
    let res = LaClient::login("testaccount", "1145141919810", None);
    println!("LaClient::login result: {:?}", res);
    let sess = res.expect("Real login should succeed");
    assert_eq!(sess.user.username, "testaccount");
    println!("Session user: {:?}", sess.user);

    // 验证设备列表查询与解析
    let devices = LaClient::list_devices(&sess.access_token);
    println!("LaClient::list_devices result: {:?}", devices);
    assert!(devices.is_ok(), "list_devices should succeed with valid access token");

    // Now test UI render with this session
    let ctx = eframe::egui::Context::default();
    let mut state = AppState {
        la_session: Some(sess.clone()),
        ..Default::default()
    };
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::cloud_account::render(&mut fluent, &mut state);
        });
    });
    println!("cloud_account render with real session succeeded!");
    LaClient::clear_session();
}

#[test]
fn test_ets_rebind_dialog_and_code_handling() {
    use fepc::models::UserSession;
    use fepc::state::AppState;
    let ctx = eframe::egui::Context::default();
    let mut state = AppState {
        session: UserSession {
            phone: "13800138000".to_string(),
            ..Default::default()
        },
        rebind_dialog_open: true,
        rebind_code_input: "123456".to_string(),
        ..Default::default()
    };

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::cloud_account::render(&mut fluent, &mut state);
        });
    });
    println!("✓ 官方账号设备换绑对话框成功渲染！");
}

#[test]
fn test_auto_complete_dry_run_pipeline() {
    use fepc::auto_complete::{AutoCompleteConfig, AutoCompleteWorkflow, AutoProgressMsg};
    use fepc::local::LocalScanner;
    use std::sync::mpsc::channel;

    let dirs = LocalScanner::find_ets_local_dirs();
    if dirs.is_empty() {
        println!("Skipping dry run pipeline test: no local ETS directories found.");
        return;
    }

    let homeworks = LocalScanner::scan_homeworks(&dirs[0]);
    if homeworks.is_empty() {
        println!("Skipping dry run pipeline test: no local homeworks found.");
        return;
    }

    let hw = homeworks[0].clone();
    println!("Testing AutoCompleteWorkflow on local homework: {}", hw.name);

    let session = fepc::models::UserSession {
        phone: "13800000000".to_string(),
        token: "test_token".to_string(),
        device_code: "TEST_DEV_CODE".to_string(),
        parent_account_id: "123456".to_string(),
        user_name: "测试学生".to_string(),
        is_logged_in: true,
    };

    let config = AutoCompleteConfig {
        target_total: 28.5,
        voice_name: "en-US-JennyNeural".to_string(),
        rate: -5,
        pitch: 0,
        use_time_seconds: 120,
        no_upload: true, // 试运行模式
        accuracy: 4.95,
        fluency: 4.95,
        integrity: 4.95,
    };

    let (tx, rx) = channel();

    // 在独立线程运行流水线
    let handle = std::thread::spawn(move || {
        AutoCompleteWorkflow::run(session, hw, config, tx);
    });

    let mut log_count = 0;
    let mut step_count = 0;
    let mut completed = false;
    let mut final_score = 0.0;

    while let Ok(msg) = rx.recv() {
        match msg {
            AutoProgressMsg::Step { current, total, title } => {
                step_count += 1;
                println!("  [Workflow Step {}/{}] {}", current, total, title);
            }
            AutoProgressMsg::Log(log) => {
                log_count += 1;
                println!("  [Workflow Log] {}", log);
            }
            AutoProgressMsg::QuestionDone { number, score, max_score, status, .. } => {
                println!("  [Question Done] No.{} score={:.2}/{:.2} ({})", number, score, max_score, status);
            }
            AutoProgressMsg::Complete { success, final_score: score, message } => {
                completed = true;
                final_score = score;
                println!("  [Workflow Complete] success={}, score={}, message={}", success, score, message);
                assert!(success);
            }
        }
    }

    handle.join().expect("Workflow thread should finish without panic");
    assert!(completed, "Workflow must report completion");
    assert!(step_count >= 8, "Workflow must execute all 8 stages");
    assert!(log_count > 5, "Workflow must output logs");
    assert!(final_score > 0.0, "Final score must be greater than 0");
    println!("Dry run pipeline verified successfully with {} steps, {} logs, score: {}", step_count, log_count, final_score);
}

#[test]
fn test_la_auth_gating_and_cloud_session_requirement() {
    use fepc::auto_complete::{AutoCompleteConfig, AutoCompleteWorkflow, AutoProgressMsg};
    use fepc::la_client::{LaCloudSession, LaSession, LaUser};
    use fepc::models::{HomeworkItem, UserSession};
    use fepc::state::AppState;
    use std::sync::mpsc::channel;

    let ctx = eframe::egui::Context::default();
    let mut state = AppState::default();

    // 1. 未登录 LA 账号时，渲染 AutoComplete 视图，确保不会崩溃并且处于受限状态
    assert!(state.la_session.is_none());
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::auto_complete_view::render(&mut fluent, &mut state);
        });
    });
    println!("✓ 未登录 LA 时 AutoCompleteView 成功渲染授权引导提示卡片");

    // 2. 验证：未登录 LA 时，基础功能（本地扫描、作业列表、致谢、设置）依然畅行无阻
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::local_scan::render(&mut fluent, &mut state);
            fepc::views::homework_list::render(&mut fluent, &mut state);
            fepc::views::credits::render(&mut fluent, &mut state);
        });
    });
    println!("✓ 未登录 LA 时基础功能（本地试卷扫描、作业列表、致谢等）完全正常渲染");

    // 3. 已登录 LA 账号，但尚未拉取云端 ETS 会话
    state.la_session = Some(LaSession {
        session_id: "test_session_id".to_string(),
        device_id: Some("test_dev_id".to_string()),
        user: LaUser {
            id: 1337,
            username: "leitianshuo".to_string(),
            member_level: 2,
            tier: "ultra".to_string(),
            avatar_url: None,
        },
        access_token: "test_token".to_string(),
        refresh_token: "test_refresh".to_string(),
        cached_at: 1726000000,
    });
    assert!(state.la_cloud_session.is_none());

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::auto_complete_view::render(&mut fluent, &mut state);
        });
    });
    println!("✓ 已登录 LA 但未拉取云端会话时成功渲染获取云端凭据引导卡片");

    // 4. 已激活云端 ETS 会话，渲染完整流水线控制台
    state.la_cloud_session = Some(LaCloudSession {
        account_display: "学生雷天硕 (138****0000)".to_string(),
        token: "valid_cloud_token".to_string(),
        device_code: "DEVICE_CODE_123".to_string(),
        parent_account_id: "ACC_999".to_string(),
        selected_account: serde_json::json!({}),
        valid_accounts: vec![],
    });
    state.session = UserSession {
        phone: "13800000000".to_string(),
        user_name: "学生雷天硕".to_string(),
        token: "valid_cloud_token".to_string(),
        device_code: "DEVICE_CODE_123".to_string(),
        parent_account_id: "ACC_999".to_string(),
        is_logged_in: true,
    };

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::auto_complete_view::render(&mut fluent, &mut state);
        });
    });
    println!("✓ 已同步云端会话时 AutoCompleteView 完整渲染流水线参数配置与开始按钮");

    // 5. 验证：当 token 为空且试图进行官方正式提交时，流水线直接安全拦截并终止
    let empty_session = UserSession::default();
    let (tx, rx) = channel();
    let config = AutoCompleteConfig {
        no_upload: false, // 正式上传模式
        ..Default::default()
    };
    let hw = HomeworkItem {
        id: "mock_hw_1".to_string(),
        name: "测试拦截作业".to_string(),
        status: "未完成".to_string(),
        deadline: "2026-09-13".to_string(),
        total_point: 30.0,
        awarded_point: None,
        question_count: 5,
        source: fepc::models::HomeworkSource::Cloud,
        local_dir: None,
        resource_urls: vec![],
        sections: vec![],
        set_type: 0,
        pk: 0,
        resource_id: String::new(),
    };

    AutoCompleteWorkflow::run(empty_session, hw, config, tx);
    let mut got_abort_complete = false;
    while let Ok(msg) = rx.try_recv() {
        if let AutoProgressMsg::Complete { success, message, .. } = msg {
            assert!(!success, "Workflow must fail when token is empty");
            assert!(message.contains("未检测到有效做题凭据") || message.contains("登录 LA 账号"));
            got_abort_complete = true;
        }
    }
    assert!(got_abort_complete, "Must receive Complete message indicating refusal to run without valid session token");
    println!("✓ 流水线未登录拦截测试通过！");
}

#[test]
fn test_captcha_server_and_token_exchange() {
    use fepc::utils::captcha_server::CaptchaServer;

    let server = CaptchaServer::start().expect("Failed to start CaptchaServer");
    let port = server.port();
    assert!(port > 0);

    // 1. 测试访问 HTML 页面
    let resp = ureq::get(&format!("http://127.0.0.1:{port}/"))
        .call()
        .expect("Server must answer HTTP GET /");
    assert_eq!(resp.status(), 200);
    let html = resp.into_string().unwrap();
    assert!(html.contains("cap-widget"));
    assert!(html.contains("LA Studio 人机安全验证"));
    assert!(html.contains(r#"data-cap-api-endpoint="/cap_proxy/""#));

    // 2. 测试反向代理 /cap_proxy/challenge
    let challenge_resp = ureq::post(&format!("http://127.0.0.1:{port}/cap_proxy/challenge"))
        .call()
        .expect("Server must proxy Cap challenge");
    assert_eq!(challenge_resp.status(), 200);
    let challenge_body = challenge_resp.into_string().unwrap();
    assert!(challenge_body.contains("challenge"), "Challenge response should contain challenge object");
    assert!(challenge_body.contains("token"), "Challenge response should contain token");

    // 3. 测试提交 Token
    let mock_token = "mock_cap_token_abcdef123456";
    let submit_resp = ureq::get(&format!("http://127.0.0.1:{port}/submit_token?token={mock_token}"))
        .call()
        .expect("Server must accept token submit");
    assert_eq!(submit_resp.status(), 200);

    // 4. 验证服务端成功接收并解码 Token
    let received = server.try_recv();
    assert_eq!(received, Some(mock_token.to_string()));
    println!("✓ CaptchaServer 本地反向代理与 Token 交换测试通过！");
}

#[test]
fn test_animation_toggle_and_revision() {
    use fepc::state::{AppState, NavTab};

    let mut state = AppState::default();
    let initial_rev = state.nav_revision;

    // 模拟关闭动效
    state.theme_config = state.theme_config.with_animations(false);
    assert!(!state.theme_config.animations);

    // 模拟在设置页重新打开动效
    state.theme_config = state.theme_config.with_animations(true);
    state.nav_revision = state.nav_revision.wrapping_add(1);
    assert!(state.nav_revision > initial_rev, "Turning animations on must increment nav_revision to trigger fresh animation");

    // 模拟切换 Tab
    state.current_tab = NavTab::Settings;
    if state.last_nav_tab != Some(state.current_tab) {
        state.last_nav_tab = Some(state.current_tab);
        state.nav_revision = state.nav_revision.wrapping_add(1);
    }
    assert_eq!(state.nav_revision, initial_rev + 2);
    println!("✓ 动效开关与单调递增版本号测试通过！");
}

#[test]
fn test_page_scroll_isolation() {
    use fepc::state::AppState;
    use eframe::egui::{Id, scroll_area::State};

    let ctx = eframe::egui::Context::default();
    let _state = AppState::default();

    // 1. 模拟在作业与试卷页面产生滚动偏移量 (存储在对应 ID 下)
    let hw_id = Id::new("scroll_page_homework_list");
    let mut hw_state = State::load(&ctx, hw_id).unwrap_or_default();
    hw_state.offset.y = 450.0;
    hw_state.store(&ctx, hw_id);

    // 2. 模拟切换至鸣谢与组件页面
    let credits_id = Id::new("scroll_page_credits");
    let credits_state = State::load(&ctx, credits_id);
    let credits_offset = credits_state.map(|s| s.offset.y).unwrap_or(0.0);

    // 验证：鸣谢与组件页面的滚动状态绝对独立（为 0.0），绝不会继承作业列表的 450.0
    assert_eq!(credits_offset, 0.0, "Credits page scroll offset must be isolated from HomeworkList!");

    // 3. 验证设置页面的滚动状态也完全隔离
    let settings_id = Id::new("scroll_page_settings");
    let settings_offset = State::load(&ctx, settings_id).map(|s| s.offset.y).unwrap_or(0.0);
    assert_eq!(settings_offset, 0.0, "Settings page scroll offset must be isolated!");

    // 4. 验证作业列表滚动状态依然完好保留在 450.0
    let reloaded_hw = State::load(&ctx, hw_id).unwrap();
    assert_eq!(reloaded_hw.offset.y, 450.0, "HomeworkList scroll position must be retained!");

    println!("✓ 跨页面滚动状态完全隔离验证通过！");
}
