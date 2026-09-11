use super::*;
use serde_json::json;

fn editor(path: &Path) -> Editor {
    let mut e = Editor::new(crate::config::EditorConfig::default()).unwrap();
    e.open(path.to_str().unwrap()).unwrap();
    e
}
fn empty() -> Editor {
    Editor::new(crate::config::EditorConfig::default()).unwrap()
}
fn text(e: &Editor) -> String {
    String::from_utf8(content(e).unwrap()).unwrap()
}
fn setup() -> (Scratch, Vec<PathBuf>) {
    let root = Scratch::new().unwrap();
    let paths: Vec<_> = ["a.rs", "b.rs", "c.rs"]
        .map(|name| root.0.join(name))
        .into();
    for path in &paths {
        fs::write(path, "old();\n").unwrap();
    }
    (root, paths)
}
fn edit(start: u64, end: u64, new: &str) -> Value {
    json!({"range":{"start":{"line":0,"character":start},"end":{"line":0,"character":end}},"newText":new})
}
fn prepare_all(root: &Path, paths: &[PathBuf], panes: &[&Editor]) -> PreparedRename {
    let changes: serde_json::Map<_, _> = paths
        .iter()
        .map(|p| (lsp::file_uri(p).unwrap(), json!([edit(0, 3, "renamed")])))
        .collect();
    let docs: Vec<_> = panes
        .iter()
        .map(|e| lsp::Document {
            path: e.path.as_ref().unwrap().canonicalize().unwrap(),
            text: text(e),
        })
        .collect();
    prepare(
        &json!({"changes":changes}),
        root,
        &docs,
        &HashMap::new(),
        SystemTime::now(),
    )
    .unwrap()
}
#[test]
fn rename_groups_two_unsaved_buffers_and_disk_with_undo_redo() {
    let (root, paths) = setup();
    let mut a = editor(&paths[0]);
    let mut b = editor(&paths[1]);
    a.document.move_cursor(a.document.len()).unwrap();
    a.insert_text("// unsaved\n").unwrap();
    let prior = text(&a);
    let previous_history = a.undo_stack.len();
    let record = Arc::new(prepare_all(&root.0, &paths, &[&a, &b]));
    apply(record, &mut a, &mut b).unwrap();
    assert_eq!(text(&a), "renamed();\n// unsaved\n");
    assert_eq!(a.undo_stack.len(), previous_history + 1);
    assert_eq!(fs::read_to_string(&paths[0]).unwrap(), "old();\n");
    assert_eq!(fs::read_to_string(&paths[2]).unwrap(), "renamed();\n");
    assert!(history(&mut b, &mut a, false).unwrap());
    assert_eq!(text(&a), prior);
    assert_eq!(text(&b), "old();\n");
    assert_eq!(fs::read_to_string(&paths[2]).unwrap(), "old();\n");
    assert!(history(&mut a, &mut b, true).unwrap());
    assert_eq!(text(&b), "renamed();\n");
    assert!(history(&mut a, &mut b, false).unwrap());
    a.undo().unwrap();
    assert_eq!(text(&a), "old();\n");
    a.redo().unwrap();
    assert_eq!(text(&a), prior);
}
#[test]
fn stale_disk_buffer_and_readonly_fail_before_any_changes() {
    for cause in 0..3 {
        let (root, paths) = setup();
        let mut a = editor(&paths[0]);
        let mut b = editor(&paths[1]);
        let record = Arc::new(prepare_all(&root.0, &paths, &[&a, &b]));
        match cause {
            0 => fs::write(&paths[2], "external").unwrap(),
            1 => b.insert_text("draft").unwrap(),
            _ => b.read_only = true,
        }
        let original_a = text(&a);
        let original_b = text(&b);
        assert!(apply(record, &mut a, &mut b).is_err());
        assert_eq!(text(&a), original_a);
        assert_eq!(text(&b), original_b);
        assert!(a.undo_stack.is_empty());
        assert_eq!(
            fs::read_to_string(&paths[2]).unwrap(),
            if cause == 0 { "external" } else { "old();\n" }
        );
    }
}
#[test]
fn undo_refuses_newer_edits_and_external_disk_changes_without_partial_undo() {
    let (root, paths) = setup();
    let mut a = editor(&paths[0]);
    let mut b = editor(&paths[1]);
    apply(
        Arc::new(prepare_all(&root.0, &paths, &[&a, &b])),
        &mut a,
        &mut b,
    )
    .unwrap();
    b.insert_text("x").unwrap();
    assert!(history(&mut a, &mut b, false).is_err());
    assert_eq!(text(&a), "renamed();\n");
    b.undo().unwrap();
    fs::write(&paths[2], "external").unwrap();
    assert!(history(&mut a, &mut b, false).is_err());
    assert_eq!(text(&a), "renamed();\n");
    assert_eq!(text(&b), "renamed();\n");
    fs::write(&paths[2], "renamed();\n").unwrap();
    assert!(history(&mut a, &mut b, false).unwrap());
}
#[test]
fn disk_only_edit_still_has_an_undo_anchor_and_preview_is_non_mutating() {
    let (root, paths) = setup();
    let mut a = editor(&paths[0]);
    let mut b = empty();
    let prepared = prepare_all(&root.0, &paths[1..], &[&a]);
    assert_eq!(fs::read_to_string(&paths[1]).unwrap(), "old();\n");
    assert!(prepared.choices()[0].contains("writes file"));
    apply(Arc::new(prepared), &mut a, &mut b).unwrap();
    assert!(!a.dirty);
    assert!(history(&mut a, &mut b, false).unwrap());
    assert_eq!(fs::read_to_string(&paths[1]).unwrap(), "old();\n");
}
#[test]
fn workspace_parser_rejects_overlap_versions_resource_ops_and_unicode_splits() {
    let (root, paths) = setup();
    fs::write(&paths[0], "🦀old\r\nnext\n").unwrap();
    let uri = lsp::file_uri(&paths[0]).unwrap();
    let bad = [
        json!({"changes":{&uri:[edit(1,2,"x")]}}),
        json!({"changes":{&uri:[edit(2,4,"x"),edit(3,5,"y")]}}),
        json!({"documentChanges":[{"textDocument":{"uri":uri,"version":99},"edits":[edit(2,5,"new")]}]}),
        json!({"documentChanges":[{"kind":"create","uri":uri}]}),
        json!({"changes":{&uri:[edit(2,99,"x")]}}),
    ];
    for edit in bad {
        assert!(prepare(&edit, &root.0, &[], &HashMap::new(), SystemTime::now()).is_err());
    }
    let prepared = prepare(
        &json!({"changes":{&uri:[edit(2,5,"new")]}}),
        &root.0,
        &[],
        &HashMap::new(),
        SystemTime::now(),
    )
    .unwrap();
    assert_eq!(
        read(&prepared.files[0].after).unwrap(),
        "🦀new\r\nnext\n".as_bytes()
    );
    assert_eq!(
        byte_position("a\n", &json!({"line":1,"character":0})).unwrap(),
        2
    );
    assert!(byte_position("a\n", &json!({"line":2,"character":0})).is_err());
}
#[test]
fn disk_commit_failure_rolls_back_previous_files() {
    let (root, paths) = setup();
    let record = Arc::new(prepare_all(&root.0, &paths, &[]));
    let writes: Vec<_> = record
        .files
        .iter()
        .map(|f| {
            DiskWrite::stage(&f.path, read(&f.before).unwrap(), read(&f.after).unwrap()).unwrap()
        })
        .collect();
    fs::remove_file(&writes[1].temp).unwrap();
    assert!(commit_disk(&writes, &record).is_err());
    for path in paths {
        assert_eq!(fs::read_to_string(path).unwrap(), "old();\n");
    }
}
#[test]
fn preview_choices_work_with_mouse_enter_and_paging() {
    let mut bar = crate::command_bar::CommandBar::new();
    bar.open(":rename next");
    assert_eq!(
        bar.parse().unwrap(),
        crate::command_bar::ParsedCommand::Rename {
            name: "next".into()
        }
    );
    bar.show_review((0..15).map(|i| format!("file {i}")).collect());
    assert_eq!(bar.review_selection(), Some(0));
    assert!(bar.select_suggestion(4));
    assert!(bar.prepare_execute());
    assert_eq!(bar.review_selection(), Some(4));
    for _ in 0..8 {
        bar.move_selection(1);
    }
    assert_eq!(bar.review_selection(), Some(12));
    bar.show_info("preview");
    assert!(bar.prepare_execute());
    assert_eq!(bar.review_selection(), None);
    bar.insert_text("x");
    assert!(!bar.is_info());
    assert_eq!(bar.review_selection(), None);
}

