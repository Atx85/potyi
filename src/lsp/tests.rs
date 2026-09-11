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
            serde_json::from_str::<Value>(&text).unwrap()
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
    assert!(hover.contains("answer"), "{hover}");
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
