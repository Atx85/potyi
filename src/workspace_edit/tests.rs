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


#[test]
fn extract_to_new_file_preserves_unsaved_source_and_has_atomic_undo_redo() {
    let (root, paths) = setup();
    let dest = root.0.join("moved.rs");
    let mut active = editor(&paths[0]); let mut other = empty();
    active.document.move_cursor(active.document.len()).unwrap();
    active.insert_text("// unsaved\n").unwrap();
    let before = text(&active);
    let doc = lsp::Document { path: paths[0].canonicalize().unwrap(), text: before.clone() };
    let workspace = json!({"documentChanges":[
        {"kind":"create","uri":lsp::file_uri(&dest).unwrap()},
        {"textDocument":{"uri":lsp::file_uri(&dest).unwrap(),"version":null},"edits":[edit(0,0,"pub fn moved() {}\n")]},
        {"textDocument":{"uri":lsp::file_uri(&paths[0]).unwrap(),"version":null},"edits":[edit(0,3,"moved")]}
    ]});
    let preview = prepare(&workspace,&root.0,&[doc],&HashMap::new(),SystemTime::now()).unwrap();
    assert!(!dest.exists());
    apply(Arc::new(preview),&mut active,&mut other).unwrap();
    assert_eq!(text(&active),"moved();\n// unsaved\n");
    assert_eq!(fs::read_to_string(&paths[0]).unwrap(),"old();\n");
    assert_eq!(fs::read_to_string(&dest).unwrap(),"pub fn moved() {}\n");
    history(&mut active,&mut other,false).unwrap();
    assert!(!dest.exists()); assert_eq!(text(&active),before);
    history(&mut active,&mut other,true).unwrap();
    assert!(dest.exists()); assert!(text(&active).starts_with("moved"));
    fs::write(&dest,"external change").unwrap();
    assert!(history(&mut active,&mut other,false).is_err());
    assert!(text(&active).starts_with("moved"));
}

#[test]
fn moving_an_open_file_preserves_dirty_text_and_reverses_paths() {
    let (root, paths) = setup();
    let dest = root.0.join("renamed.rs");
    let mut active = editor(&paths[0]); let mut other = empty();
    active.insert_text("// draft\n").unwrap();
    let before = text(&active);
    let workspace = json!({"documentChanges":[{"kind":"rename",
        "oldUri":lsp::file_uri(&paths[0]).unwrap(),"newUri":lsp::file_uri(&dest).unwrap()}]});
    let preview = prepare(&workspace,&root.0,&[lsp::Document {path:paths[0].canonicalize().unwrap(),text:before.clone()}],
        &HashMap::new(),SystemTime::now()).unwrap();
    apply(Arc::new(preview),&mut active,&mut other).unwrap();
    assert!(!paths[0].exists()); assert_eq!(active.path.as_ref().unwrap(),&dest.canonicalize().unwrap());
    assert_eq!(text(&active),before); assert!(active.dirty);
    assert_eq!(fs::read_to_string(&dest).unwrap(),"old();\n", "moving must not silently save a dirty buffer");
    history(&mut active,&mut other,false).unwrap();
    assert!(paths[0].exists()); assert!(!dest.exists()); assert_eq!(text(&active),before);
    history(&mut active,&mut other,true).unwrap(); assert!(dest.exists());
}

#[test]
fn resource_preview_refuses_collisions_and_stale_disk_without_partial_apply() {
    let (root, paths) = setup(); let dest=root.0.join("new.rs");
    let make = || json!({"documentChanges":[
        {"kind":"create","uri":lsp::file_uri(&dest).unwrap()},
        {"textDocument":{"uri":lsp::file_uri(&paths[0]).unwrap(),"version":null},"edits":[edit(0,3,"new")]}
    ]});
    let preview=prepare(&make(),&root.0,&[],&HashMap::new(),SystemTime::now()).unwrap();
    fs::write(&dest,"someone else").unwrap();
    assert!(apply(Arc::new(preview),&mut empty(),&mut empty()).is_err());
    assert_eq!(fs::read_to_string(&paths[0]).unwrap(),"old();\n");
    assert!(prepare(&make(),&root.0,&[],&HashMap::new(),SystemTime::now()).is_err());
    let outside=root.0.parent().unwrap().join("potyi-outside-refactor.rs");
    assert!(prepare(&json!({"documentChanges":[{"kind":"create","uri":lsp::file_uri(&outside).unwrap()}]}),
        &root.0,&[],&HashMap::new(),SystemTime::now()).is_err());
}

#[test]
fn deleting_open_file_keeps_draft_and_undo_restores_file_and_path() {
    let (root, paths) = setup(); let mut active = editor(&paths[0]); let mut other = empty();
    active.insert_text("draft ").unwrap(); let before = text(&active);
    let preview = prepare(&json!({"documentChanges":[{"kind":"delete","uri":lsp::file_uri(&paths[0]).unwrap()}]}),
        &root.0,&[lsp::Document {path:paths[0].clone(),text:before.clone()}],&HashMap::new(),SystemTime::now()).unwrap();
    apply(Arc::new(preview),&mut active,&mut other).unwrap();
    assert!(!paths[0].exists()); assert!(active.path.is_none()); assert!(active.dirty); assert_eq!(text(&active),before);
    assert_eq!(recent_disk_changes(&active,false)[0].1,3);
    history(&mut active,&mut other,false).unwrap();
    assert!(active.path.is_some()); assert_eq!(fs::read_to_string(&paths[0]).unwrap(),"old();\n"); assert_eq!(text(&active),before);
    assert_eq!(recent_disk_changes(&active,true)[0].1,1);
    history(&mut active,&mut other,true).unwrap(); assert!(!paths[0].exists());
}

