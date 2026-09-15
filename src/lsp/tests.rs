use super::*;

#[test]
fn documented_server_recipes_parse_and_select_each_language() {
    let guide = include_str!("../../docs/lsp.md");
    let recipes: Vec<_> = guide
        .split("```toml\n")
        .skip(1)
        .map(|part| part.split_once("```").unwrap().0)
        .collect();
    assert!(!recipes.is_empty());
    for recipe in &recipes {
        Config::parse(&format!("enabled = true\n{recipe}")).unwrap();
    }
    let combined = Config::parse(&format!("enabled = true\n{}", recipes.join("\n"))).unwrap();
    for (extension, language) in [
        ("rs", "rust"),
        ("c", "c"),
        ("cpp", "cpp"),
        ("cs", "csharp"),
        ("py", "python"),
        ("js", "javascript"),
        ("jsx", "javascriptreact"),
        ("ts", "typescript"),
        ("tsx", "typescriptreact"),
        ("go", "go"),
        ("java", "java"),
        ("php", "php"),
        ("sh", "shellscript"),
        ("lua", "lua"),
        ("luau", "luau"),
        ("html", "html"),
        ("css", "css"),
        ("json", "json"),
        ("jsonc", "jsonc"),
        ("yaml", "yaml"),
        ("toml", "toml"),
        ("sql", "sql"),
    ] {
        let file = format!("example.{extension}");
        assert_eq!(
            combined.server_for(Path::new(&file)).unwrap().language_id,
            language
        );
    }
    assert!(combined.server_for(Path::new("example.vue")).is_none());
}

#[test]
fn configuration_is_opt_in_and_validated() {
    let config = Config::parse(crate::embedded_config::LSP).unwrap();
    assert!(!config.enabled);
    assert_eq!(
        config.server_for(Path::new("main.RS")).unwrap().name,
        "rust"
    );
    assert!(Config::parse("enabled = true\nunknown = 1").is_err());
    assert!(
        Config::parse("[[servers]]\nname='x'\ncommand=''\nextensions=['rs']\nlanguage_id='rust'")
            .is_err()
    );
}

#[test]
fn unicode_positions_and_incremental_changes() {
    assert_eq!(
        position("a🦀é\r\nx", 7).unwrap(),
        json!({"line":0,"character":4})
    );
    assert_eq!(
        position("a🦀é\r\nx", 9).unwrap(),
        json!({"line":1,"character":0})
    );
    assert_eq!(utf16_column("a🦀é", 3).unwrap(), 2);
    assert!(utf16_column("a🦀é", 2).is_err());
    assert!(position("🦀", 1).is_err());
    assert_eq!(
        incremental_change("hello 🦀!", "hello é!").unwrap(),
        json!({
            "range":{"start":{"line":0,"character":6},"end":{"line":0,"character":8}},"text":"é"
        })
    );
    assert_eq!(
        incremental_change("x\r\ny", "x\ny").unwrap(),
        json!({
            "range":{"start":{"line":0,"character":1},"end":{"line":1,"character":0}},"text":"\n"
        })
    );
}

#[test]
fn incremental_changes_reconstruct_unicode_and_multiline_documents() {
    fn offset(text: &str, point: &Value) -> usize {
        let line = point["line"].as_u64().unwrap() as usize;
        let start = if line == 0 {
            0
        } else {
            text.match_indices('\n').nth(line - 1).unwrap().0 + 1
        };
        let units = point["character"].as_u64().unwrap() as usize;
        let mut used = 0;
        for (byte, ch) in text[start..].char_indices() {
            if used == units {
                return start + byte;
            }
            used += ch.len_utf16();
        }
        assert_eq!(used, units);
        text.len()
    }
    let samples = [
        "",
        "🦀",
        "é",
        "a🦀é",
        "one\ntwo\n",
        "one\r\ntwo\r\n",
        "one\n🦀two\n",
        "one\r\n\r\n",
    ];
    for old in samples {
        for new in samples {
            let change = incremental_change(old, new).unwrap();
            let start = offset(old, &change["range"]["start"]);
            let end = offset(old, &change["range"]["end"]);
            let rebuilt = format!(
                "{}{}{}",
                &old[..start],
                change["text"].as_str().unwrap(),
                &old[end..]
            );
            assert_eq!(rebuilt, new, "{old:?} -> {new:?}");
        }
    }
}

