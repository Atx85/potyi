use super::*;
use crate::config::EditorConfig;

fn editor(text: &str) -> Editor {
    let mut editor = Editor::new(EditorConfig::default()).unwrap();
    editor.document.insert(0, text).unwrap();
    editor
}

#[test]
fn pending_results_are_invalid_after_edits_reopens_cursor_moves_or_dismissal() {
    let mut editor = editor("hello");
    let other = self::editor("other");
    let mut bar = CommandBar::new();
    bar.open(":hover");
    let pending = Pending {
        id: 1,
        revision: editor.document.revision(),
        other_revision: other.document.revision(),
        path: editor.path.clone(),
        cursor: 0,
        epoch: bar.epoch(),
    };
    assert!(pending.matches(&editor, &other, &bar));
    editor.document.move_cursor(1).unwrap();
    assert!(!pending.matches(&editor, &other, &bar));
    editor.document.move_cursor(0).unwrap();
    bar.close();
    bar.open(":hover");
    assert!(!pending.matches(&editor, &other, &bar));
    bar.epoch();
    let old = editor.document.revision();
    editor.insert_text("x").unwrap();
    assert_ne!(old, editor.document.revision());
    let changed = editor.document.revision();
    editor.undo().unwrap();
    assert_ne!(changed, editor.document.revision());
    assert_ne!(old, editor.document.revision());
    let undone = editor.document.revision();
    editor.redo().unwrap();
    assert_ne!(undone, editor.document.revision());
}

#[test]
fn batch_formatting_and_replacing_a_document_invalidate_pending_results() {
    let mut active = editor("original\n");
    let mut other = editor("other");
    let mut bar = CommandBar::new();
    bar.open(":definition");
    let pending = Pending {
        id: 1,
        revision: active.document.revision(),
        other_revision: other.document.revision(),
        path: active.path.clone(),
        cursor: active.document.cursor.position,
        epoch: bar.epoch(),
    };
    assert!(pending.matches(&active, &other, &bar));
    active
        .apply_formatted(&mut std::io::Cursor::new(b"formatted\n"))
        .unwrap();
    assert!(!pending.matches(&active, &other, &bar));
    active.undo().unwrap();
    assert_eq!(active.document.text().unwrap(), "original\n");
    assert!(!pending.matches(&active, &other, &bar));
    let pending = Pending {
        revision: active.document.revision(),
        ..pending
    };
    assert!(pending.matches(&active, &other, &bar));
    other = editor("other");
    assert!(!pending.matches(&active, &other, &bar));
}

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "potyi-lsp-ui-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        Self(directory)
    }
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[cfg(unix)]
#[ignore = "SDL integration with a temporary working directory; run with --ignored --test-threads=1"]
fn hover_clicks_refresh_only_the_latest_target_and_respect_dismissal() {
    use std::time::{Duration, Instant};
    struct WorkingDirectory(PathBuf);
    impl Drop for WorkingDirectory {
        fn drop(&mut self) { std::env::set_current_dir(&self.0).unwrap(); }
    }
    let files = Files::new();
    std::fs::create_dir(files.0.join("config")).unwrap();
    let fixture = format!("{}/tests/fixtures/lsp_server.py", env!("CARGO_MANIFEST_DIR"));
    files.write("config/lsp.toml", &format!(
        "enabled = true\n[[servers]]\nname = 'mock'\ncommand = 'python3'\nargs = ['{fixture}', 'incremental']\nextensions = ['rs']\nlanguage_id = 'rust'\n"));
    let path = files.write("main.rs", "one two three");
    let _cwd = WorkingDirectory(std::env::current_dir().unwrap());
    std::env::set_current_dir(&files.0).unwrap();
    let sdl = sdl3::init().unwrap();
    let events = sdl.event().unwrap();
    events.register_custom_event::<lsp::Event>().unwrap();
    let mut pump = sdl.event_pump().unwrap();
    let mut ui = LspUi::new(events);
    let mut active = editor("");
    active.open(path.to_str().unwrap()).unwrap();
    let mut other = editor("");
    let mut bar = CommandBar::new();
    bar.open(":hover");
    ui.document_clicked(&active, &other, &mut bar);
    assert!(ui.pending.is_none(), "typing a command must not enable click inspection");
    ui.request(lsp::Action::Hover, &active, &other, &mut bar).unwrap();
    let first = ui.pending.as_ref().unwrap().id;
    for position in [4, 2, 8] {
        active.document.move_cursor(position).unwrap();
        ui.document_clicked(&active, &other, &mut bar);
    }
    assert_eq!(ui.next_id, first + 1, "clicks must not queue server jobs while busy");
    assert_eq!(ui.hover_refresh.as_ref().unwrap().cursor, 8);
    let mut drain = |ui: &mut LspUi, active: &mut Editor, other: &mut Editor, bar: &mut CommandBar| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while ui.pending.is_some() {
            assert!(Instant::now() < deadline, "hover result timed out");
            if let Some(event) = pump.wait_event_timeout(Duration::from_millis(100))
                && let Some(event) = event.as_user_event_type::<lsp::Event>()
            {
                ui.accept(event, active, other, bar);
            }
        }
    };
    drain(&mut ui, &mut active, &mut other, &mut bar);
    assert_eq!(ui.next_id, first + 2, "only the latest click should create a follow-up request");
    let result: String = (0..bar.suggestion_count()).map(|i| bar.suggestion(i).unwrap().label).collect();
    let result: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(result["position"]["character"], 8);
    assert!(bar.is_active());

    active.document.move_cursor(4).unwrap();
    ui.document_clicked(&active, &other, &mut bar);
    assert_eq!(ui.pending.as_ref().unwrap().cursor, 4, "completed hover must refresh on a click");
    active.document.move_cursor(0).unwrap();
    ui.document_clicked(&active, &other, &mut bar);
    let next_id = ui.next_id;
    bar.close();
    drain(&mut ui, &mut active, &mut other, &mut bar);
    assert!(!bar.is_active(), "a late reply must not reopen a dismissed hover");
    assert_eq!(ui.next_id, next_id, "dismissal must discard the queued click");
    assert!(ui.hover_refresh.is_none());
    bar.open(":definition");
    ui.request(lsp::Action::Definition, &active, &other, &mut bar).unwrap();
    active.document.move_cursor(5).unwrap();
    ui.document_clicked(&active, &other, &mut bar);
    drain(&mut ui, &mut active, &mut other, &mut bar);
    assert_eq!(active.document.cursor.position, 5, "a stale definition must not jump away from a click");
    assert!(bar.suggestion(0).unwrap().label.contains("Cursor moved"));
    assert!(bar.prepare_execute());
    ui.request(lsp::Action::Definition, &active, &other, &mut bar).unwrap();
    drain(&mut ui, &mut active, &mut other, &mut bar);
    assert_eq!(active.document.cursor.position, 2);
    assert!(!bar.is_active());
    ui.stop();
}

