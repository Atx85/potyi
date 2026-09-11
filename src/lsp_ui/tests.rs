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