#[test]
fn uri_roundtrip_escapes_spaces_unicode_and_reserved_characters() {
    let path = std::env::temp_dir().join("a space # % 🦀.rs");
    let uri = file_uri(&path).unwrap();
    assert!(uri.contains("%23"));
    assert_eq!(uri_path(&uri).unwrap(), path);
    assert!(uri_path("https://example.com/code.rs").is_err());
    assert!(uri_path("file://remote-host/code.rs").is_err());
}

#[test]
fn hover_documentation_spans_preserve_code_and_plaintext_signatures() {
    let content = hover_content(&json!({"contents": [
        {"language":"rust", "value":"fn greeting() -> &'static str"},
        "Returns a greeting.",
        {"language":"rust", "value":"greeting();"}
    ]}));
    let docs: String = content.documentation.iter().map(|range| &content.text[range.clone()]).collect();
    assert_eq!(docs, "Returns a greeting.");
    let content = hover_content(&json!({"contents": {"kind":"plaintext", "value":
        "crate_name\n\nfn greeting()\n\n\nReturns café.\nMore documentation."}}));
    assert_eq!(&content.text[content.documentation[0].clone()], "Returns café.\nMore documentation.");
    let content = hover_content(&json!({"contents": {"kind":"plaintext", "value":"fn greeting()"}}));
    assert!(content.documentation.is_empty());
    let content = hover_content(&json!({"contents": {"kind":"markdown", "value":
        "Description\n```rust\nfn example() {}\n```\nMore description"}}));
    let docs: String = content.documentation.iter().map(|range| &content.text[range.clone()]).collect();
    assert_eq!(docs, "Description\nMore description");
}

#[test]
fn parses_hover_variants_and_definition_links() {
    assert_eq!(
        hover_text(&json!({"contents":["one",{"language":"rust","value":"two"}]})),
        "one\ntwo"
    );
    assert_eq!(
        hover_text(&json!({"contents":{"kind":"markdown","value":"**hello**"}})),
        "**hello**"
    );
    assert!(!hover_text(&Value::Null).is_empty());
    let path = std::env::temp_dir().join("file.rs");
    let uri = file_uri(&path).unwrap();
    let positions = json!({"start":{"line":4,"character":3},"end":{"line":4,"character":4}});
    let a = definitions(&json!({"uri":uri,"range":positions})).unwrap();
    let b = definitions(&json!([{"targetUri":uri,"targetSelectionRange":positions}])).unwrap();
    assert_eq!(a, b);
    assert_eq!(a[0].path, path);
    assert!(definitions(&Value::Null).unwrap().is_empty());
    assert!(definitions(&json!({"uri":uri,"range":{"start":{"line":-1,"character":0}}})).is_err());
}

#[cfg(unix)]
fn mock_config(mode: &str) -> ServerConfig {
    ServerConfig {
        name: "mock".into(),
        command: "python3".into(),
        args: vec![
            format!(
                "{}/tests/fixtures/lsp_server.py",
                env!("CARGO_MANIFEST_DIR")
            ),
            mode.into(),
        ],
        extensions: vec!["rs".into()],
        language_id: "rust".into(),
        root_markers: vec![],
        initialization_options: Value::Null,
    }
}

