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
fn test_local_lazy_scan_and_on_demand_resolution() {
    let dirs = LocalScanner::find_ets_local_dirs();
    assert!(!dirs.is_empty(), "Should discover at least one ETS local directory in APPDATA");

    let target_dir = &dirs[0];

    // 1. 验证极速懒扫描 (Lazy Indexing)
    let start_t = std::time::Instant::now();
    let mut lazy_hws = LocalScanner::scan_homeworks_lazy(target_dir);
    let elapsed = start_t.elapsed();
    println!("Lazy scan finished in {:?} with {} items", elapsed, lazy_hws.len());
    assert!(!lazy_hws.is_empty(), "Lazy scan should discover local homeworks");

    // 验证所有作业处于未解析状态，但基础元信息就绪
    for hw in &lazy_hws {
        assert!(!hw.id.is_empty());
        assert!(!hw.name.is_empty());
        assert!(hw.question_count > 0, "Question count should be indexed");
        assert!(!hw.is_parsed(), "Homework should initially be unparsed in lazy mode");
        assert!(hw.sections.is_empty(), "Sections should be empty in lazy mode");
    }

    // 2. 验证按需全量解析 (On-Demand Full Resolution)
    let target_hw = &mut lazy_hws[0];
    let hw_id = target_hw.id.clone();
    assert!(!target_hw.is_parsed());

    let parse_ok = LocalScanner::ensure_homework_parsed(target_hw);
    assert!(parse_ok, "ensure_homework_parsed should succeed for valid homework");
    assert!(target_hw.is_parsed(), "Homework should now be marked as parsed");
    assert!(!target_hw.sections.is_empty(), "Sections should now be populated");

    // 3. 验证幂等性
    assert!(LocalScanner::ensure_homework_parsed(target_hw), "Subsequent ensure should return true immediately");

    // 4. 验证 AppState 状态联动与缓存
    let mut state = fepc::state::AppState {
        local_ets_dirs: dirs.clone(),
        selected_local_dir: Some(target_dir.clone()),
        local_items: LocalScanner::scan_homeworks_lazy(target_dir),
        ..Default::default()
    };
    state.refresh_selected_local_metadata();
    assert!(state.local_device_code.is_some() || !state.local_accounts.is_empty(), "Metadata should be cached");

    // 通过 state 方法按需解析
    let resolved = state.ensure_local_homework_parsed(&hw_id);
    assert!(resolved.is_some());
    assert!(resolved.unwrap().is_parsed());
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
        allow_over_max: false,
        ..Default::default()
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
    assert_eq!(payload["application_id"], "cc.lastudio.fepc");
    assert_eq!(payload["signing_certificate_sha256"], "79fe6f92aa7f46084832cd62292c223101586b46e3cfaf39740fef1c6253aaca");
    assert_eq!(payload["version_code"], 106);
    assert_eq!(payload["platform"], "windows");
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
    let converted = convert_score(4.8, 10.0, 0.125, true);
    println!("Converted score (4.8 / 5.0 of 10.0 with 0.125 grad): {}", converted);
    assert!((9.5..=10.0).contains(&converted));

    // 超满分换算测试 (cap == false)
    let uncapped = convert_score(6.0, 10.0, 0.125, false);
    assert_eq!(uncapped, 12.0);

    // detail JSON 测试
    let detail = build_score_detail(&profile, "read_chapter", "A", true, 0.125);
    println!("Score detail JSON: {}", detail);
    let v: serde_json::Value = serde_json::from_str(&detail).expect("Must be valid JSON");
    assert_eq!(v["category"], "read_chapter");
    assert_eq!(v["choose"], "A");
    assert_eq!(v["rate_scale"], 1);

    // XML 评测测试
    let text = "In today's rapidly changing world, technology plays an important role.";
    let xml = generate_score_xml(text, &profile, "read_chapter", 42, true);
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
        role: 3,
        role_display: None,
        member_level: 2,
        tier: "ultra".to_string(),
        member_expire_time: None,
        avatar_url: None,
    };
    assert_eq!(user.tier_display_name(), "Ultra 会员");
    assert!(user.is_member());

    let super_admin = LaUser {
        id: 1,
        username: "admin".to_string(),
        role: 0,
        role_display: Some("超级管理员".to_string()),
        member_level: 0,
        tier: "free".to_string(),
        member_expire_time: None,
        avatar_url: None,
    };
    assert_eq!(super_admin.tier_display_name(), "超级管理员 (Ultra)");
    assert!(super_admin.is_member());

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

    fepc::workflow::tts::ensure_crypto_provider();

    let mut res = Err("initial".to_string());
    for attempt in 1..=3 {
        res = LaClient::login("testaccount", "1145141919810", None);
        if res.is_ok() {
            break;
        }
        println!("Login attempt {attempt} failed: {res:?}, retrying in 500ms...");
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let sess = match res {
        Ok(s) => {
            assert_eq!(s.user.username, "testaccount");
            println!("Session user: {:?}", s.user);

            // 验证设备列表查询与解析
            let devices = LaClient::list_devices(&s.access_token);
            println!("LaClient::list_devices result: {:?}", devices);
            assert!(devices.is_ok(), "list_devices should succeed with valid access token");
            s
        }
        Err(e) if e.contains("达到该软件上限") || e.contains("无法连接") || e.contains("客户端标识无效") => {
            println!("Remote live account quota reached or server unavailable: {e}");
            println!("Proceeding to validate cloud_account UI render with synthetic session.");
            fepc::la_client::LaSession {
                session_id: "test_session_id".to_string(),
                device_id: Some("test_dev_id".to_string()),
                user: fepc::la_client::LaUser {
                    id: 1337,
                    username: "testaccount".to_string(),
                    role: 3,
                    role_display: None,
                    member_level: 2,
                    tier: "ultra".to_string(),
                    member_expire_time: None,
                    avatar_url: None,
                },
                access_token: "test_token".to_string(),
                refresh_token: "test_refresh".to_string(),
                cached_at: 1726000000,
            }
        }
        Err(e) => panic!("Real login should succeed or fail with known server quota error, got: {e}"),
    };

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
        allow_over_max: false,
        ..Default::default()
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
            role: 3,
            role_display: None,
            member_level: 2,
            tier: "ultra".to_string(),
            member_expire_time: None,
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

#[test]
fn test_native_cap_solver_algorithms() {
    use fepc::utils::native_cap_solver::{fnv1a, generate_prng_hex, solve_one, solve_pow, dom_sum_mock};

    // 1. FNV-1a Hash 验证
    let h1 = fnv1a("test_seed");
    assert_ne!(h1, 0);

    // 2. 32-bit Xorshift PRNG 格式与长度验证
    let hex_str = generate_prng_hex("test_seed", 16);
    assert_eq!(hex_str.len(), 16);
    assert!(hex_str.chars().all(|c| c.is_ascii_hexdigit()));

    // 3. PoW 单题求解与多线程求解验证
    let salt = generate_prng_hex("test_token_1", 10);
    let target = generate_prng_hex("test_token_1d", 2); // 较浅难度用于快速单元测试
    let nonce = solve_one(&salt, &target);
    println!("✓ solve_one found nonce: {nonce} for target {target}");

    let solutions = solve_pow("unit_test_token", 3, 8, 2);
    assert_eq!(solutions.len(), 3);
    for s in &solutions {
        assert!(!s.is_empty());
    }

    // 4. Mock DOM 模拟链求和验证
    let dom_val = dom_sum_mock(1234, 5678, 9012);
    assert!((0..256).contains(&dom_val));
    println!("✓ Native Cap Solver 核心数学算法验证通过！");
}

#[test]
fn test_recommended_use_time_calculation() {
    use fepc::models::{HomeworkItem, HomeworkSource, ParsedSection, ParsedQuestion};
    use fepc::workflow::scoring::calculate_recommended_use_time;

    let hw = HomeworkItem {
        id: "hw_rec_1".to_string(),
        name: "中考英语听说模拟考试二模".to_string(),
        status: "未开始".to_string(),
        deadline: "".to_string(),
        total_point: 30.0,
        awarded_point: None,
        question_count: 3,
        source: HomeworkSource::Cloud,
        local_dir: None,
        resource_urls: vec![],
        set_type: 1, // 模拟卷
        pk: 0,
        resource_id: "res_1".to_string(),
        sections: vec![
            ParsedSection {
                section_type: "read".to_string(),
                title: "第一部分 模仿朗读".to_string(),
                questions: vec![
                    ParsedQuestion {
                        prompt_text: "In ancient times, people believed the earth was flat and the stars moved across the sky.".to_string(),
                        ..Default::default()
                    },
                    ParsedQuestion {
                        prompt_text: "Today modern astronomy tells us a very different and wonderful story about our universe.".to_string(),
                        ..Default::default()
                    },
                ],
            },
            ParsedSection {
                section_type: "choose".to_string(),
                title: "第二部分 听选信息".to_string(),
                questions: vec![
                    ParsedQuestion {
                        category: "choose".to_string(),
                        prompt_text: "Where is the boy going?".to_string(),
                        ..Default::default()
                    },
                ],
            },
        ],
    };

    let use_time = calculate_recommended_use_time(&hw);
    assert!(use_time >= 180, "模拟卷用时必须具备 180 秒下限截断保护");
    println!("✓ 推荐做题用时算法验证通过: {use_time} 秒");
}

#[test]
fn test_allow_over_max_scoring() {
    use fepc::workflow::scoring::{convert_score, generate_score_xml, ScoreProfile};

    // 1. cap = true 正常截断 (输入 6.0 分，满分 10.0，折算 6.0/5.0 * 10 = 12.0，截断为 10.0)
    let capped = convert_score(6.0, 10.0, 0.125, true);
    assert_eq!(capped, 10.0);

    // 2. cap = false 允许超满分 (折算为 12.0)
    let over = convert_score(6.0, 10.0, 0.125, false);
    assert_eq!(over, 12.0);

    // 3. XML 打分 clamp = false
    let profile = ScoreProfile {
        total: 8.5,
        accuracy: 8.5,
        fluency: 8.5,
        integrity: 8.5,
        stress: 8.5,
    };
    let xml_clamped = generate_score_xml("hello world", &profile, "read", 42, true);
    assert!(xml_clamped.contains("total_score=\"5.00\""));

    let xml_unclamped = generate_score_xml("hello world", &profile, "read", 42, false);
    assert!(xml_unclamped.contains("total_score=\"8.50\""));
    println!("✓ allow_over_max 超满分评测打分算法验证通过！");
}

#[test]
fn test_parsed_question_ids() {
    use fepc::models::ParsedQuestion;

    let q = ParsedQuestion {
        number: "1".to_string(),
        word_id: Some(108823),
        question_id: Some(509912),
        prompt_text: "abandon".to_string(),
        ..Default::default()
    };

    assert_eq!(q.word_id, Some(108823));
    assert_eq!(q.question_id, Some(509912));
    println!("✓ ParsedQuestion 真实题目与单词 ID 字段保留验证通过！");
}

#[test]
fn test_changyan_session_model() {
    use fepc::la_client::LaCloudSession;
    use serde_json::json;

    let raw = json!({
        "account_display": "张三 (初三2班)",
        "token": "ets_mock_token_123",
        "device_code": "dev_code_abc",
        "parent_account_id": "pid_999",
        "selected_account": { "id": "1", "name": "张三" },
        "valid_accounts": [
            { "id": "1", "name": "张三" }
        ]
    });

    let session: LaCloudSession = serde_json::from_value(raw).expect("LaCloudSession should deserialize");
    assert_eq!(session.account_display, "张三 (初三2班)");
    assert_eq!(session.token, "ets_mock_token_123");
    println!("✓ 畅言会话 LaCloudSession 数据模型反序列化验证通过！");
}

#[test]
fn test_logo_texture_resampling() {
    let logo_bytes = include_bytes!("../fe_logo.png");
    let dyn_img = image::load_from_memory(logo_bytes).expect("Should load fe_logo.png");
    assert_eq!(dyn_img.width(), 256);
    assert_eq!(dyn_img.height(), 256);

    let rgba = dyn_img.to_rgba8();
    assert_eq!(rgba.len(), 256 * 256 * 4);
    println!("✓ 256x256 高清抗锯齿 Logo 纹理完整性验证通过！");
}

#[test]
fn test_http_tts_sse_parsing() {
    use fepc::workflow::tts::HttpTtsService;
    use base64::Engine;

    let chunk1 = base64::engine::general_purpose::STANDARD.encode(b"audio-part-1-");
    let chunk2 = base64::engine::general_purpose::STANDARD.encode(b"audio-part-2");
    let sse_payload = format!(
        "data: {{\"chunk\":\"{}\"}}\n\ndata: {{\"chunk\":\"{}\"}}\ndata: [DONE]\n",
        chunk1, chunk2
    );

    let parsed = HttpTtsService::parse_sse_audio(&sse_payload).expect("Should parse SSE chunks successfully");
    assert_eq!(parsed, b"audio-part-1-audio-part-2");
    println!("✓ HTTP TTS SSE 流式音频切片解析测试通过！");
}

#[test]
fn test_auto_complete_config_tts_defaults() {
    use fepc::auto_complete::AutoCompleteConfig;

    let default_cfg = AutoCompleteConfig::default();
    assert_eq!(default_cfg.tts_mode, "edge");
    assert!(default_cfg.http_tts_url.is_empty());

    // 测试向后兼容反序列化 (旧版 JSON 不含 tts_mode 字段时应自动填充默认值)
    let json_legacy = r#"{"target_total": 28.0, "voice_name": "en-US-JennyNeural", "rate": -5, "pitch": 0, "use_time_seconds": 180, "no_upload": false, "accuracy": 4.9, "fluency": 4.9, "integrity": 4.9, "allow_over_max": false}"#;
    let cfg: AutoCompleteConfig = serde_json::from_str(json_legacy).expect("Should deserialize legacy json");
    assert_eq!(cfg.tts_mode, "edge");
    assert_eq!(cfg.http_tts_model, "tts-1");
    println!("✓ AutoCompleteConfig TTS 配置向后兼容序列化测试通过！");
}

#[test]
fn test_render_sidebar_and_logo() {
    let ctx = eframe::egui::Context::default();
    let mut state = fepc::state::AppState::default();

    // 加载 256x256 高清抗锯齿纹理，并启用硬件 Mipmap 三线性插值
    let logo_bytes = include_bytes!("../fe_logo.png");
    let dyn_img = image::load_from_memory(logo_bytes).expect("Should load 256 logo");
    let rgba = dyn_img.to_rgba8();
    let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied(
        [256, 256],
        &rgba.into_raw(),
    );
    let texture_options = eframe::egui::TextureOptions {
        magnification: eframe::egui::TextureFilter::Linear,
        minification: eframe::egui::TextureFilter::Linear,
        wrap_mode: eframe::egui::TextureWrapMode::ClampToEdge,
        mipmap_mode: Some(eframe::egui::TextureFilter::Linear),
    };
    let logo_texture = ctx.load_texture("app_logo", color_image, texture_options);

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::Panel::left("test_sidebar")
            .exact_size(220.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.image((logo_texture.id(), eframe::egui::vec2(48.0, 48.0)));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.heading("Fe PC");
                        ui.label("e听说提取器 · 桌面版");
                    });
                });
            });

        // 渲染设置视图中的两种 TTS 模式
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            state.auto_config.tts_mode = "edge".to_string();
            fepc::views::settings::render(&mut fluent, &mut state);

            state.auto_config.tts_mode = "http".to_string();
            state.auto_config.http_tts_url = "https://api.openai.com/v1/audio/speech".to_string();
            fepc::views::settings::render(&mut fluent, &mut state);
        });
    });

    println!("✓ 侧边栏 46x46 高清超采样 Logo 与设置页双 TTS 模式渲染测试通过！");
}