#[test]
fn vim_insert_groups_cannot_swallow_a_workspace_undo_marker() {
    struct Clipboard;
    impl crate::clipboard::TextClipboard for Clipboard {
        fn set_text(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn text(&self) -> Result<String, String> {
            Ok(String::new())
        }
    }
    let (root, paths) = setup();
    let mut a = editor(&paths[0]);
    let mut b = empty();
    a.config.keybinding_mode = crate::config::KeybindingMode::Vim;
    let mut vim = crate::vim::VimController::new();
    a.document.move_cursor(a.document.len()).unwrap();
    vim.handle_key(
        &mut a,
        &Clipboard,
        crate::Keycode::I,
        crate::Mod::NOMOD,
        false,
        20,
    )
    .unwrap();
    a.insert_text("// before\n").unwrap();
    vim.record_text("// before\n");
    let original = text(&a);
    apply(
        Arc::new(prepare_all(&root.0, &paths[..1], &[&a])),
        &mut a,
        &mut b,
    )
    .unwrap();
    vim.finish_formatting(&mut a);
    assert!(matches!(
        a.undo_stack.last().unwrap().kind,
        HistoryKind::Workspace(_)
    ));
    a.insert_text("after").unwrap();
    vim.record_text("after");
    vim.handle_key(
        &mut a,
        &Clipboard,
        crate::Keycode::Escape,
        crate::Mod::NOMOD,
        false,
        20,
    )
    .unwrap();
    a.undo().unwrap();
    assert!(history(&mut a, &mut b, false).unwrap());
    assert_eq!(text(&a), original);
    a.undo().unwrap();
    assert_eq!(text(&a), "old();\n");
}