#[cfg(unix)]
#[test]
fn server_roundtrip_syncs_unsaved_text_and_closes_documents() {
    for mode in ["incremental", "full"] {
        let (sender, receiver) = mpsc::channel();
        let root = std::env::temp_dir();
        let client = Client::start(mock_config(mode), root.clone(), move |event| {
            sender.send(event).unwrap();
        })
        .unwrap();
        let path = root.join("mock space 🦀.rs");
        let other = root.join("other.rs");
        let send = |id, text: &str, two: bool| {
            let mut documents = vec![Document {
                path: path.clone(),
                text: text.into(),
            }];
            if two {
                documents.push(Document {
                    path: other.clone(),
                    text: "unsaved other".into(),
                });
            }
            client
                .request(Request {
                    id,
                    action: Action::Hover,
                    documents,
                    cursor: text.len(),
                })
                .unwrap();
            let event = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(event.id, id);
            let Reply::Hover(text) = event.result.unwrap() else {
                panic!("expected hover")
            };
            serde_json::from_str::<Value>(&text.text).unwrap()
        };
        let first = send(1, "a🦀\r\nx", true);
        assert_eq!(
            first["documents"][file_uri(&other).unwrap()],
            "unsaved other"
        );
        let second = send(2, "aé\nx!", true);
        assert_eq!(second["documents"][file_uri(&path).unwrap()], "aé\nx!");
        assert_eq!(
            second["changes"][0].get("range").is_some(),
            mode == "incremental"
        );
        assert!(client.keep(vec![path.clone()]));
        let third = send(3, "aé\nx!", false);
        assert_eq!(third["documents"].as_object().unwrap().len(), 1);
        client
            .request(Request {
                id: 4,
                action: Action::Definition,
                documents: vec![Document {
                    path: path.clone(),
                    text: "aé\nx!".into(),
                }],
                cursor: 1,
            })
            .unwrap();
        let event = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        let Reply::Definition(locations) = event.result.unwrap() else {
            panic!("expected definition")
        };
        assert_eq!(
            locations[0],
            Location {
                path,
                line: 0,
                character: 2
            }
        );
        let alive = client.alive.clone();
        drop(client);
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while alive.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!alive.load(Ordering::Acquire), "server worker did not stop");
    }
}