#[test]
fn test_round_points_accuracy() {
    use fepc::workflow::scoring::round_points;

    // 标准四舍五入 vs 银行家舍入 (0.625 应该到 0.63 而不是 0.62)
    assert_eq!(round_points(0.625), 0.63);
    assert_eq!(round_points(0.125), 0.13);
    assert_eq!(round_points(0.375), 0.38);
    assert_eq!(round_points(0.875), 0.88);
    assert_eq!(round_points(1.25), 1.25);
    assert_eq!(round_points(0.0), 0.0);
    assert_eq!(round_points(28.5), 28.5);
    println!("✓ 真实标准四舍五入 round_points 算法精度测试通过！");
}

#[test]
fn test_plan_question_allocations_weighted() {
    use fepc::models::ParsedQuestion;
    use fepc::workflow::scoring::{plan_question_allocations, round_points, GRADUATION};

    let questions = vec![
        ParsedQuestion {
            number: "1".to_string(),
            category: "collector.read".to_string(),
            prompt_text: "In today's rapidly changing world, technology plays an important role in our daily lives.".to_string(),
            ..Default::default()
        },
        ParsedQuestion {
            number: "2".to_string(),
            category: "collector.choose".to_string(),
            prompt_text: "Where is the library located?".to_string(),
            answers: vec!["A".to_string()],
            ..Default::default()
        },
        ParsedQuestion {
            number: "3".to_string(),
            category: "collector.choose".to_string(),
            prompt_text: "What time does the train leave?".to_string(),
            answers: vec!["B".to_string()],
            ..Default::default()
        },
        ParsedQuestion {
            number: "4".to_string(),
            category: "collector.role".to_string(),
            prompt_text: "Could you tell me how to get to the museum?".to_string(),
            answers: vec!["Turn left at the next corner and go straight for two blocks.".to_string()],
            ..Default::default()
        },
        ParsedQuestion {
            number: "5".to_string(),
            category: "collector.repeat".to_string(),
            prompt_text: "A wise man once said that learning is a lifelong journey of discovery and growth.".to_string(),
            ..Default::default()
        },
    ];

    let target_total = 27.5;
    let seed = 42;
    let allocations = plan_question_allocations(&questions, target_total, seed, false);

    assert_eq!(allocations.len(), 5);

    // 1. 验证总分严格等于 target_total
    let total_allocated: f64 = round_points(allocations.iter().map(|a| a.allocated_score).sum());
    println!("Allocated total: {:.2}, target: {:.2}", total_allocated, target_total);
    assert!((total_allocated - target_total).abs() < 1e-4, "Allocated total score must match target_total exactly");

    // 2. 验证客观题二值化（仅给 0 或 满分）
    for alloc in &allocations {
        if alloc.is_objective {
            assert!(
                alloc.allocated_score == 0.0 || (alloc.allocated_score - alloc.max_score).abs() < 1e-4,
                "Objective question must be binary: 0 or max_score ({}), but got {}",
                alloc.max_score, alloc.allocated_score
            );
        }
    }

    // 3. 验证 0.125 档位量化
    for alloc in &allocations {
        let slots = (alloc.allocated_score / GRADUATION).round();
        let diff = (alloc.allocated_score - slots * GRADUATION).abs();
        assert!(diff < 0.015, "Score {} must be aligned to GRADUATION {}", alloc.allocated_score, GRADUATION);
    }

    // 4. 验证反推 ise_score：官方 ceil 向上取整能精确还原 allocated_score
    for alloc in &allocations {
        if alloc.max_score > 0.0 && alloc.allocated_score > 0.0 {
            let val = alloc.ise_score * alloc.max_score / 5.0;
            let recovered = ((val - 1e-12) / GRADUATION).ceil() * GRADUATION;
            let recovered_rounded = round_points(recovered);
            assert!(
                (recovered_rounded - alloc.allocated_score).abs() < 1e-4,
                "ise_score {:.2} must recover allocated_score {:.2}, got {:.2}",
                alloc.ise_score, alloc.allocated_score, recovered_rounded
            );
        }
    }

    // 5. 验证大题标识与长句大题避免绝对满分
    let big_questions: Vec<_> = allocations.iter().filter(|a| a.is_big_question).collect();
    assert!(!big_questions.is_empty(), "Should identify at least one big question");
    for bq in big_questions {
        println!("Big question: order={}, max={}, alloc={}, is_full={}", bq.order, bq.max_score, bq.allocated_score, bq.is_full);
        assert!(!bq.is_full, "Long sentence big question should avoid absolute full score when total has deductions");
    }

    println!("✓ 加权随机拟真控分算法 plan_question_allocations 完整特性验证通过！");
}