#[test]
fn navigation_preserves_both_dirty_panes_and_existing_undo_history() {
    let files = Files::new();
    let path = files.write("existing.rs", "a🦀b\n");
    let new = files.write("new.rs", "new\n");
    let mut active = editor("unsaved");
    active.dirty = true;
    let mut other = editor("");
    other.open(path.to_str().unwrap()).unwrap();
    other.document.move_cursor(other.document.len()).unwrap();
    other.insert_text("edit").unwrap();
    let history = other.undo_stack.len();
    other.read_only = true;
    let result = navigate(
        &lsp::Location {
            path: path.clone(),
            line: 0,
            character: 3,
        },
        &mut active,
        &mut other,
    )
    .unwrap();
    assert!(result.focus_other);
    assert!(!result.document_reloaded);
    assert_eq!(other.document.cursor.column, 2);
    assert_eq!(other.undo_stack.len(), history);
    assert!(other.dirty);
    assert!(other.read_only);
    assert_eq!(active.document.text().unwrap(), "unsaved");
    assert!(
        navigate(
            &lsp::Location {
                path: new,
                line: 0,
                character: 0
            },
            &mut active,
            &mut other
        )
        .is_err()
    );
    assert_eq!(other.path, Some(path));
}

#[test]
fn invalid_definition_does_not_replace_a_clean_document() {
    let files = Files::new();
    let path = files.write("new.rs", "a🦀b");
    let mut active = editor("original");
    let mut other = editor("other");
    for (line, character) in [(100, 0), (0, 2), (0, 100)] {
        assert!(
            navigate(
                &lsp::Location {
                    path: path.clone(),
                    line,
                    character
                },
                &mut active,
                &mut other
            )
            .is_err()
        );
        assert_eq!(active.document.text().unwrap(), "original");
        assert_eq!(other.document.text().unwrap(), "other");
    }
}

