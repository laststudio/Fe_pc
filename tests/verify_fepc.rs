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
    let mut state = fepc::state::AppState::default();
    state.local_ets_dirs = fepc::local::LocalScanner::find_ets_local_dirs();
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
        rate: "+0%".to_string(),
        pitch: "+0Hz".to_string(),
        use_time_seconds: 600,
        no_upload: true,
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