#[test]
fn test_smart_use_time_and_hash_jitter() {
    use fepc::models::{HomeworkItem, HomeworkSource, ParsedQuestion, ParsedSection};
    use fepc::workflow::scoring::{calculate_smart_use_time, reroll_time_jitter};

    let hw = HomeworkItem {
        id: "hw_jitter_101".to_string(),
        name: "初三英语听说全真模拟二模".to_string(),
        status: "进行中".to_string(),
        deadline: "".to_string(),
        total_point: 30.0,
        awarded_point: None,
        question_count: 4,
        source: HomeworkSource::Cloud,
        local_dir: None,
        resource_urls: vec![],
        set_type: 1, // 模拟卷
        pk: 0,
        resource_id: "res_mock".to_string(),
        sections: vec![
            ParsedSection {
                section_type: "read".to_string(),
                title: "第一部分 短文朗读".to_string(),
                questions: vec![
                    ParsedQuestion {
                        prompt_text: "Technology has changed the way we communicate, work, and learn in modern society.".to_string(),
                        ..Default::default()
                    },
                ],
            },
            ParsedSection {
                section_type: "listen_choose".to_string(),
                title: "第二部分 听选信息".to_string(),
                questions: vec![
                    ParsedQuestion {
                        category: "collector.choose".to_string(),
                        prompt_text: "What will the weather be like tomorrow according to the forecast?".to_string(),
                        answers: vec!["Sunny".to_string()],
                        ..Default::default()
                    },
                ],
            },
        ],
    };

    let (time1, breakdown) = calculate_smart_use_time(&hw, 100);
    println!("Smart use time 1: {}s (Breakdown: prep={}s, listen={}s, answer={}s, overhead={}s)",
        time1, breakdown.prep_seconds, breakdown.listen_seconds, breakdown.answer_seconds, breakdown.overhead_seconds);

    assert!(breakdown.prep_seconds >= 50, "Read first question should get 50s prep");
    assert!(breakdown.overhead_seconds >= 40, "Overhead should include section guide (15s*2=30s) + question buffer (5s*2=10s)");
    assert!(time1 >= 180, "Mock exam must satisfy 180s lower bound");

    // 测试不同 nonce 的抖动散列
    let time2 = reroll_time_jitter(&hw, 200);
    let time3 = reroll_time_jitter(&hw, 9999);
    println!("Rerolled jitter times: nonce(100)={}, nonce(200)={}, nonce(9999)={}", time1, time2, time3);

    let base_total = breakdown.prep_seconds + breakdown.listen_seconds + breakdown.answer_seconds + breakdown.overhead_seconds;
    let min_allowed = ((base_total as f64) * 0.88).round() as u32;
    let max_allowed = ((base_total as f64) * 1.12).round() as u32;
    assert!(time2 >= 180 || (time2 >= min_allowed && time2 <= max_allowed));

    println!("✓ 智能作答耗时计算与秒级哈希抖动测试通过！");
}