#[test]
#[ignore = "requires SDL; run with SDL_VIDEODRIVER=dummy and --test-threads=1"]
fn rename_reply_preview_click_enter_apply_and_dismissal() {
    let files = Files::new();
    let path = files.write("main.rs", "old();\n").canonicalize().unwrap();
    let sdl = sdl3::init().unwrap();
    let mut ui = LspUi::new(sdl.event().unwrap());
    let mut active = editor(""); active.open(path.to_str().unwrap()).unwrap();
    let mut other = editor(""); let mut bar = CommandBar::new();
    let make_preview = || crate::workspace_edit::prepare(&serde_json::json!({"changes": {
        lsp::file_uri(&path).unwrap(): [{"range":{"start":{"line":0,"character":0},
            "end":{"line":0,"character":3}},"newText":"renamed"}]
    }}),&files.0,&[lsp::Document { path:path.clone(), text:"old();\n".into() }],
        &std::collections::HashMap::new(),std::time::SystemTime::now()).unwrap();
    bar.open(":rename renamed");
    ui.pending = Some(Pending { id:42,revision:active.document.revision(),other_revision:other.document.revision(),
        cursor:active.document.cursor.position,path:active.path.clone(),epoch:bar.epoch() });
    ui.accept(lsp::Event { id:42,result:Ok(lsp::Reply::Rename(make_preview())) }, &mut active,&mut other,&mut bar);
    assert!(!bar.is_info()); assert!(ui.preview.is_some());
    bar.select_suggestion(0); assert!(!ui.review(&mut active,&mut other,&mut bar).unwrap().unwrap());
    assert!(bar.is_info()); assert_eq!(active.document.text().unwrap(),"old();\n");
    assert!(bar.prepare_execute()); assert!(!ui.review(&mut active,&mut other,&mut bar).unwrap().unwrap());
    assert!(!bar.is_info());
    bar.select_suggestion(1); assert!(ui.review(&mut active,&mut other,&mut bar).unwrap().unwrap());
    assert_eq!(active.document.text().unwrap(),"renamed();\n"); assert!(!bar.is_active());
    assert_eq!(std::fs::read_to_string(&path).unwrap(),"old();\n");
    crate::workspace_edit::history(&mut active,&mut other,false).unwrap();
    bar.open(":rename renamed"); let preview = make_preview(); bar.show_review(preview.choices());
    ui.preview = Some((bar.epoch(),std::sync::Arc::new(preview)));
    bar.close(); ui.discard_dismissed_preview(&bar); assert!(ui.preview.is_none());
    assert_eq!(active.document.text().unwrap(),"old();\n");
}

#[test]
#[ignore = "requires SDL; run with SDL_VIDEODRIVER=dummy and --test-threads=1"]
fn action_picker_click_enter_cancel_and_selection_invalidation() {
    let sdl = sdl3::init().unwrap();
    let mut ui = LspUi::new(sdl.event().unwrap());
    let mut active = editor("Unknown"); let mut other = editor("");
    let mut bar = CommandBar::new();
    let prepare = |ui: &mut LspUi, bar: &mut CommandBar, active: &mut Editor, other: &mut Editor| {
        bar.open(":actions");
        ui.pending = Some(Pending { id:1, revision:active.document.revision(), other_revision:other.document.revision(),
            cursor:active.document.cursor.position, path:active.path.clone(), epoch:bar.epoch() });
        ui.accept(lsp::Event {id:1, result:Ok(lsp::Reply::CodeActions(lsp::CodeActions {
            started:std::time::SystemTime::now(), items:vec![lsp::CodeActionItem {
                title:"Unavailable fix".into(), disabled:Some("Needs a server extension".into()), value:serde_json::json!({})
            }]
        }))}, active, other, bar);
    };
    prepare(&mut ui,&mut bar,&mut active,&mut other);
    assert!(ui.actions.is_some()); assert!(!bar.is_info());
    bar.select_suggestion(0); ui.review(&mut active,&mut other,&mut bar).unwrap().unwrap();
    assert!(bar.is_info()); assert!(bar.prepare_execute());
    ui.review(&mut active,&mut other,&mut bar).unwrap().unwrap(); assert!(!bar.is_info());
    bar.select_suggestion(1); ui.review(&mut active,&mut other,&mut bar).unwrap().unwrap();
    assert!(!bar.is_active()); assert!(ui.actions.is_none());
    prepare(&mut ui,&mut bar,&mut active,&mut other);
    active.document.cursor.anchor = 1;
    ui.review(&mut active,&mut other,&mut bar).unwrap().unwrap();
    assert!(ui.actions.is_none()); assert!(bar.is_info());
    assert_eq!(active.document.text().unwrap(),"Unknown");
    prepare(&mut ui,&mut bar,&mut active,&mut other);
    bar.close(); ui.discard_dismissed_preview(&bar); assert!(ui.actions.is_none());
}