#[test]
fn annotated_edits_are_reviewed_and_unknown_annotations_and_snippets_rejected() {
    let (root, paths) = setup(); let uri=lsp::file_uri(&paths[0]).unwrap();
    let mut replacement=edit(0,3,"new"); replacement["annotationId"]=json!("note");
    let mut workspace=json!({"changes":{uri.clone():[replacement]},"changeAnnotations":{"note":{"label":"Changes public API","needsConfirmation":true}}});
    let prepare_it = |w: &Value| prepare(w,&root.0,&[],&HashMap::new(),SystemTime::now());
    assert!(prepare_it(&workspace).unwrap().files[0].preview.contains("Changes public API"));
    workspace["changeAnnotations"]=json!({}); assert!(prepare_it(&workspace).is_err());
    workspace["changeAnnotations"]=json!({"note":{"label":"Note"}});
    workspace["changes"][&uri][0]["insertTextFormat"]=json!(2); assert!(prepare_it(&workspace).is_err());
}

#[test]
fn resource_undo_waits_for_newer_edits_in_both_copies_of_an_open_buffer() {
    let (root, paths) = setup(); let mut active=editor(&paths[0]); let mut other=editor(&paths[0]);
    let dest=root.0.join("new.rs");
    let preview=prepare(&json!({"documentChanges":[
        {"kind":"create","uri":lsp::file_uri(&dest).unwrap()},
        {"textDocument":{"uri":lsp::file_uri(&paths[0]).unwrap(),"version":null},"edits":[edit(0,3,"new")]}
    ]}),&root.0,&[lsp::Document {path:paths[0].clone(),text:text(&active)}],&HashMap::new(),SystemTime::now()).unwrap();
    apply(Arc::new(preview),&mut active,&mut other).unwrap();
    other.insert_text("later ").unwrap();
    assert!(history(&mut active,&mut other,false).is_err());
    assert!(dest.exists()); assert_eq!(text(&active),"new();\n");
    other.undo().unwrap(); history(&mut active,&mut other,false).unwrap();
    assert!(!dest.exists()); assert_eq!(text(&active),"old();\n"); assert_eq!(text(&other),"old();\n");
}

#[test]
fn rename_treats_shared_views_as_one_buffer_and_undo_works_from_either_view() {
    let (root, paths) = setup();
    let mut a = editor(&paths[0]);
    let mut b = a.duplicate_view();
    let record = Arc::new(prepare_all(&root.0, &paths, &[&a]));
    apply(record, &mut a, &mut b).unwrap();
    assert_eq!(text(&a), "renamed();\n");
    assert_eq!(text(&a), text(&b));
    assert_eq!(a.undo_stack.len(), 1);
    assert_eq!(b.undo_stack.len(), 1);
    assert!(history(&mut b, &mut a, false).unwrap());
    assert_eq!(text(&a), "old();\n");
    assert_eq!(text(&a), text(&b));
    assert!(history(&mut a, &mut b, true).unwrap());
    assert_eq!(text(&a), "renamed();\n");
    assert_eq!(text(&a), text(&b));
}

#[test]
fn shared_views_save_as_together_and_replacing_one_view_keeps_the_other_editable() {
    let (root, paths) = setup();
    let mut a = editor(&paths[0]);
    a.insert_text("draft ").unwrap();
    let mut b = a.duplicate_view();
    let destination = root.0.join("saved.rs");
    b.save_as(destination.to_str().unwrap(), false).unwrap();
    Editor::synchronize_views(&mut b, &mut a).unwrap();
    assert_eq!(a.path, b.path);
    assert!(!a.dirty && !b.dirty);
    assert_eq!(fs::read_to_string(&destination).unwrap(), "draft old();\n");
    a.open(paths[1].to_str().unwrap()).unwrap();
    assert!(!a.shares_document_with(&b));
    b.insert_text("more ").unwrap();
    Editor::synchronize_views(&mut b, &mut a).unwrap();
    assert_eq!(text(&a), "old();\n");
    assert_eq!(text(&b), "draft more old();\n");
    b.save().unwrap();
    assert_eq!(fs::read_to_string(&destination).unwrap(), text(&b));
}

#[test]
fn moving_a_file_updates_both_shared_views_and_undo_restores_both_paths() {
    let (root, paths) = setup();
    let dest = root.0.join("renamed.rs");
    let mut a = editor(&paths[0]);
    a.insert_text("draft ").unwrap();
    let mut b = a.duplicate_view();
    let before = text(&a);
    let preview = prepare(&json!({"documentChanges":[{"kind":"rename",
        "oldUri":lsp::file_uri(&paths[0]).unwrap(),"newUri":lsp::file_uri(&dest).unwrap()}]}),
        &root.0, &[lsp::Document { path: paths[0].clone(), text: before.clone() }],
        &HashMap::new(), SystemTime::now()).unwrap();
    apply(Arc::new(preview), &mut a, &mut b).unwrap();
    assert_eq!(a.path.as_ref(), Some(&dest.canonicalize().unwrap()));
    assert_eq!(a.path, b.path);
    assert_eq!(text(&a), before);
    assert_eq!(text(&b), before);
    assert!(history(&mut b, &mut a, false).unwrap());
    assert_eq!(a.path.as_ref(), Some(&paths[0].canonicalize().unwrap()));
    assert_eq!(a.path, b.path);
}