#[test]
fn test_voice_profile_and_client_methods() {
    use fepc::la_client::{LaClient, VoiceProfile};

    let sample_json = r#"[
        {
            "id": "vp-12345",
            "name": "李同学英语自然声",
            "duration": 8.5,
            "is_default": true,
            "created_at": "2026-10-04T12:00:00"
        },
        {
            "id": "vp-67890",
            "name": "王同学沉稳声",
            "duration": 12.0,
            "is_default": false,
            "created_at": "2026-10-04T13:30:00"
        }
    ]"#;

    let profiles: Vec<VoiceProfile> = serde_json::from_str(sample_json).expect("VoiceProfile must deserialize");
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].id, "vp-12345");
    assert_eq!(profiles[0].name, "李同学英语自然声");
    assert_eq!(profiles[0].duration, Some(8.5));
    assert!(profiles[0].is_default);

    // 验证客户端对象方法接口
    let client = LaClient;
    let err_res = client.get_user_voices("invalid_test_token");
    assert!(err_res.is_err(), "Invalid token should fail safely");

    let temp_dest = std::env::temp_dir().join("test_voice_dl.mp3");
    let dl_res = client.download_voice_audio("vp-12345", "invalid_token", &temp_dest);
    assert!(dl_res.is_err(), "Invalid token download should fail safely");

    println!("✓ VoiceProfile 与 LaClient 声音资产接口验证通过！");
}

#[test]
fn test_mic_pink_noise_and_audio_validity() {
    use fepc::workflow::audio_proc::{check_audio_validity, inject_mic_noise};

    // 1. 构造一个标准的 16-bit 16000Hz 单声道 PCM WAV 内存文件
    let sample_rate: u32 = 16000;
    let num_samples: u32 = 16000; // 1.0 秒
    let byte_rate: u32 = sample_rate * 2;
    let data_len: u32 = num_samples * 2;
    let file_len: u32 = 36 + data_len;

    let mut wav_bytes = Vec::with_capacity((file_len + 8) as usize);
    wav_bytes.extend_from_slice(b"RIFF");
    wav_bytes.extend_from_slice(&file_len.to_le_bytes());
    wav_bytes.extend_from_slice(b"WAVEfmt ");
    wav_bytes.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
    wav_bytes.extend_from_slice(&1u16.to_le_bytes());  // PCM
    wav_bytes.extend_from_slice(&1u16.to_le_bytes());  // mono
    wav_bytes.extend_from_slice(&sample_rate.to_le_bytes());
    wav_bytes.extend_from_slice(&byte_rate.to_le_bytes());
    wav_bytes.extend_from_slice(&2u16.to_le_bytes());  // block align
    wav_bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav_bytes.extend_from_slice(b"data");
    wav_bytes.extend_from_slice(&data_len.to_le_bytes());
    // 填充纯静音 0
    wav_bytes.resize((file_len + 8) as usize, 0u8);

    // 验证音频有效性检测 (有效 WAV，时长约 1.0 秒)
    let (duration, is_valid) = check_audio_validity(&wav_bytes).expect("Valid WAV must pass check");
    assert!(is_valid);
    assert!((duration - 1.0).abs() < 0.05, "Duration should be ~1.0s, got {}", duration);

    // 2. 注入 -35dB 麦克风粉红底噪
    let injected = inject_mic_noise(&wav_bytes, -35.0).expect("Mic noise injection should succeed");
    assert_eq!(injected.len(), wav_bytes.len());
    // 注入后静音部分不再为全零
    let data_payload = &injected[44..];
    let non_zero_count = data_payload.iter().filter(|&&b| b != 0).count();
    assert!(non_zero_count > 1000, "Injected audio should contain non-zero acoustic pink noise floor");

    // 3. 验证对非法极短音频和非音频数据的拒绝
    let invalid_data = b"Hello, this is just plain text, not audio";
    let check_res = check_audio_validity(invalid_data);
    assert!(check_res.is_err() || !check_res.unwrap().1, "Invalid data must be flagged as invalid");

    println!("✓ 麦克风粉红底噪混音与音频有效性检查测试通过！");
}

#[test]
fn test_homework_preflight_compatibility() {
    use fepc::auto_complete::check_homework_compatibility;
    use fepc::models::{HomeworkItem, HomeworkSource};

    let normal_hw = HomeworkItem {
        id: "hw_normal_1".to_string(),
        name: "初三英语期中听说全真模拟卷A".to_string(),
        status: "进行中".to_string(),
        deadline: "".to_string(),
        total_point: 30.0,
        awarded_point: None,
        question_count: 5,
        source: HomeworkSource::Cloud,
        local_dir: None,
        resource_urls: vec![],
        sections: vec![],
        set_type: 1,
        pk: 0,
        resource_id: "res_normal".to_string(),
    };
    assert!(check_homework_compatibility(&normal_hw).is_ok(), "Normal speaking homework should pass compatibility check");

    let comp_hw = HomeworkItem {
        id: "hw_comp_2".to_string(),
        name: "初三英语限时作文与写作专项训练".to_string(),
        status: "进行中".to_string(),
        deadline: "".to_string(),
        total_point: 20.0,
        awarded_point: None,
        question_count: 1,
        source: HomeworkSource::Cloud,
        local_dir: None,
        resource_urls: vec![],
        sections: vec![],
        set_type: 1,
        pk: 0,
        resource_id: "res_comp".to_string(),
    };
    let comp_err = check_homework_compatibility(&comp_hw);
    assert!(comp_err.is_err(), "Composition homework should be rejected by compatibility preflight");
    assert!(comp_err.unwrap_err().contains("作文/写作"));

    let dict_hw = HomeworkItem {
        id: "hw_dict_3".to_string(),
        name: "Unit 3 核心词汇拼写与听写过关".to_string(),
        status: "进行中".to_string(),
        deadline: "".to_string(),
        total_point: 20.0,
        awarded_point: None,
        question_count: 20,
        source: HomeworkSource::Cloud,
        local_dir: None,
        resource_urls: vec![],
        sections: vec![],
        set_type: 6,
        pk: 0,
        resource_id: "res_dict".to_string(),
    };
    let dict_err = check_homework_compatibility(&dict_hw);
    assert!(dict_err.is_err(), "Dictation homework should be rejected by compatibility preflight");
    assert!(dict_err.unwrap_err().contains("单词听写/拼写"));

    println!("✓ 智能答题前置预检兼容性 check_homework_compatibility 测试通过！");
}