#[cfg(unix)]
#[test]
fn hung_initialization_can_be_cancelled_without_blocking_the_caller() {
    let (sender, receiver) = mpsc::channel();
    let root = std::env::temp_dir();
    let client = Client::start(mock_config("hang"), root.clone(), move |event| {
        let _ = sender.send(event);
    })
    .unwrap();
    client
        .request(Request {
            id: 1,
            action: Action::Hover,
            documents: vec![Document {
                path: root.join("test.rs"),
                text: "x".into(),
            }],
            cursor: 0,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let alive = client.alive.clone();
    let start = std::time::Instant::now();
    drop(client);
    assert!(start.elapsed() < Duration::from_millis(100));
    let _ = receiver.recv_timeout(Duration::from_secs(2));
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while alive.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!alive.load(Ordering::Acquire));
}

#[cfg(unix)]
#[test]
fn crashed_server_reports_failure_and_transport_times_out() {
    let cancel = Arc::new(AtomicBool::new(false));
    let mut server = Transport::start(&mock_config("hang"), &std::env::temp_dir(), cancel).unwrap();
    let start = std::time::Instant::now();
    assert!(
        server
            .request("initialize", json!({}), Duration::from_millis(100))
            .unwrap_err()
            .contains("timed out")
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(server.has_failed(), "initialization timeouts must still stop the server");
    drop(server);
    assert!(
        Session::start(
            &mock_config("crash"),
            &std::env::temp_dir(),
            Arc::new(AtomicBool::new(false))
        )
        .is_err()
    );
}

#[test]
#[ignore = "requires an installed clangd; set POTYI_TEST_CLANGD to its executable"]
fn real_clangd_hover_and_definition() {
    let root = std::env::temp_dir().join(format!("potyi-real-lsp-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("main.cpp");
    let text = "int answer() { return 42; }\nint main() { return answer(); }\n";
    std::fs::write(&path, text).unwrap();
    let config = ServerConfig {
        name: "clangd".into(),
        command: std::env::var("POTYI_TEST_CLANGD").unwrap_or_else(|_| "clangd".into()),
        args: vec!["--background-index=false".into()],
        extensions: vec!["cpp".into()],
        language_id: "cpp".into(),
        root_markers: vec![],
        initialization_options: Value::Null,
    };
    let (sender, receiver) = mpsc::channel();
    let client = Client::start(config, root.clone(), move |event| {
        let _ = sender.send(event);
    })
    .unwrap();
    let cursor = text.rfind("answer").unwrap() + 2;
    let request = |id, action| Request {
        id,
        action,
        documents: vec![Document {
            path: path.clone(),
            text: text.into(),
        }],
        cursor,
    };
    client.request(request(1, Action::Hover)).unwrap();
    let Reply::Hover(hover) = receiver
        .recv_timeout(Duration::from_secs(35))
        .unwrap()
        .result
        .unwrap()
    else {
        panic!("hover expected")
    };
    assert!(hover.text.contains("answer"), "{}", hover.text);
    client.request(request(2, Action::Definition)).unwrap();
    let Reply::Definition(locations) = receiver
        .recv_timeout(Duration::from_secs(20))
        .unwrap()
        .result
        .unwrap()
    else {
        panic!("definition expected")
    };
    assert_eq!(locations[0].line, 0);
    assert_eq!(locations[0].character, 4);
    assert_eq!(
        locations[0].path.canonicalize().unwrap(),
        path.canonicalize().unwrap()
    );
    drop(client);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "requires installed rust-analyzer and rust-src; set POTYI_TEST_RUST_ANALYZER to its executable"]
fn real_rust_analyzer_hover_and_definition() {
    let demo = std::env::var_os("POTYI_TEST_RUST_DEMO").map(PathBuf::from);
    let (root, path, text, config, symbol, type_name) = if let Some(demo) = &demo {
        let path = demo.join("project/src/main.rs").canonicalize().unwrap();
        let settings = Config::parse(&std::fs::read_to_string(demo.join("config/lsp.toml")).unwrap()).unwrap();
        assert!(settings.enabled);
        let config = settings.server_for(&path).unwrap().clone();
        let root = project_root(&path, &config);
        assert_eq!(root, path.parent().unwrap().parent().unwrap(), "demo must use its own Cargo project");
        let text = std::fs::read_to_string(&path).unwrap();
        (root, path, text, config, "greeting", "str")
    } else {
        let root = std::env::temp_dir().join(format!("potyi-real-rust-lsp-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("Cargo.toml"),
            "[package]\nname = \"potyi-lsp-test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n").unwrap();
        let path = root.join("src/main.rs");
        let text = "/// Test hover documentation.\nfn answer() -> u32 { 42 }\nfn main() { let value = answer(); }\n".to_string();
        std::fs::write(&path, &text).unwrap();
        let config = ServerConfig {
            name: "rust".into(),
            command: std::env::var("POTYI_TEST_RUST_ANALYZER").unwrap_or_else(|_| "rust-analyzer".into()),
            args: vec![], extensions: vec!["rs".into()], language_id: "rust".into(),
            root_markers: vec!["Cargo.toml".into()],
            initialization_options: json!({"cachePriming":{"enable":false},"checkOnSave":false}),
        };
        (root, path, text, config, "answer", "u32")
    };
    let (sender, receiver) = mpsc::channel();
    let client = Client::start(config, root.clone(), move |event| {
        let _ = sender.send(event);
    }).unwrap();
    let request = |id, action| Request {
        id, action,
        documents: vec![Document { path: path.clone(), text: text.clone() }],
        cursor: text.rfind(symbol).unwrap() + 2,
    };
    // Regression: rename must work as the first command, without a hover warm-up.
    client.request(request(1000, Action::Rename("cold_renamed".into()))).unwrap();
    let cold = receiver.recv_timeout(Duration::from_secs(40)).unwrap().result;
    assert!(matches!(&cold, Ok(Reply::Rename(_))), "cold call-site rename failed: {cold:?}");
    // Cargo project discovery continues after initialize. Retry empty hover
    // responses while that initial analysis is still loading.
    let deadline = std::time::Instant::now() + Duration::from_secs(45);
    let mut id = 0;
    loop {
        id += 1;
        client.request(request(id, Action::Hover)).unwrap();
        let reply = match receiver.recv_timeout(Duration::from_secs(35)).unwrap().result {
            Err(error) if error.eq_ignore_ascii_case("content modified") && std::time::Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(250)); continue;
            }
            result => result.unwrap(),
        };
        let Reply::Hover(hover) = reply else { panic!("hover expected") };
        if hover.text.contains(symbol) && hover.text.contains(type_name) {
            assert!(hover.documentation.iter().any(|range| {
                let text = &hover.text[range.clone()];
                text.contains("Test hover documentation") || text.contains("Returns a greeting")
            }), "real rust-analyzer documentation must be styled separately from its signature");
            eprintln!("rust-analyzer hover: {}", hover.text);
            break;
        }
        assert!(std::time::Instant::now() < deadline, "rust-analyzer never returned symbol hover: {}", hover.text);
        thread::sleep(Duration::from_millis(250));
    }
    let locations = loop {
        id += 1;
        client.request(request(id, Action::Definition)).unwrap();
        let Reply::Definition(locations) = receiver.recv_timeout(Duration::from_secs(20))
            .unwrap().result.unwrap() else { panic!("definition expected") };
        if !locations.is_empty() { break locations; }
        assert!(std::time::Instant::now() < deadline, "rust-analyzer never returned a definition");
        thread::sleep(Duration::from_millis(250));
    };
    assert_eq!(locations[0].line, 1);
    assert_eq!(locations[0].character, 3);
    assert_eq!(locations[0].path.canonicalize().unwrap(), path.canonicalize().unwrap());
    eprintln!("rust-analyzer definition: main.rs:2:4");
    id += 1;
    client.request(request(id, Action::Rename("renamed_symbol".into()))).unwrap();
    let Reply::Rename(preview) = receiver.recv_timeout(Duration::from_secs(20)).unwrap().result.unwrap()
        else { panic!("rename preview expected") };
    assert_eq!(preview.files.len(), 1);
    assert!(preview.files[0].preview.contains("renamed_symbol"));
    let mut editor = crate::Editor::new(crate::config::EditorConfig::default()).unwrap();
    editor.open(path.to_str().unwrap()).unwrap();
    let mut other = crate::Editor::new(crate::config::EditorConfig::default()).unwrap();
    crate::workspace_edit::apply(Arc::new(preview), &mut editor, &mut other).unwrap();
    let renamed = editor.document.text().unwrap();
    assert_eq!(renamed.matches("renamed_symbol").count(), 2);
    assert!(!renamed.contains(&format!("fn {symbol}")));
    assert_eq!(std::fs::read_to_string(&path).unwrap(),text, "open buffer must not be saved implicitly");
    assert!(crate::workspace_edit::history(&mut editor,&mut other,false).unwrap());
    assert_eq!(editor.document.text().unwrap(),text);
    eprintln!("rust-analyzer rename: declaration + call, preview/apply/undo passed");
    id += 1;
    let mut variable_request = request(id, Action::Rename("renamed_variable".into()));
    variable_request.cursor = text.find("let ").unwrap() + 5;
    client.request(variable_request).unwrap();
    let Reply::Rename(preview) = receiver.recv_timeout(Duration::from_secs(20)).unwrap().result.unwrap()
        else { panic!("variable rename expected") };
    crate::workspace_edit::apply(Arc::new(preview), &mut editor, &mut other).unwrap();
    assert!(editor.document.text().unwrap().contains("let renamed_variable ="));
    assert!(crate::workspace_edit::history(&mut editor,&mut other,false).unwrap());
    assert_eq!(editor.document.text().unwrap(),text);


    drop(client);
    if demo.is_none() { let _ = std::fs::remove_dir_all(root); }
}

#[cfg(unix)]
#[test]
fn a_feature_error_keeps_the_initialized_session_alive() {
    let root = std::env::temp_dir();
    let (sender, receiver) = mpsc::channel();
    let client = Client::start(mock_config("feature-error"), root.clone(), move |event| {
        let _ = sender.send(event);
    }).unwrap();
    for id in 1..=2 {
        client.request(Request { id, action:Action::Hover,
            documents:vec![Document {path:root.join("feature-error.rs"),text:"hello".into()}],cursor:2 }).unwrap();
        let result = receiver.recv_timeout(Duration::from_secs(5)).unwrap().result;
        if id == 1 { assert_eq!(result.unwrap_err(),"No references found at position"); }
        else { assert!(matches!(result,Ok(Reply::Hover(_))),"session restarted: {result:?}"); }
    }
}

#[cfg(unix)]
#[test]
fn readiness_wait_is_bounded_cancellable_and_does_not_break_a_healthy_session() {
    let cancel = Arc::new(AtomicBool::new(false));
    let server = Transport::start(&mock_config("incremental"),&std::env::temp_dir(),cancel.clone()).unwrap();
    let started = std::time::Instant::now();
    assert!(server.wait_until_ready(Duration::from_millis(80)).unwrap_err().contains("still loading"));
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(!server.has_failed());
    let completion_cancel = AtomicBool::new(true);
    assert!(server.wait_until_ready_cancellable(Duration::from_secs(10), Some(&completion_cancel)).unwrap_err().contains("cancelled"));
    assert!(!server.has_failed());
    cancel.store(true,Ordering::Relaxed);
    assert_eq!(server.wait_until_ready(Duration::from_secs(10)).unwrap_err(),"LSP stopped");
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[cfg(unix)]
#[test]
fn code_actions_pass_diagnostics_selection_and_resolve_import_preview() {
    let root = std::env::temp_dir().join(format!("potyi-action-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("main.rs");
    std::fs::write(&path, "🦀Unknown").unwrap();
    let (sender, receiver) = mpsc::channel();
    let client = Client::start(mock_config("actions"), root.clone(), move |e| { sender.send(e).unwrap(); }).unwrap();
    let request = |id, action| Request { id, action, documents: vec![Document {
        path: path.clone(), text: "🦀Unknown".into(),
    }], cursor: 11 };
    client.request(request(1, Action::CodeActions { anchor: 4, refactor_only: false })).unwrap();
    let Reply::CodeActions(actions) = receiver.recv_timeout(Duration::from_secs(10)).unwrap().result.unwrap() else { panic!() };
    assert_eq!(actions.items[0].title, "Add missing import");
    assert_eq!(actions.items[1].disabled.as_deref(), Some("Not applicable"));
    client.request(request(2, Action::ResolveCodeAction { action: actions.items[0].value.clone(), started: actions.started })).unwrap();
    let Reply::Rename(preview) = receiver.recv_timeout(Duration::from_secs(10)).unwrap().result.unwrap() else { panic!() };
    assert!(preview.files[0].preview.contains("use std::fmt;"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "🦀Unknown", "resolution must not apply edits");
    drop(client);
    std::fs::remove_dir_all(root).unwrap();
}


#[test]
#[ignore = "requires installed rust-analyzer and rust-src"]
fn real_rust_analyzer_extract_module_to_file() {
    let root=std::env::temp_dir().join(format!("potyi-extract-module-{}",std::process::id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"),"[package]\nname=\"extract-test\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[workspace]\n").unwrap();
    let path=root.join("src/lib.rs").canonicalize().unwrap_or_else(|_| root.join("src/lib.rs"));
    let text="mod example { pub fn answer() -> u32 { 42 } }\nfn use_map() { let _ = HashMap::<u8, u8>::new(); }\n";
    std::fs::write(&path,text).unwrap();
    let config=ServerConfig { name:"rust".into(), command:std::env::var("POTYI_TEST_RUST_ANALYZER").unwrap_or_else(|_| "rust-analyzer".into()),
        args:vec![],extensions:vec!["rs".into()],language_id:"rust".into(),root_markers:vec!["Cargo.toml".into()],
        initialization_options:json!({"cachePriming":{"enable":false},"checkOnSave":false}) };
    let (sender,receiver)=mpsc::channel();
    let client=Client::start(config,root.clone(),move |e| {let _=sender.send(e);}).unwrap();
    let import_cursor = text.find("HashMap").unwrap() + 2;
    let import_request = |id, action| Request {id, action, documents:vec![Document {path:path.clone(),text:text.into()}], cursor:import_cursor};
    client.request(import_request(10, Action::CodeActions {anchor:import_cursor,refactor_only:false})).unwrap();
    let Reply::CodeActions(imports) = receiver.recv_timeout(Duration::from_secs(45)).unwrap().result.unwrap() else {panic!()};
    let import = imports.items.iter().find(|a| a.title.contains("HashMap") && a.title.to_lowercase().contains("import"))
        .unwrap_or_else(|| panic!("Missing import fix: {:?}",imports.items.iter().map(|a| &a.title).collect::<Vec<_>>()));
    eprintln!("Real rust-analyzer quick fix: {}",import.title);
    client.request(import_request(11,Action::ResolveCodeAction {action:import.value.clone(),started:imports.started})).unwrap();
    let Reply::Rename(import_preview) = receiver.recv_timeout(Duration::from_secs(30)).unwrap().result.unwrap() else {panic!()};
    assert!(import_preview.files[0].preview.contains("use std::collections::HashMap;"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(),text);
    let request=|id,action| Request {id,action,documents:vec![Document {path:path.clone(),text:text.into()}],cursor:5};
    client.request(request(1,Action::CodeActions {anchor:5,refactor_only:true})).unwrap();
    let Reply::CodeActions(actions)=receiver.recv_timeout(Duration::from_secs(45)).unwrap().result.unwrap() else {panic!()};
    let item=actions.items.iter().find(|a| a.title.to_lowercase().contains("file") && a.title.to_lowercase().contains("module"))
        .unwrap_or_else(|| panic!("Missing extraction action: {:?}",actions.items.iter().map(|a| &a.title).collect::<Vec<_>>()));
    eprintln!("Real rust-analyzer action: {}",item.title);
    assert!(item.disabled.is_none(),"{:?}",item.disabled);
    client.request(request(2,Action::ResolveCodeAction {action:item.value.clone(),started:actions.started})).unwrap();
    let Reply::Rename(preview)=receiver.recv_timeout(Duration::from_secs(30)).unwrap().result.unwrap() else {panic!()};
    let mut editor=crate::Editor::new(crate::config::EditorConfig::default()).unwrap(); editor.open(path.to_str().unwrap()).unwrap();
    let mut other=crate::Editor::new(crate::config::EditorConfig::default()).unwrap();
    crate::workspace_edit::apply(std::sync::Arc::new(preview),&mut editor,&mut other).unwrap();
    assert!(std::fs::read_to_string(root.join("src/example.rs")).unwrap().contains("fn answer"));
    assert!(editor.document.text().unwrap().contains("mod example;"));
    crate::workspace_edit::history(&mut editor,&mut other,false).unwrap();
    assert!(!root.join("src/example.rs").exists());
    assert_eq!(editor.document.text().unwrap(),text);
    drop(client); drop(editor); drop(other); std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn completion_cancels_without_breaking_the_session_and_resolves_items() {
    let (sender, receiver) = mpsc::channel();
    let root = std::env::temp_dir();
    let client = Client::start(mock_config("slow-completion"), root.clone(), move |event| { let _ = sender.send(event); }).unwrap();
    let request = |id,action| Request { id, action, documents:vec![Document { path:root.join("completion.cs"),text:"transform.".into() }],cursor:10 };
    // Warm the session, then cancel a request after it has reached the server.
    client.request(request(1,Action::Hover)).unwrap();
    receiver.recv_timeout(Duration::from_secs(5)).unwrap().result.unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    client.request(request(2,Action::Complete {trigger:".".into(),cancel:cancel.clone()})).unwrap();
    thread::sleep(Duration::from_millis(80));
    cancel.store(true,Ordering::Relaxed);
    assert!(receiver.recv_timeout(Duration::from_secs(2)).unwrap().result.unwrap_err().contains("cancelled"));
    client.request(request(3,Action::Complete {trigger:".".into(),cancel:Arc::new(AtomicBool::new(false))})).unwrap();
    let Reply::Completions(items) = receiver.recv_timeout(Duration::from_secs(5)).unwrap().result.unwrap() else {panic!()};
    assert_eq!(items[0].label,"position");
    client.request(request(4,Action::ResolveCompletion {item:items[0].value.clone(),cancel:Arc::new(AtomicBool::new(false))})).unwrap();
    let Reply::CompletionEdit(edit) = receiver.recv_timeout(Duration::from_secs(5)).unwrap().result.unwrap() else {panic!()};
    assert_eq!(edit.text,"transform.position");
    assert_eq!(edit.cursor,edit.text.len());
}

#[test]
#[ignore = "requires installed rust-analyzer and clangd"]
fn real_servers_complete_member_access_and_prepare_insertion() {
    for (name, extension, text, member) in [
        ("rust-analyzer", "rs", "struct Player { health: i32 }\nfn main() { let player = Player { health: 10 }; player. }\n", "health"),
        ("clangd", "cpp", "struct Player { int health; };\nint main() { Player player; player. }\n", "health"),
        ("rust-analyzer", "rs", "struct Player;\nimpl Player { fn heal(&self) {} }\nfn main() { let player = Player; player. }\n", "heal"),
        ("clangd", "cpp", "struct Player { void heal() {} };\nint main() { Player player; player. }\n", "heal"),
    ] {
        let root = std::env::temp_dir().join(format!("potyi-completion-real-{}-{extension}",std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname='completion_test'\nversion='0.1.0'\nedition='2021'\n[workspace]\n").unwrap();
        let path = root.join(format!("src/main.{extension}"));
        std::fs::write(&path,text).unwrap();
        let config = ServerConfig {name:name.into(),command:name.into(),args:if name=="clangd" {vec!["--background-index=false".into()]} else {vec![]},extensions:vec![extension.into()],language_id:if extension=="rs" {"rust"}else{"cpp"}.into(),root_markers:vec![],initialization_options:json!({"cachePriming":{"enable":false},"checkOnSave":false})};
        let (sender,receiver) = mpsc::channel();
        let client = Client::start(config,root.clone(),move |event| {let _=sender.send(event);}).unwrap();
        let request = |id,action| Request {id,action,documents:vec![Document {path:path.clone(),text:text.into()}],cursor:text.find("player.").unwrap()+7};
        let mut selected = None;
        let mut last_labels = Vec::new();
        // The first trigger must succeed even when project loading is still underway.
        for id in 0..1 {
            client.request(request(id,Action::Complete {trigger:".".into(),cancel:Arc::new(AtomicBool::new(false))})).unwrap();
            let Reply::Completions(items) = receiver.recv_timeout(Duration::from_secs(35)).unwrap().result.unwrap() else {panic!()};
            last_labels = items.iter().map(|i| i.label.clone()).collect();
            if let Some(item) = items.into_iter().find(|i|i.label.trim_start().starts_with(member)) { selected=Some(item);break; }
            thread::sleep(Duration::from_millis(200));
        }
        let item = selected.unwrap_or_else(||panic!("{name} did not return {member}: {last_labels:?}"));
        client.request(request(100,Action::ResolveCompletion {item:item.value,cancel:Arc::new(AtomicBool::new(false))})).unwrap();
        let Reply::CompletionEdit(edit) = receiver.recv_timeout(Duration::from_secs(20)).unwrap().result.unwrap() else {panic!()};
        let inserted = format!("player.{member}");
        assert!(edit.text.contains(&inserted),"{name}: {}",edit.text);
        assert!(edit.cursor >= edit.text.find(&inserted).unwrap()+inserted.len());
        drop(client);
        std::fs::remove_dir_all(root).unwrap();
    }
}


#[cfg(unix)]
#[test]
fn hover_timeout_keeps_loaded_documents_and_ignores_late_responses() {
    let root = std::env::temp_dir();
    let config = mock_config("timeout-hover");
    let mut session = Session::start(&config, &root, Arc::new(AtomicBool::new(false))).unwrap();
    let path = root.join("timeout-hover.rs");
    let mut request = Request {
        id: 1, action: Action::Start,
        documents: vec![Document {path: path.clone(), text: "OnEnable".into()}], cursor: 0,
    };
    session.execute(&config, &request).unwrap();
    request.action = Action::Hover;
    let started = std::time::Instant::now();
    let error = session.execute(&config, &request).unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(4), "hover must use its short deadline");
    assert!(error.contains("timed out"));
    assert!(error.contains("server is still running"));
    assert!(!session.transport.has_failed(), "timeout must not trigger reconnection");
    request.action = Action::Hover;
    let Reply::Hover(hover) = session.execute(&config, &request).unwrap() else { panic!() };
    assert!(hover.text.contains("OnEnable"), "server must retain the open document");
    assert!(!hover.text.contains("stale hover"), "late responses must not replace current results");
    assert!(!session.transport.has_failed());
}

#[cfg(unix)]
#[test]
fn crashed_hover_still_marks_the_transport_failed() {
    let config = mock_config("crash-hover");
    let mut session = Session::start(&config, &std::env::temp_dir(), Arc::new(AtomicBool::new(false))).unwrap();
    assert!(session.transport.request("textDocument/hover", json!({}), Duration::from_secs(2)).is_err());
    assert!(session.transport.has_failed());
}