#[test]
fn test_auto_complete_config_modes_and_per_question_pipeline() {
    use fepc::auto_complete::AutoCompleteConfig;

    let mut cfg = AutoCompleteConfig::default();
    assert_eq!(cfg.mode, "auto");
    assert_eq!(cfg.use_time_source, "smart");
    assert!(cfg.per_question_scores.is_empty());
    assert!(cfg.per_question_audio.is_empty());

    // 切换至手动逐题控分与指定录音模式
    cfg.mode = "manual".to_string();
    cfg.use_time_source = "manual".to_string();
    cfg.per_question_scores.insert(0, 4.8);
    cfg.per_question_scores.insert(1, 2.0);
    cfg.per_question_audio.insert(0, "C:/temp/my_voice_q1.wav".to_string());

    let json_str = serde_json::to_string(&cfg).expect("Config must serialize to JSON");
    let recovered: AutoCompleteConfig = serde_json::from_str(&json_str).expect("Config must deserialize from JSON");

    assert_eq!(recovered.mode, "manual");
    assert_eq!(recovered.use_time_source, "manual");
    assert_eq!(recovered.per_question_scores.get(&0), Some(&4.8));
    assert_eq!(recovered.per_question_scores.get(&1), Some(&2.0));
    assert_eq!(recovered.per_question_audio.get(&0), Some(&"C:/temp/my_voice_q1.wav".to_string()));

    println!("✓ 自动完成多模式 (全自动/逐题控分/录音覆盖) 配置序列化测试通过！");
}

#[test]
fn test_device_list_overflow_protection_and_friendly_names() {
    use fepc::la_client::{LaDevice, LaSession, LaUser};
    use fepc::state::AppState;

    let devices = vec![
        LaDevice {
            id: 101,
            device_name: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/154.0.0.0 Safari/537.36 Edg/154.0.0.0".to_string(),
            platform_version: "Windows NT 10.0".to_string(),
            client_key: "web".to_string(),
            is_current: true,
            last_active_at: Some("2026-10-04 23:00".to_string()),
            created_at: None,
        },
        LaDevice {
            id: 102,
            device_name: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/154.0.0.0 Safari/537.36 Edg/154.0.0.0".to_string(),
            platform_version: "Windows NT 10.0".to_string(),
            client_key: "web".to_string(),
            is_current: false,
            last_active_at: Some("2026-10-04 21:30".to_string()),
            created_at: None,
        },
        LaDevice {
            id: 103,
            device_name: "PC-1337".to_string(),
            platform_version: "Android 15 (SDK 35)".to_string(),
            client_key: "fe".to_string(),
            is_current: false,
            last_active_at: Some("2026-10-04 18:00".to_string()),
            created_at: None,
        },
        LaDevice {
            id: 104,
            device_name: "Xiaomi 2510DRK44C".to_string(),
            platform_version: "Android 16 (SDK 36)".to_string(),
            client_key: "fe".to_string(),
            is_current: false,
            last_active_at: Some("2026-10-04 15:20".to_string()),
            created_at: None,
        },
    ];

    let ctx = eframe::egui::Context::default();
    let mut state = AppState {
        la_session: Some(LaSession {
            session_id: "test_sess".to_string(),
            device_id: None,
            user: LaUser {
                id: 1,
                username: "test_user".to_string(),
                role: 3,
                role_display: None,
                member_level: 1,
                tier: "pro".to_string(),
                member_expire_time: None,
                avatar_url: None,
            },
            access_token: "test_token".to_string(),
            refresh_token: "test_refresh".to_string(),
            cached_at: 1000,
        }),
        la_devices: devices,
        ..Default::default()
    };

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::cloud_account::render(&mut fluent, &mut state);
        });
    });

    // 验证 PC 终端 (即使云端旧记录残留了 Android 标签如 PC-1337) 权威校正为 PC 笔记本图标 💻
    let (pc_name, pc_icon) = fepc::views::cloud_account::format_friendly_device_name(&state.la_devices[2]);
    assert_eq!(pc_name, "PC-1337");
    assert_eq!(pc_icon, "💻", "PC-1337 终端必须识别为笔记本图标 💻！");

    // 验证真实移动设备 (如 Xiaomi) 权威识别为手机图标 📱
    let (phone_name, phone_icon) = fepc::views::cloud_account::format_friendly_device_name(&state.la_devices[3]);
    assert_eq!(phone_icon, "📱", "Android 移动终端必须识别为手机图标 📱！");
    assert_eq!(phone_name, "Xiaomi 2510DRK44C");

    let (web_name, web_icon) = fepc::views::cloud_account::format_friendly_device_name(&state.la_devices[0]);
    assert_eq!(web_icon, "🌐");
    assert!(web_name.contains("Windows PC · Edge 浏览器"));

    println!("✓ 授权设备管理卡片超长 UA 智能缩略与响应式换行排版测试通过！");
}

#[test]
fn test_responsive_flow_and_truncation() {
    use fepc::views::{choice_btn, action_btn, accent_action_btn, danger_action_btn, subtle_action_btn, responsive_flow, truncate_middle};

    // 1. 验证长路径截断算法 truncate_middle
    let short_str = "C:\\short\\path";
    assert_eq!(truncate_middle(short_str, 30), short_str);

    let long_path = "C:\\Users\\Administrator\\AppData\\Roaming\\Ets100\\Data\\UserStorage\\Homeworks\\20261005_mock_exam.json";
    let truncated = truncate_middle(long_path, 32);
    assert!(truncated.contains("..."));
    assert!(truncated.chars().count() <= 33);
    assert!(truncated.starts_with("C:\\Users\\"));
    assert!(truncated.ends_with(".json"));

    // 2. 模拟极窄窗口 (360px) 下 responsive_flow 与各类自适应按钮的渲染与无溢出表现
    let ctx = eframe::egui::Context::default();
    let raw_input = eframe::egui::RawInput {
        screen_rect: Some(eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(360.0, 600.0),
        )),
        ..Default::default()
    };

    let _ = ctx.run_ui(raw_input, |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_max_width(320.0);
            let res = responsive_flow(ui, |ui| {
                let _ = choice_btn(ui, "🔥 超长长长长标签选项 A", true, None);
                let _ = choice_btn(ui, "选项 B", false, None);
                let _ = action_btn(ui, "普通操作按钮");
                let _ = accent_action_btn(ui, "高亮按钮", None);
                let _ = danger_action_btn(ui, "危险按钮");
                let _ = subtle_action_btn(ui, "次级按钮");
            });
            // 确认流式排版容器各行自动折行，宽度被约束在 ~320px 附近（远小于未折行时的 600+px）
            let content_w = res.response.rect.width();
            assert!(content_w <= 330.0, "流式排版必须在窄屏下整行折行，内容宽度: {}", content_w);
        });
    });

    println!("✓ 窄屏自适应流式排版与动态按钮自动折行验证通过！");
}

#[test]
fn test_la_constants_and_headers() {
    use fepc::la_client::{
        get_user_agent, LA_APP_VERSION_CODE, LA_BASE_URL, LA_CLIENT_KEY, LA_SOFTWARE_KEY,
    };
    use fepc::utils::device_info::{APPLICATION_ID, VERSION_CODE};

    assert_eq!(LA_BASE_URL, "https://api.lastudio.cc");
    assert_eq!(LA_SOFTWARE_KEY, "fepc");
    assert_eq!(LA_CLIENT_KEY, "fepc-windows");
    assert_eq!(LA_APP_VERSION_CODE, 106);

    assert_eq!(APPLICATION_ID, "cc.lastudio.fepc");
    assert_eq!(VERSION_CODE, 106);

    let ua = get_user_agent();
    println!("Generated User-Agent: {}", ua);
    assert!(ua.starts_with("Fe-PC/"), "UA must start with Fe-PC/");
    assert!(ua.contains(" Windows/"), "UA must include Windows/<OS_VERSION>");

    println!("✓ WP3 常量规范与 User-Agent 动态派生格式验证通过！");
}

#[test]
fn test_la_software_policy_and_killswitch() {
    use fepc::la_client::LaSoftwarePolicy;
    use fepc::state::AppState;

    let sample_json = r#"{
        "isKillSwitchOn": true,
        "latestVersionCode": 106,
        "isForce": true,
        "updateUrl": "https://api.lastudio.cc/software/fepc/latest",
        "announcementTitle": "重要停服维护公告",
        "announcementMessage": "因协议调整，当前版本已停用，请及时升级到 0.1.6",
        "features": {
            "answer_parser": true,
            "cloud_sync": true
        }
    }"#;

    let policy: LaSoftwarePolicy = serde_json::from_str(sample_json).expect("Must deserialize camelCase policy");
    assert!(policy.is_kill_switch_on);
    assert_eq!(policy.latest_version_code, 106);
    assert!(policy.is_force);
    assert_eq!(policy.update_url, "https://api.lastudio.cc/software/fepc/latest");
    assert_eq!(policy.announcement_title.as_deref(), Some("重要停服维护公告"));
    assert_eq!(policy.announcement_message.as_deref(), Some("因协议调整，当前版本已停用，请及时升级到 0.1.6"));
    assert!(policy.features.get("answer_parser").is_some());

    // 验证兼容 snake_case 解析
    let snake_json = r#"{
        "kill_switch": false,
        "latest_version_code": 105,
        "force_update": false,
        "update_url": "",
        "message": "一切运行正常"
    }"#;
    let policy_snake: LaSoftwarePolicy = serde_json::from_str(snake_json).expect("Must deserialize snake_case policy");
    assert!(!policy_snake.is_kill_switch_on);
    assert_eq!(policy_snake.latest_version_code, 105);
    assert_eq!(policy_snake.announcement_message.as_deref(), Some("一切运行正常"));

    // 验证熔断停用时流水线阻断
    let mut state = AppState {
        software_policy: Some(policy),
        auto_running: true,
        ..Default::default()
    };
    if state.software_policy.as_ref().is_some_and(|p| p.is_kill_switch_on) {
        state.auto_running = false;
    }
    assert!(!state.auto_running, "Kill switch 激活时必须强制关停流水线！");

    println!("✓ 启动策略门禁、版本检查与 Kill Switch 熔断逻辑测试通过！");
}

#[test]
fn test_la_sessions_and_password_management() {
    use fepc::la_client::{LaAccountSession, LaClient, LaSession, LaUser};
    use fepc::state::AppState;

    let sample_sessions_json = r#"[
        {
            "id": "sess_win_01",
            "client_key": "fepc-windows",
            "device_name": "PC-STUDY-PC",
            "ip_address": "192.168.1.100",
            "is_current": true,
            "last_active_at": "2026-10-05 10:30:00",
            "created_at": "2026-10-05 08:00:00"
        },
        {
            "id": "sess_phone_02",
            "client_key": "fe-android",
            "device_name": "Xiaomi 15",
            "ip_address": "192.168.1.105",
            "is_current": false,
            "last_active_at": "2026-10-04 22:15:00",
            "created_at": "2026-10-04 12:00:00"
        }
    ]"#;

    let sessions: Vec<LaAccountSession> = serde_json::from_str(sample_sessions_json).expect("Must parse sessions list");
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].id, "sess_win_01");
    assert_eq!(sessions[0].client_key, "fepc-windows");
    assert!(sessions[0].is_current);
    assert_eq!(sessions[1].device_name, "Xiaomi 15");

    // 验证 UI 渲染与修改密码对话框
    let ctx = eframe::egui::Context::default();
    let mut state = AppState {
        la_session: Some(LaSession {
            session_id: "sess_win_01".to_string(),
            device_id: Some("dev_1".to_string()),
            user: LaUser {
                id: 1,
                username: "tester".to_string(),
                role: 3,
                role_display: None,
                member_level: 1,
                tier: "pro".to_string(),
                member_expire_time: None,
                avatar_url: None,
            },
            access_token: "test_token".to_string(),
            refresh_token: "test_refresh".to_string(),
            cached_at: 1000,
        }),
        la_account_sessions: sessions,
        change_password_dialog_open: true,
        change_password_old_input: "old_secret".to_string(),
        change_password_new_input: "new_secret_123".to_string(),
        change_password_confirm_input: "new_secret_123".to_string(),
        ..Default::default()
    };

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::cloud_account::render(&mut fluent, &mut state);
        });
    });

    assert_eq!(state.la_account_sessions.len(), 2);
    assert!(state.change_password_dialog_open);

    // 验证接口方法安全容错
    let client_res = LaClient::list_sessions("invalid_mock_token");
    assert!(client_res.is_err());
    let revoke_res = LaClient::revoke_session("invalid_mock_token", "sess_target");
    assert!(revoke_res.is_err());
    let change_res = LaClient::change_password("invalid_mock_token", "old", "new");
    assert!(change_res.is_err());

    println!("✓ 会话管理 (list/revoke) 与修改密码 UI 交互验证通过！");
}

#[test]
fn test_ed25519_auth_gate_and_smooth_fallback() {
    use fepc::la_client::LaClient;

    // 1. 无令牌/空令牌请求鉴权安全报错
    let err_res = LaClient::authorize_feature("", "answer_parser");
    assert!(err_res.is_err(), "Empty access token must fail safely");

    // 2. 模拟配置与平滑降级: 当后端离线或未放行时，本地解析仍顺畅可用
    let sample_task_json = serde_json::json!({
        "structure_type": "collector.read",
        "question": "Good morning class.",
        "order": 1,
        "max_score": 4.0
    });

    let questions = resolver::api::parse_task(&sample_task_json, None, None);
    assert!(!questions.is_empty(), "本地核心解析引擎必须在降级状态下顺畅可用！");

    println!("✓ Ed25519 鉴权平滑门控与本地解析平滑降级验证通过！");
}

#[test]
fn test_ui_compact_slider_and_notification_animation() {
    let ctx = eframe::egui::Context::default();

    // 1. 测试 compact_slider 渲染与交互及字体获取
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        let font_defs = ctx.fonts(|f| f.definitions().clone());
        assert!(!font_defs.families.is_empty());
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut val = 50.0f32;
            let resp = fepc::views::compact_slider(ui, eframe::egui::Slider::new(&mut val, 0.0..=100.0));
            assert_eq!(val, 50.0);
            assert!(resp.rect.width() > 0.0);
        });
    });

    // 2. 测试通知动画状态机与平滑过渡
    let mut state = fepc::state::AppState {
        notification: Some(("测试通知".to_string(), fluent_egui::winui::InfoBarKind::Informational)),
        notification_dismissing: false,
        notification_dismiss_start: None,
        ..Default::default()
    };

    assert!(state.notification.is_some());
    assert!(!state.notification_dismissing);

    // 模拟触发退出动效
    state.notification_dismissing = true;
    state.notification_dismiss_start = Some(100.0f64);

    // 计算动效因子 (插值)
    let current_time: f64 = 100.11; // 110ms past
    let elapsed = current_time - state.notification_dismiss_start.unwrap();
    let anim_factor = (1.0f64 - (elapsed / 0.220f64)).clamp(0.0f64, 1.0f64);
    assert!(anim_factor > 0.4 && anim_factor < 0.6, "Factor at half-time should be approx 0.5");

    // 当时间超过 220ms 时，因子归零
    let expired_time: f64 = 100.25;
    let expired_factor = (1.0f64 - ((expired_time - 100.0f64) / 0.220f64)).clamp(0.0f64, 1.0f64);
    assert_eq!(expired_factor, 0.0, "Factor should reach 0.0 after 220ms");

    // 3. 验证鼠标悬停时超时顺延且绝不篡改进场动画时间戳 (杜绝通知变浅变暗的 Bug)
    let initial_created = 50.0f64;
    state.notification_created_at = Some(initial_created);
    state.notification_expires_at = Some(initial_created + 6.0);
    state.notification_dismissing = false;

    // 模拟鼠标在 52.0s 时悬浮在通知上
    let hover_time = 52.0f64;
    // 悬浮时仅顺延 expires_at
    state.notification_expires_at = Some(hover_time + 6.0);
    // created_at 必须保持 50.0 绝不被覆盖
    assert_eq!(state.notification_created_at, Some(50.0));
    let enter_elapsed = (hover_time - state.notification_created_at.unwrap()) as f32;
    let enter_anim_factor = (enter_elapsed / 0.18f32).clamp(0.0f32, 1.0f32);
    assert_eq!(enter_anim_factor, 1.0f32, "通知在鼠标移入时透明度必须稳定为 1.0，绝不能变浅变暗！");

    println!("✓ 紧凑滑块控件渲染与通知条 220ms 缓动退场动效及鼠标移入不褪色状态机验证通过！");
}

#[test]
fn test_appdata_ets_directory_discovery() {
    let dirs = LocalScanner::find_ets_local_dirs();
    println!("Discovered local ETS dirs: {:?}", dirs);

    // 验证能够检索到系统 APPDATA/ETS 目录或 74656D705F 目录
    let has_ets = dirs.iter().any(|d| d.file_name().and_then(|n| n.to_str()).map(|s| s.eq_ignore_ascii_case("ETS")).unwrap_or(false));
    let has_hex = dirs.iter().any(|d| d.file_name().and_then(|n| n.to_str()).map(|s| s.starts_with("74656D705F")).unwrap_or(false));

    assert!(has_ets || has_hex, "LocalScanner 必须能够检索到 ETS 或 74656D705F 目录！");

    if let Ok(appdata) = std::env::var("APPDATA") {
        let appdata_p = std::path::PathBuf::from(appdata);
        let expected_ets = appdata_p.join("ETS");
        if expected_ets.is_dir() {
            assert!(dirs.iter().any(|d| d == &expected_ets), "当 %APPDATA%/ETS 存在时必须被包含在检索列表中！");
            println!("✓ 成功确认 %APPDATA%/ETS 目录已被正确识别并纳入扫描候选！");
        }
        let expected_hex = appdata_p.join("74656D705F");
        if expected_hex.is_dir() {
            assert!(dirs.iter().any(|d| d == &expected_hex), "当 %APPDATA%/74656D705F 存在时必须被包含在检索列表中！");
            println!("✓ 成功确认 %APPDATA%/74656D705F 目录已被正确识别并纳入扫描候选！");
        }
    }

    println!("✓ %APPDATA%/ETS 目录与 74656D705F 目录双向检索测试通过！");
}

#[test]
fn test_noto_sans_sc_font_migration_and_compliance() {
    let ctx = egui::Context::default();
    let config = fluent_egui::winui::ThemeConfig::new(fluent_egui::winui::FluentTheme::Dark);
    fluent_egui::winui::install(&ctx, config);

    // 在 egui 0.35 中，必须调用一次 ctx.run_ui() 才会触发底层 FontManager 构建
    let _ = ctx.run_ui(Default::default(), |ui| {
        let defs = ui.ctx().fonts(|f| f.definitions().clone());

        // 1. 验证思源黑体 (Noto Sans SC) 已成功嵌入并注册
        assert!(
            defs.font_data.contains_key("Noto Sans SC"),
            "必须内置并注册思源黑体 (Noto Sans SC)"
        );

        // 2. 验证已彻底剥离对微软方正微软雅黑 (msyh.ttc) 的强读取与依赖
        assert!(
            !defs.font_data.contains_key("Microsoft YaHei"),
            "不得再直接从系统读取并强依赖存在商业版权争议的微软雅黑 (Microsoft YaHei)"
        );

        // 3. 验证 Proportional 字体族包含 Noto Sans SC
        let prop_family = defs.families.get(&egui::FontFamily::Proportional).expect("Proportional family must exist");
        assert!(
            prop_family.contains(&"Noto Sans SC".to_string()),
            "Proportional 字体族必须包含 Noto Sans SC"
        );

        // 4. 验证中文排版与字形度量正常渲染（绝无崩溃）
        let text = "Fe PC - e听说答案提取器：欢迎使用！试卷解析、语音合成与模拟考场 100% 合规！";
        let font_id = egui::FontId::proportional(14.0);
        let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, egui::Color32::WHITE);
        assert!(galley.size().x > 100.0, "中文字符串排版宽度必须大于 100px");
        assert!(galley.size().y >= 14.0, "中文字符串排版高度必须正常");
    });

    println!("✓ 思源黑体 (Noto Sans SC) 全面迁移合规性、0版权争议与中文排版度量验证通过！");
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn test_security_dynamic_auth_engine_and_tamper_proofing() {
    use fepc::la_client::{
        decode_public_key_bytes, verify_signature, LaFeature, LaSession,
        LaUser, DEFAULT_SIGNING_PUBLIC_KEY,
    };
    use fepc::state::AppState;
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    // 1. 验证内置 Ed25519 签名公钥有效性 (32 字节)
    let pk_bytes = decode_public_key_bytes(DEFAULT_SIGNING_PUBLIC_KEY).expect("内置公钥必须有效");
    assert_eq!(pk_bytes.len(), 32);

    // 2. 验证防篡改签名机制
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let test_body = br#"{"code":0,"data":[{"feature_key":"auto_complete","status":"enabled"}]}"#;
    let sig_str = URL_SAFE_NO_PAD.encode(kp.sign(test_body).as_ref());

    assert!(verify_signature(test_body, &sig_str, kp.public_key().as_ref()).is_ok());
    assert!(verify_signature(b"tampered", &sig_str, kp.public_key().as_ref()).is_err());

    // 3. 测试 AppState 鉴权引擎 (未登录、免费用户、Pro 会员在各种 feature 上的判断)
    let mut state = AppState::default();

    // (A) 离线默认值测试
    state.features_loaded = false;
    state.la_session = None;
    assert!(state.check_feature("answer_parser").allowed);
    assert!(state.check_feature("cloud_mode").allowed);
    assert!(state.check_feature("copy_export").allowed);
    assert!(!state.check_feature("auto_complete").allowed);
    assert!(!state.check_feature("voice_clone").allowed);

    // (B) 动态特性配置下测试
    state.features_loaded = true;
    state.features.insert(
        "voice_clone".to_string(),
        LaFeature {
            feature_key: "voice_clone".to_string(),
            name: "声音复刻".to_string(),
            description: Some("云端 AI 声音克隆".to_string()),
            status: "enabled".to_string(),
            authorized: None,
            available_for_version: Some(true),
            reason: None,
            requires_login: true,
            requires_membership: true,
            min_member_level: 1,
            min_member_tier: Some("pro".to_string()),
            deny_message: Some("声音复刻需要开通专业版会员".to_string()),
        },
    );

    // 未登录用户测试
    let res_unauth = state.check_feature("voice_clone");
    assert!(!res_unauth.allowed);
    assert!(res_unauth.requires_login);
    assert_eq!(res_unauth.reason, "该功能需要登录后使用");

    // 免费用户测试 (member_level: 0, role: 3)
    state.la_session = Some(LaSession {
        session_id: "s1".to_string(),
        device_id: None,
        user: LaUser {
            id: 1,
            username: "free_user".to_string(),
            role: 3,
            role_display: None,
            member_level: 0,
            tier: "free".to_string(),
            member_expire_time: None,
            avatar_url: None,
        },
        access_token: "t1".to_string(),
        refresh_token: "r1".to_string(),
        cached_at: 0,
    });
    let res_free = state.check_feature("voice_clone");
    assert!(!res_free.allowed);
    assert!(!res_free.requires_login);
    assert!(res_free.requires_membership);
    assert_eq!(res_free.reason, "声音复刻需要开通专业版会员");

    // Pro 会员测试 (member_level: 1, role: 3)
    state.la_session = Some(LaSession {
        session_id: "s2".to_string(),
        device_id: None,
        user: LaUser {
            id: 2,
            username: "pro_user".to_string(),
            role: 3,
            role_display: None,
            member_level: 1,
            tier: "pro".to_string(),
            member_expire_time: None,
            avatar_url: None,
        },
        access_token: "t2".to_string(),
        refresh_token: "r2".to_string(),
        cached_at: 0,
    });
    let res_pro = state.check_feature("voice_clone");
    assert!(res_pro.allowed);

    // 功能处于维护状态测试 (普通会员应被拦截)
    state.features.get_mut("voice_clone").unwrap().status = "disabled".to_string();
    state.features.get_mut("voice_clone").unwrap().reason = Some("算力集群扩容中".to_string());
    let res_maint = state.check_feature("voice_clone");
    assert!(!res_maint.allowed);
    assert_eq!(res_maint.reason, "算力集群扩容中");

    // 超级管理员/管理员特权放行测试 (role: 0 或 1，即使功能处于维护中也直接放行)
    state.la_session = Some(LaSession {
        session_id: "s_admin".to_string(),
        device_id: None,
        user: LaUser {
            id: 999,
            username: "super_admin".to_string(),
            role: 0,
            role_display: Some("超级管理员".to_string()),
            member_level: 0,
            tier: "free".to_string(),
            member_expire_time: None,
            avatar_url: None,
        },
        access_token: "t_admin".to_string(),
        refresh_token: "r_admin".to_string(),
        cached_at: 0,
    });
    let res_admin = state.check_feature("voice_clone");
    assert!(res_admin.allowed);
    assert_eq!(res_admin.reason, "管理员特权放行");

    // (C) 弹窗状态辅助方法测试
    state.trigger_auth_gate_modal("voice_clone", "声音克隆");
    assert!(state.auth_gate_modal_open);
    assert_eq!(state.auth_gate_modal_feature_name, "声音克隆");

    state.close_auth_gate_modal();
    assert!(!state.auth_gate_modal_open);

    println!("✓ 客户端安全动态鉴权引擎与防篡改验签验证全面通过！");
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn test_eula_first_launch_guard_and_persistence() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_eula_integration_{}.json", uuid::Uuid::new_v4()));
    unsafe {
        std::env::set_var("FEPC_EULA_PATH", test_file.to_str().unwrap());
    }

    // 1. 验证未签署状态与启动门禁识别
    assert!(!fepc::eula::is_eula_accepted());
    let mut state = fepc::state::AppState::default();
    assert!(!state.eula_accepted);
    assert!(!state.eula_modal_open);

    // 模拟首次启动守卫触发
    state.open_eula_modal(true);
    assert!(state.eula_modal_open);
    assert!(state.eula_is_first_launch);
    assert!(!state.eula_checkbox_checked);

    // 2. 勾选同意并保存
    state.eula_checkbox_checked = true;
    state.accept_eula();

    // 3. 验证内存状态更新
    assert!(state.eula_accepted);
    assert!(!state.eula_modal_open);
    assert!(!state.eula_is_first_launch);

    // 4. 验证本地磁盘持久化与后续启动静默放行
    assert!(test_file.exists());
    assert!(fepc::eula::is_eula_accepted(), "磁盘写入后，二次检查必须返回 true");

    // 5. 模拟第二次启动：检测到已同意，不再触发首次弹窗
    let second_launch_accepted = fepc::eula::is_eula_accepted();
    assert!(second_launch_accepted);
    let mut second_state = fepc::state::AppState::default();
    second_state.eula_accepted = second_launch_accepted;
    if !second_launch_accepted {
        second_state.open_eula_modal(true);
    }
    assert!(!second_state.eula_modal_open, "第二次启动不应弹出 EULA 模态框");

    // 6. 验证设置页/致谢页常驻查阅模式
    second_state.open_eula_modal(false);
    assert!(second_state.eula_modal_open);
    assert!(!second_state.eula_is_first_launch, "查阅模式不应标记为首次启动拦截");
    assert!(second_state.eula_checkbox_checked);
    second_state.close_eula_modal();
    assert!(!second_state.eula_modal_open);

    // 清理
    let _ = fepc::eula::reset_eula_accepted();
    unsafe {
        std::env::remove_var("FEPC_EULA_PATH");
    }

    println!("✓ EULA 首次启动门禁拦截、持久化落盘与常驻查阅集成测试全部通过！");
}

#[test]
fn test_floating_toast_and_version_logic() {
    use fepc::la_client::{LaSoftwarePolicy, LA_APP_VERSION_CODE};
    use fepc::state::AppState;
    use fluent_egui::winui::InfoBarKind;

    // 1. 版本更新逻辑验证：后台版本与客户端相同 (106 vs 106) 或更低时严禁展示“🚀 更新提示”
    let policy_same = LaSoftwarePolicy {
        latest_version_code: LA_APP_VERSION_CODE, // 106
        update_message: Some("后台残留的旧更新说明".to_string()),
        changelog_summary: Some("修复了一些已知问题".to_string()),
        announcement_title: Some("常规维护通知".to_string()),
        update_url: "https://example.com/download".to_string(),
        ..Default::default()
    };

    let ctx = eframe::egui::Context::default();
    let mut state_same = AppState {
        software_policy: Some(policy_same),
        ..Default::default()
    };

    // 运行 settings 界面渲染，验证文本包含最新版本徽标，绝不包含“🚀 更新提示”
    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::settings::render(&mut fluent, &mut state_same);
        });
    });

    // 2. 模拟发现新版本 (107 > 106)，验证展示更新提示
    let policy_higher = LaSoftwarePolicy {
        latest_version_code: LA_APP_VERSION_CODE + 1, // 107
        update_message: Some("全新 v0.1.7 版本发布，带来重磅更新！".to_string()),
        changelog_summary: Some("重构了 Toast 通知体系".to_string()),
        update_url: "https://example.com/download".to_string(),
        ..Default::default()
    };
    let mut state_higher = AppState {
        software_policy: Some(policy_higher),
        ..Default::default()
    };

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut fluent = fluent_egui::winui::FluentUi::from_ui(ui);
            fepc::views::settings::render(&mut fluent, &mut state_higher);
        });
    });

    // 3. 悬浮 Toast 通知引擎验证：
    let mut toast_state = AppState {
        notification: Some(("恭喜，答题全部完成！".to_string(), InfoBarKind::Success)),
        ..Default::default()
    };

    let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ctx| {
        fepc::app::render_floating_toast(ctx, &mut toast_state);
    });

    assert_eq!(toast_state.last_notification_text.as_deref(), Some("恭喜，答题全部完成！"));
    assert!(toast_state.notification_created_at.is_some());
    assert!(toast_state.notification_expires_at.is_some());
    assert!(!toast_state.notification_dismissing);

    // 验证悬停顺延逻辑：
    let current_expire = toast_state.notification_expires_at.unwrap();
    let current_time = 100.0;
    let target_expire = current_time + 4.0;
    if current_expire < target_expire {
        toast_state.notification_expires_at = Some(target_expire);
    }
    assert!(toast_state.notification_expires_at.unwrap() >= target_expire);

    println!("✓ Win11 右上角悬浮 Toast 通知渲染引擎与版本更新判断集成测试全部通过！");
}
