// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;

#[test]
fn formatting_preserves_selection_and_is_one_undo_step() {
    let mut editor = test_editor("fn main(){\nlet café=1;\n}\n");
    editor.set_cursor_and_anchor(17, 11).unwrap();
    let before = editor.cursor_state();
    let original = editor.document.text().unwrap();
    let formatted = "fn main() {\n    let café = 1;\n}\n";
    assert!(editor.apply_formatted(&mut io::Cursor::new(formatted.as_bytes())).unwrap());
    assert_eq!(editor.undo_stack.len(), 1);
    assert!(editor.dirty);
    assert_eq!(editor.document.cursor.line, before.line);
    assert_eq!(editor.document.cursor.column, before.column);
    assert_eq!(editor.document.cursor.anchor_line, before.anchor_line);
    assert_eq!(editor.document.cursor.anchor_column, before.anchor_column);
    editor.undo().unwrap();
    assert_eq!(editor.document.text().unwrap(), original);
    assert_eq!(editor.document.cursor.position, before.position);
    assert_eq!(editor.document.cursor.anchor, before.anchor);
    editor.redo().unwrap();
    assert_eq!(editor.document.text().unwrap(), formatted);
}

#[test]
fn formatting_noop_preserves_redo_dirty_and_cursor() {
    let mut editor = test_editor("original\n");
    editor.insert_text("x").unwrap();
    editor.undo().unwrap();
    editor.dirty = false;
    let before = editor.cursor_state();
    assert!(!editor.apply_formatted(&mut io::Cursor::new(b"original\n")).unwrap());
    assert!(!editor.dirty);
    assert_eq!(editor.undo_stack.len(), 0);
    assert_eq!(editor.redo_stack.len(), 1);
    assert_eq!(editor.document.cursor.position, before.position);
    editor.redo().unwrap();
    assert!(editor.document.text().unwrap().contains('x'));
}

#[test]
fn formatting_rejects_readonly_empty_and_invalid_output_without_changes() {
    let mut editor = test_editor("original");
    for bytes in [vec![], vec![0xff], vec![0], vec![b'x'; formatting::MAX_OUTPUT_BYTES + 1]] {
        assert!(editor.apply_formatted(&mut io::Cursor::new(bytes)).is_err());
        assert_eq!(editor.document.text().unwrap(), "original");
        assert!(editor.undo_stack.is_empty());
        assert!(!editor.dirty);
    }
    editor.read_only = true;
    assert!(editor.apply_formatted(&mut io::Cursor::new(b"changed")).is_err());
    assert!(editor.format_document(Some("rustfmt")).is_err());
    assert_eq!(editor.document.text().unwrap(), "original");
}

#[test]
fn formatting_only_changes_the_target_pane_and_preserves_older_history() {
    let mut editor = test_editor("one");
    let other = test_editor("two");
    editor.insert_text("x").unwrap();
    let typed = editor.document.text().unwrap();
    assert!(editor.apply_formatted(&mut io::Cursor::new(b"formatted")).unwrap());
    assert_eq!(other.document.text().unwrap(), "two");
    editor.undo().unwrap();
    assert_eq!(editor.document.text().unwrap(), typed);
    editor.undo().unwrap();
    assert_eq!(editor.document.text().unwrap(), "one");
}

use std::time::{SystemTime, UNIX_EPOCH};
use crate::config::{EditorConfig, LineNumberMode};
use crate::search::SearchMode;

// ==========================================================================
// Test helpers
// ==========================================================================
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

fn test_file(contents: &[u8]) -> PathBuf {
    let mut path = std::env::temp_dir();

    path.push(format!(
        "potyi_test_{}.txt",
        std::process::id()
    ));

    let mut file = File::create(&path).unwrap();
    file.write_all(contents).unwrap();

    path
}

fn empty_table() -> PieceTable {
    let mut path = std::env::temp_dir();

    path.push(format!(
        "potyi_test_{}.txt",
        std::process::id()
    ));

    File::create(&path).unwrap();

    let path_string =
        path.to_string_lossy().to_string();

    PieceTable::open(&path_string).unwrap()
}
fn temporary_path() -> std::path::PathBuf {
    let temp_dir = std::env::temp_dir();

    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    temp_dir.join(
        format!("potyi_test_{}.txt", id)
    )
}

#[test]
fn new_document_creates_named_file_and_saves_edits_to_it() {
    let path = temporary_path().with_extension("é new.txt");
    let mut editor = test_editor("old document");
    editor.read_only = true;
    editor.new_document(Some(path.to_str().unwrap())).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"");
    assert_eq!(editor.document.text().unwrap(), "");
    assert_eq!(editor.path.as_ref(), Some(&path));
    assert!(!editor.dirty);
    assert!(!editor.read_only);
    assert!(editor.undo_stack.is_empty());
    assert!(editor.redo_stack.is_empty());
    editor.insert_text("new contents").unwrap();
    editor.save().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "new contents");
    editor.new_document(None).unwrap();
    assert!(editor.path.is_none());
    assert_eq!(editor.document.len(), 0);
    assert_eq!(editor.document.cursor.position, 0);
    assert!(editor.undo_stack.is_empty());
    fs::remove_file(path).unwrap();
}

#[test]
fn new_document_preserves_unsaved_edits_and_existing_destinations() {
    let path = temporary_path();
    let mut editor = test_editor("original");
    editor.insert_text("unsaved ").unwrap();
    let before = editor.document.text().unwrap();
    let cursor = editor.cursor_state();
    let history = editor.undo_stack.len();
    assert!(editor.new_document(Some(path.to_str().unwrap())).is_err());
    assert!(editor.new_document(None).is_err());
    assert!(!path.exists());
    assert_eq!(editor.document.text().unwrap(), before);
    assert_eq!(editor.document.cursor.position, cursor.position);
    assert_eq!(editor.undo_stack.len(), history);
    editor.dirty = false;
    fs::write(&path, "existing contents").unwrap();
    assert_eq!(editor.new_document(Some(path.to_str().unwrap())).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read_to_string(&path).unwrap(), "existing contents");
    assert_eq!(editor.document.text().unwrap(), before);
    assert!(editor.new_document(Some(path.join("missing/file.txt").to_str().unwrap())).is_err());
    assert_eq!(editor.document.text().unwrap(), before);
    fs::remove_file(path).unwrap();
}

#[test]
fn save_as_preserves_original_cursor_history_and_uses_new_path_for_later_saves() {
    let source = temporary_path();
    let target = temporary_path().with_extension("é saved.rs");
    fs::write(&source, "original").unwrap();
    let mut editor = test_editor("");
    editor.open(source.to_str().unwrap()).unwrap();
    editor.insert("new ").unwrap();
    editor.document.select_right().unwrap();
    let cursor = editor.cursor_state();
    let history = editor.undo_stack.len();

    editor.save_as(target.to_str().unwrap(), false).unwrap();
    assert_eq!(fs::read_to_string(&source).unwrap(), "original");
    assert_eq!(fs::read_to_string(&target).unwrap(), "new original");
    assert_eq!(editor.path.as_ref(), Some(&target));
    assert!(!editor.dirty);
    assert_eq!(editor.cursor_state().position, cursor.position);
    assert_eq!(editor.cursor_state().anchor, cursor.anchor);
    assert_eq!(editor.undo_stack.len(), history);

    editor.undo().unwrap();
    assert_eq!(editor.document.text().unwrap(), "original");
    editor.save().unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    editor.redo().unwrap();
    assert_eq!(editor.document.text().unwrap(), "new original");
    assert_eq!(fs::read_to_string(&source).unwrap(), "original");
    fs::remove_file(source).unwrap();
    fs::remove_file(target).unwrap();
}

#[test]
fn save_as_failure_preserves_document_and_requires_explicit_overwrite() {
    let target = temporary_path();
    fs::write(&target, "existing").unwrap();
    let mut editor = test_editor("");
    editor.insert("draft").unwrap();
    for destination in [target.clone(), temporary_path().join("missing.txt")] {
        assert!(editor.save_as(destination.to_str().unwrap(), false).is_err());
        assert!(editor.path.is_none());
        assert!(editor.dirty);
        assert_eq!(editor.document.text().unwrap(), "draft");
        assert_eq!(editor.undo_stack.len(), 1);
    }
    assert_eq!(fs::read_to_string(&target).unwrap(), "existing");
    editor.save_as(target.to_str().unwrap(), true).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "draft");
    // Saving to the active filename is also allowed without a bang.
    editor.save_as(target.to_str().unwrap(), false).unwrap();
    fs::remove_file(target).unwrap();
}

#[test]
fn save_as_protects_other_pane_and_read_only_documents() {
    let target = temporary_path();
    fs::write(&target, "other pane").unwrap();
    let mut other = test_editor("");
    other.open(target.to_str().unwrap()).unwrap();
    let mut editor = test_editor("draft");
    assert!(save_as_in_pane(&mut editor, &other, target.to_str().unwrap(), true).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "other pane");
    assert!(editor.path.is_none());

    editor.read_only = true;
    assert!(editor.save_as(target.to_str().unwrap(), true).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "other pane");
    fs::remove_file(target).unwrap();
}

#[test]
fn goto_respects_line_number_mode_and_overrides() {
    for (configured, mode, start, label, expected) in [
        (LineNumberMode::Normal, GotoMode::Automatic, 12, 2, 2),
        (LineNumberMode::Relative, GotoMode::Automatic, 12, 2, 10),
        (LineNumberMode::Dynamic, GotoMode::Automatic, 12, 2, 10),
        (LineNumberMode::Dynamic, GotoMode::Automatic, 1, 2, 3),
        (LineNumberMode::Dynamic, GotoMode::Automatic, 12, 12, 12),
        (LineNumberMode::Relative, GotoMode::Automatic, 12, 0, 12),
        (LineNumberMode::Normal, GotoMode::Relative, 7, 2, 9),
        (LineNumberMode::Dynamic, GotoMode::Relative, 7, -2, 5),
        (LineNumberMode::Relative, GotoMode::Absolute, 12, 2, 2),
    ] {
        let mut editor = test_editor(&["text"; 12].join("\n"));
        editor.document.move_cursor_to_line_column(start - 1, 0).unwrap();
        assert_eq!(
            goto_line_index(&mut editor.document, label, mode, configured),
            Ok(expected - 1),
            "{configured:?}, {mode:?}, label {label}, from {start}",
        );
    }
}

#[test]
fn goto_dynamic_commands_move_to_expected_document_lines() {
    // These cases use one-based document lines, as a user sees them.
    for (input, start_line, expected_line, expected_column) in [
        (":goto 1", 10, 9, 1),
        (":goto +3", 7, 10, 1),
        (":goto -3", 7, 4, 1),
        (":goto +3:3", 7, 10, 3),
        (":goto +0", 7, 7, 1),
        (":goto -0", 7, 7, 1),
        (":goto -1", 7, 6, 1),
        (":goto -1", 8, 7, 1),
        (":goto 0 --rel", 7, 7, 1),
        (":goto 7", 7, 7, 1),
        (":goto 1 --abs", 7, 1, 1),
        (":goto 7 --abs", 7, 7, 1),
        (":goto 1 --rel", 7, 8, 1),
        (":goto -1:3", 7, 6, 3),
    ] {
        let mut editor = test_editor(
            &vec!["aé中z"; 10].join("\n")
        );
        editor.document
            .move_cursor_to_line_column(start_line - 1, 1)
            .unwrap();
        editor.document.select_right().unwrap();

        let mut bar = CommandBar::new();
        bar.open(input);
        let ParsedCommand::Goto { line, column, mode } =
            bar.parse().unwrap()
        else {
            panic!("Expected goto: {input}");
        };

        let destination = goto_line_index(
            &mut editor.document,
            line,
            mode,
            editor.config.line_numbers,
        ).unwrap();
        editor.document
            .move_cursor_to_line_column(
                destination,
                column.unwrap_or(1) - 1,
            )
            .unwrap();

        assert_eq!(
            (
                editor.document.cursor.line + 1,
                editor.document.cursor.column + 1,
            ),
            (expected_line, expected_column),
            "{input} from line {start_line}",
        );
        assert_eq!(
            editor.document.cursor.anchor,
            editor.document.cursor.position,
            "{input} should clear the selection",
        );
    }
}

#[test]
#[ignore = "Run with SDL_VIDEODRIVER=dummy and --ignored --test-threads=1"]
fn goto_executes_command_and_renders_destination() {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video.window("goto test", 800, 600)
        .hidden().build().unwrap();
    let ttf = sdl3::ttf::init().unwrap();
    let font = || {
        ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(FONT_DATA).unwrap(),
            18.0,
        ).unwrap()
    };
    let canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();
    let mut renderer = Renderer::new(
        canvas, &texture_creator, font(), font(), 18.0, (font(), font()),
        window::WindowHitTestState::new(800, 1.0),
    ).unwrap();
    let mut other = test_editor("");
    let mut search = SearchUi::new();
    let mut bar = CommandBar::new();
    let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
    let mut formatting_vim = VimController::new();
    let mut lsp_ui = lsp_ui::LspUi::new(sdl.event().unwrap());

    for configured in [
        LineNumberMode::Normal,
        LineNumberMode::Relative,
        LineNumberMode::Dynamic,
    ] {
        for (input, line_count, expected_line, expected_column) in [
            (":goto +3", 40, 10, 1),
            (":goto -3", 40, 4, 1),
            (":goto +3:3", 40, 10, 3),
            (":goto +0", 40, 7, 1),
            (":goto -0", 40, 7, 1),
            (":goto 3 --rel", 40, 10, 1),
            (":goto 3 --abs", 40, 3, 1),
            (":goto +3 --abs", 40, 3, 1),
        ] {
            let mut editor = test_editor("");
            editor.config.line_numbers = configured;
            renderer.set_line_number_mode(configured);
            editor.document = test_table(
                &(1..=line_count).map(|line| format!("Line {line}"))
                    .collect::<Vec<_>>().join("\n")
            );
            editor.document.move_cursor_to_line_column(6, 0).unwrap();
            renderer.ensure_cursor_visible(&mut editor.document);
            bar.open(":");
            bar.insert_text(&input[1..]);
            sync_command_search(&bar, &mut search, &mut editor.document, None).unwrap();
            renderer.render(&mut editor.document, &search, &bar).unwrap();

            let outcome = execute_command_bar(
                &mut bar, &mut search, &mut editor, &mut other,
                &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
            );
            assert!(outcome.cursor_changed, "{input}: {:?}", bar.status());
            assert!(!bar.is_active(), "{input}: {:?}", bar.status());
            renderer.ensure_cursor_visible(&mut editor.document);
            renderer.render(&mut editor.document, &search, &bar).unwrap();
            assert_eq!(editor.document.cursor.line + 1, expected_line, "{input}");
            assert_eq!(editor.document.cursor.column + 1, expected_column, "{input}");
        }

        // Unsigned numbers retain their label meaning, even though +3 and -3
        // explicitly choose a direction. Failed commands must preserve selection.
        let mut editor = test_editor(&["text"; 40].join("\n"));
        editor.config.line_numbers = configured;
        editor.document.move_cursor_to_line_column(6, 0).unwrap();
        editor.document.select_right().unwrap();
        let original_cursor = (editor.document.cursor.position, editor.document.cursor.anchor);
        bar.open(":goto 3");
        let outcome = execute_command_bar(
            &mut bar, &mut search, &mut editor, &mut other,
            &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
        );
        if configured == LineNumberMode::Normal {
            assert!(outcome.cursor_changed);
            assert!(!bar.is_active());
            assert_eq!(editor.document.cursor.line + 1, 3);
        } else {
            assert!(!outcome.cursor_changed);
            assert!(bar.is_active());
            let status = bar.status().unwrap();
            assert!(status.contains(":goto -3") && status.contains(":goto +3"));
            assert_eq!(
                (editor.document.cursor.position, editor.document.cursor.anchor),
                original_cursor,
            );
        }
    }

    // Reproduce the screenshot: the cursor is on the empty twelfth line.
    let mut editor = test_editor("a\nsdf\nasdf\nas\ndf\nasdf\nas\n\nsdaf\nas\ndf\n");
    editor.document.move_cursor_to_line_column(11, 0).unwrap();
    let original_position = editor.document.cursor.position;
    for input in [":goto +2", ":goto 2 --rel", ":goto 14 --abs"] {
        bar.open(input);
        let outcome = execute_command_bar(
            &mut bar, &mut search, &mut editor, &mut other,
            &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
        );
        assert!(!outcome.cursor_changed, "{input}");
        assert!(bar.is_active(), "{input}");
        assert_eq!(bar.status(), Some("Line 14 is past the last line (12)."));
        assert_eq!(editor.document.cursor.position, original_position);
        renderer.render(&mut editor.document, &search, &bar).unwrap();
    }

    bar.open(":goto 2");
    let outcome = execute_command_bar(
        &mut bar, &mut search, &mut editor, &mut other,
        &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
    );
    assert!(outcome.cursor_changed);
    assert!(!bar.is_active());
    assert_eq!(editor.document.cursor.line + 1, 10);
    assert_eq!(editor.document.cursor.column, 0);

    // Information rows use bounded rendering and do not modify the document.
    let original = editor.document.text().unwrap();
    bar.open(":hover");
    bar.show_info(&"Unicode help: café 🦀 and a deliberately long signature. ".repeat(100));
    renderer.render(&mut editor.document, &search, &bar).unwrap();
    bar.move_selection(1);
    renderer.render(&mut editor.document, &search, &bar).unwrap();
    assert_eq!(editor.document.text().unwrap(), original);
    bar.close();
    renderer.render(&mut editor.document, &search, &bar).unwrap();

    bar.open(":hover");
    let outcome = execute_command_bar(
        &mut bar, &mut search, &mut editor, &mut other,
        &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
    );
    assert!(!outcome.document_changed);
    assert!(!outcome.cursor_changed);
    assert!(bar.suggestion(0).unwrap().label.contains("disabled"));
    assert!(lsp_ui.status(&editor).contains("0 background session(s)"));

    // Selecting a command from a later page follows the same dispatch as Enter.
    bar.open(":");
    bar.scroll_suggestions(100);
    assert_eq!(bar.suggestion(bar.selected()).unwrap().label, "lsp");
    execute_command_bar(
        &mut bar, &mut search, &mut editor, &mut other,
        &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
    );
    assert_eq!(bar.input(), ":lsp ");
    bar.scroll_suggestions(100);
    assert_eq!(bar.suggestion(bar.selected()).unwrap().label, "lsp stop");
    execute_command_bar(
        &mut bar, &mut search, &mut editor, &mut other,
        &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
    );
    assert!(bar.suggestion(0).unwrap().label.contains("stopped"));

    editor = test_editor("MATCH");
    bar.open(":find match ");
    sync_command_search(&bar, &mut search, &mut editor.document, None).unwrap();
    assert!(search.current_match().is_none());
    bar.move_selection(1);
    execute_command_bar(
        &mut bar, &mut search, &mut editor, &mut other,
        &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
    );
    assert!(search.current_match().is_some());

    let created = temporary_path();
    bar.open(&format!(":new {}", quote_argument(created.to_str().unwrap())));
    let outcome = execute_command_bar(
        &mut bar, &mut search, &mut editor, &mut other,
        &mut renderer, &mut terminal, &mut formatting_vim, false, &mut VimController::new(), &mut lsp_ui,
    );
    assert!(outcome.document_reloaded && outcome.path_changed);
    assert_eq!(editor.path.as_ref(), Some(&created));
    assert_eq!(fs::read(&created).unwrap(), b"");
    fs::remove_file(created).unwrap();
}

#[test]
fn goto_rejects_invalid_absolute_and_relative_lines() {
    let mut editor = test_editor(&["text"; 12].join("\n"));
    editor.document.move_cursor_to_line_column(3, 0).unwrap();
    assert_eq!(
        goto_line_index(
            &mut editor.document,
            0,
            GotoMode::Absolute,
            LineNumberMode::Relative,
        )
        .unwrap_err(),
        "line numbers start at 1",
    );

    assert_eq!(
        goto_line_index(
            &mut editor.document,
            -4,
            GotoMode::Relative,
            LineNumberMode::Normal,
        )
        .unwrap_err(),
        "Relative line is before the start of the document",
    );

    for configured in [LineNumberMode::Relative, LineNumberMode::Dynamic] {
        assert!(goto_line_index(
            &mut editor.document, 2, GotoMode::Automatic, configured,
        ).unwrap_err().contains("matches multiple lines"));
        assert_eq!(goto_line_index(
            &mut editor.document, 99, GotoMode::Automatic, configured,
        ).unwrap_err(), "No line is labelled 99.");
    }
    assert_eq!(goto_line_index(
        &mut editor.document, 0, GotoMode::Automatic, LineNumberMode::Dynamic,
    ).unwrap_err(), "No line is labelled 0.");
    assert_eq!(editor.document.cursor.line, 3);
}

#[test]
fn insert_hello() {
    let mut table = empty_table();

    table.insert(0, "hello").unwrap();

    assert_eq!(
        table.text().unwrap(),
        "hello"
    );
}

#[test]
fn insert_space_after_hello() {
    let mut table = empty_table();

    table.insert(0, "hello").unwrap();
    table.insert(5, " ").unwrap();

    assert_eq!(
        table.text().unwrap(),
        "hello "
    );
}
#[test]
fn typing_hello_then_space() {
    let mut table = empty_table();

    let position = table.cursor.position;

    table.insert(position, "hello").unwrap();

    table
        .move_cursor(position + "hello".len())
        .unwrap();

    assert_eq!(
        table.cursor.position,
        5
    );

    let position = table.cursor.position;

    table.insert(position, " ").unwrap();

    table
        .move_cursor(position + " ".len())
        .unwrap();

    assert_eq!(
        table.text().unwrap(),
        "hello "
    );

    assert_eq!(
        table.cursor.position,
        6
    );
}

#[test]
fn insert_utf8_then_space() {
    let mut table = empty_table();

    table.insert(0, "hello").unwrap();
    table.move_cursor(5).unwrap();

    table.insert(5, "é").unwrap();
    table.move_cursor(5 + "é".len()).unwrap();

    assert_eq!(
        table.text().unwrap(),
        "helloé"
    );

    assert_eq!(
        table.cursor.position,
        7
    );

    table.insert(7, " ").unwrap();
    table.move_cursor(8).unwrap();

    assert_eq!(
        table.text().unwrap(),
        "helloé "
    );
}

#[test]
fn editor_typing_hello_then_space() {
    let mut editor = Editor {
        document: empty_table(),
        path: None,
        config: EditorConfig {
            tab_width: 4,
            insert_spaces: false,
            line_numbers: LineNumberMode::Dynamic,
            font_size: 18,
            keybinding_mode:
                crate::config::KeybindingMode::Conventional,
        },
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        multi_edit_group: None,
        applying_history: false,
        dirty: false,
        read_only: false,
    };

    editor.insert_text("hello").unwrap();

    assert_eq!(
        editor.document.text().unwrap(),
        "hello"
    );

    assert_eq!(
        editor.document.cursor.position,
        5
    );

    editor.insert_text(" ").unwrap();

    assert_eq!(
        editor.document.text().unwrap(),
        "hello "
    );

    assert_eq!(
        editor.document.cursor.position,
        6
    );
}


/// Create a temporary source file and open it as a PieceTable.
fn test_table(text: &str) -> PieceTable {
    let path = temporary_path();

    fs::write(&path, text)
        .expect("failed to create temporary test file");

    let mut table = PieceTable::open(
        path.to_str().unwrap(),
    )
    .expect("failed to open temporary test file");

    // PieceTable owns the file handle.
    // On Windows removal may fail, which is harmless.
    let _ = fs::remove_file(&path);

    table
}


/// Read the complete logical document as UTF-8.
///
/// PieceTable positions are byte positions, so this deliberately
/// reconstructs the document from the piece table rather than relying
/// on a convenience `text()` method.
fn table_text(table: &PieceTable) -> String {
    let bytes = table
        .read_range(0, table.len())
        .expect("failed to read document");

    String::from_utf8(bytes)
        .expect("piece table contained invalid UTF-8")
}


/// Create an Editor backed by a temporary file.
///
/// The path is deliberately kept alive because the current Editor owns
/// the document file and also stores its path.
fn test_editor(text: &str) -> Editor {
    let mut editor = Editor::new(
        EditorConfig {
            tab_width: 4,
            insert_spaces: false,
            line_numbers: LineNumberMode::Dynamic,
            font_size: 18,
            keybinding_mode:
                crate::config::KeybindingMode::Conventional,
        },
    )
    .expect("failed to create test editor");

    editor
        .document
        .insert(0, text)
        .expect("failed to insert test document");

    editor
}

fn test_replace_ui(
    editor: &mut Editor,
    query: &str,
    replacement: &str,
    mode: SearchMode,
) -> SearchUi {
    let mut search = SearchUi::new();

    search
        .open_replace(&mut editor.document)
        .unwrap();
    search
        .set_mode(&mut editor.document, mode)
        .unwrap();
    search
        .set_query(&mut editor.document, query)
        .unwrap();
    search.focus_next_field(false);
    search
        .insert_focused_text(
            &mut editor.document,
            replacement,
        )
        .unwrap();

    search
}

// ==========================================================================
// Basic document tests
// ==========================================================================

#[test]
fn new_document_contains_original_text() {
    let mut table =
        test_table("Hello world");

    assert_eq!(
        table_text(&table),
        "Hello world"
    );
}


#[test]
fn empty_document() {
    let mut table =
        test_table("");

    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
    assert_eq!(table_text(&table), "");
}


#[test]
fn document_length_is_byte_length() {
    let mut table =
        test_table("Hello");

    assert_eq!(
        table.len(),
        5
    );
}


// ==========================================================================
// byte_at
// ==========================================================================

#[test]
fn byte_at_reads_original_document() {
    let mut table =
        test_table("Hello world");

    let expected =
        b"Hello world";

    for (index, byte) in expected.iter().enumerate() {
        assert_eq!(
            table.byte_at(index).unwrap(),
            Some(*byte)
        );
    }

    assert_eq!(
        table.byte_at(expected.len()).unwrap(),
        None
    );
}


#[test]
fn byte_at_reads_inserted_document() {
    let mut table =
        test_table("Hello world");

    table
        .insert(6, "beautiful ")
        .unwrap();

    let expected =
        b"Hello beautiful world";

    for (index, byte) in expected.iter().enumerate() {
        assert_eq!(
            table.byte_at(index).unwrap(),
            Some(*byte)
        );
    }

    assert_eq!(
        table.byte_at(expected.len()).unwrap(),
        None
    );
}


// ==========================================================================
// Piece-table insertion
// ==========================================================================

#[test]
fn insert_at_beginning() {
    let mut table =
        test_table("world");

    table
        .insert(0, "Hello ")
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello world"
    );
}


#[test]
fn insert_at_middle() {
    let mut table =
        test_table("Helo world");

    table
        .insert(2, "l")
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello world"
    );
}


#[test]
fn insert_at_end() {
    let mut table =
        test_table("Hello");

    table
        .insert(5, " world")
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello world"
    );
}


#[test]
fn multiple_inserts() {
    let mut table =
        test_table("world");

    table
        .insert(0, "Hello ")
        .unwrap();

    table
        .insert(11, "!")
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello world!"
    );
}


#[test]
fn insert_creates_three_pieces() {
    let mut table =
        test_table("Hello world");

    table
        .insert(6, "beautiful ")
        .unwrap();

    assert_eq!(
        table.pieces.len(),
        3
    );

    assert!(
        table.pieces[0].original
    );

    assert_eq!(
        table.pieces[0].start,
        0
    );

    assert_eq!(
        table.pieces[0].length,
        6
    );

    assert!(
        !table.pieces[1].original
    );

    assert_eq!(
        table.pieces[1].start,
        0
    );

    assert_eq!(
        table.pieces[1].length,
        10
    );

    assert!(
        table.pieces[2].original
    );

    assert_eq!(
        table.pieces[2].start,
        6
    );

    assert_eq!(
        table.pieces[2].length,
        5
    );
}


// ==========================================================================
// Reading across pieces
// ==========================================================================

#[test]
fn read_range_from_inserted_piece() {
    let mut table =
        test_table("Hello world");

    table
        .insert(6, "beautiful ")
        .unwrap();

    let bytes =
        table
            .read_range(6, 10)
            .unwrap();

    assert_eq!(
        bytes,
        b"beautiful "
    );
}


#[test]
fn read_range_spans_multiple_pieces() {
    let mut table =
        test_table("Hello world");

    table
        .insert(6, "beautiful ")
        .unwrap();

    let bytes =
        table
            .read_range(3, 15)
            .unwrap();

    assert_eq!(
        bytes,
        b"lo beautiful wo"
    );
}


// ==========================================================================
// Piece-table deletion
// ==========================================================================

#[test]
fn delete_at_beginning() {
    let mut table =
        test_table("Hello world");

    table
        .delete(0, 6)
        .unwrap();

    assert_eq!(
        table_text(&table),
        "world"
    );
}


#[test]
fn delete_at_end() {
    let mut table =
        test_table("Hello world");

    table
        .delete(5, 6)
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello"
    );
}


#[test]
fn delete_in_middle() {
    let mut table =
        test_table("Hello beautiful world");

    table
        .delete(6, 10)
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello world"
    );
}


#[test]
fn delete_across_pieces() {
    let mut table =
        test_table("Hello world");

    table
        .insert(6, "beautiful ")
        .unwrap();

    table
        .delete(6, 10)
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello world"
    );
}


#[test]
fn delete_entire_document() {
    let mut table =
        test_table("Hello world");

    table
        .delete(0, 11)
        .unwrap();

    assert_eq!(
        table_text(&table),
        ""
    );

    assert!(table.is_empty());
}


// ==========================================================================
// Insert / delete combinations
// ==========================================================================

#[test]
fn insert_then_delete() {
    let mut table =
        test_table("Hello world");

    table
        .insert(6, "beautiful ")
        .unwrap();

    table
        .delete(6, 10)
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello world"
    );
}


#[test]
fn insert_delete_insert() {
    let mut table =
        test_table("Hello world");

    table
        .insert(6, "beautiful ")
        .unwrap();

    table
        .delete(6, 10)
        .unwrap();

    table
        .insert(6, "cruel ")
        .unwrap();

    assert_eq!(
        table_text(&table),
        "Hello cruel world"
    );
}


// ==========================================================================
// Cursor movement
// ==========================================================================

#[test]
fn cursor_starts_at_zero() {
    let mut table =
        test_table("Hello world");

    assert_eq!(
        table.cursor.position,
        0
    );

    assert_eq!(
        table.cursor.anchor,
        0
    );
}


#[test]
fn cursor_moves_left_and_right() {
    let mut table =
        test_table("Hello world");

    table
        .move_cursor(6)
        .unwrap();

    assert_eq!(
        table.cursor.position,
        6
    );

    table
        .cursor_left()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        5
    );

    table
        .cursor_left()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        4
    );

    table
        .cursor_right()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        5
    );
}


#[test]
fn cursor_cannot_move_before_beginning() {
    let mut table =
        test_table("Hello");

    table
        .move_cursor(0)
        .unwrap();

    table
        .cursor_left()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        0
    );
}


#[test]
fn cursor_cannot_move_after_end() {
    let mut table =
        test_table("Hello");

    let end =
        table.len();

    table
        .move_cursor(end)
        .unwrap();

    table
        .cursor_right()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        end
    );
}


// ==========================================================================
// Cursor home / end
// ==========================================================================

#[test]
fn cursor_home_and_end() {
    let mut table =
        test_table(
            "Hello\nworld",
        );

    table
        .move_cursor(2)
        .unwrap();

    table
        .cursor_end()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        5
    );

    table
        .cursor_home()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        0
    );
}


#[test]
fn cursor_home_moves_to_current_line() {
    let mut table =
        test_table(
            "Hello\nworld",
        );

    table
        .move_cursor(8)
        .unwrap();

    table
        .cursor_home()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        6
    );
}


#[test]
fn cursor_end_moves_to_current_line() {
    let mut table =
        test_table(
            "Hello\nworld",
        );

    table
        .move_cursor(6)
        .unwrap();

    table
        .cursor_end()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        11
    );
}


// ==========================================================================
// Selection
// ==========================================================================

#[test]
fn selection_left_and_right() {
    let mut table =
        test_table("Hello world");

    table
        .move_cursor(6)
        .unwrap();

    table
        .select_right()
        .unwrap();

    table
        .select_right()
        .unwrap();

    assert_eq!(
        table.selection_start(),
        6
    );

    assert_eq!(
        table.selection_end(),
        8
    );

    assert!(
        table.has_selection()
    );

    table
        .select_left()
        .unwrap();

    assert_eq!(
        table.selection_start(),
        6
    );

    assert_eq!(
        table.selection_end(),
        7
    );
}


#[test]
fn selection_can_go_backwards() {
    let mut table =
        test_table("Hello world");

    table
        .move_cursor(6)
        .unwrap();

    table
        .select_left()
        .unwrap();

    table
        .select_left()
        .unwrap();

    assert_eq!(
        table.selection_start(),
        4
    );

    assert_eq!(
        table.selection_end(),
        6
    );

    assert!(
        table.has_selection()
    );
}


#[test]
fn normal_cursor_movement_clears_selection() {
    let mut table =
        test_table("Hello world");

    table
        .move_cursor(5)
        .unwrap();

    table
        .select_right()
        .unwrap();

    table
        .select_right()
        .unwrap();

    assert!(
        table.has_selection()
    );

    table
        .cursor_right()
        .unwrap();

    assert!(
        !table.has_selection()
    );

    assert_eq!(
        table.selection_start(),
        table.cursor.position
    );

    assert_eq!(
        table.selection_end(),
        table.cursor.position
    );
}


// ==========================================================================
// UTF-8 cursor tests
// ==========================================================================

#[test]
fn cursor_moves_over_multibyte_character() {
    let mut table =
        test_table("aébc");

    // UTF-8:
    //
    // a  = byte 0
    // é  = bytes 1..3
    // b  = byte 3
    // c  = byte 4

    table
        .move_cursor(1)
        .unwrap();

    table
        .cursor_right()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        3
    );

    table
        .cursor_right()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        4
    );

    table
        .cursor_left()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        3
    );

    table
        .cursor_left()
        .unwrap();

    assert_eq!(
        table.cursor.position,
        1
    );
}


#[test]
fn cursor_does_not_enter_middle_of_utf8_character() {
    let mut table =
        test_table("aé");

    // Position 2 is inside the two-byte
    // UTF-8 representation of é.

    let result =
        table.move_cursor(2);

    assert!(
        result.is_err()
    );

    assert_eq!(
        table.cursor.position,
        0
    );
}


#[test]
fn delete_removes_entire_utf8_character() {
    let mut table =
        test_table("aéb");

    table
        .move_cursor(1)
        .unwrap();

    table
        .delete(
            1,
            2,
        )
        .unwrap();

    assert_eq!(
        table_text(&table),
        "ab"
    );
}


// ==========================================================================
// Line / column tests
// ==========================================================================

#[test]
fn line_count_for_single_line() {
    let mut table =
        test_table("Hello");

    assert_eq!(
        table.line_count().unwrap(),
        1
    );
}


#[test]
fn line_count_for_multiple_lines() {
    let mut table =
        test_table(
            "Hello\nWorld\nAgain",
        );

    assert_eq!(
        table.line_count().unwrap(),
        3
    );
}


#[test]
fn line_starts_are_correct() {
    let mut table =
        test_table(
            "Hello\nWorld\nAgain",
        );

    assert_eq!(
        table.line_start(0).unwrap(),
        0
    );

    assert_eq!(
        table.line_start(1).unwrap(),
        6
    );

    assert_eq!(
        table.line_start(2).unwrap(),
        12
    );
}


#[test]
fn line_lengths_are_correct() {
    let mut table =
        test_table(
            "Hello\nWorld!\nAgain",
        );

    assert_eq!(
        table.line_length(0).unwrap(),
        5
    );

    assert_eq!(
        table.line_length(1).unwrap(),
        6
    );

    assert_eq!(
        table.line_length(2).unwrap(),
        5
    );
}


#[test]
fn cursor_line_and_column() {
    let mut table =
        test_table(
            "Hello\nWorld",
        );

    table
        .move_cursor(8)
        .unwrap();

    let (line, column) =
        table
            .cursor_line_column()
            .unwrap();

    assert_eq!(
        line,
        1
    );

    assert_eq!(
        column,
        2
    );
}


#[test]
fn cursor_move_to_clicked_column_is_utf8_safe_and_clears_selection() {
    let mut table =
        test_table("aé🙂b\nxy");

    table.move_cursor(0).unwrap();
    table.select_right().unwrap();
    table.select_right().unwrap();

    table.cursor.desired_column =
        Some(99);

    let original = table.text().unwrap();

    table
        .move_cursor_to_line_column(0, 3)
        .unwrap();

    assert_eq!(table.cursor.position, 7);
    assert_eq!(table.cursor.line, 0);
    assert_eq!(table.cursor.column, 3);
    assert_eq!(table.cursor.anchor, 7);
    assert_eq!(table.cursor.anchor_line, 0);
    assert_eq!(table.cursor.anchor_column, 3);
    assert_eq!(table.cursor.desired_column, None);
    assert_eq!(table.text().unwrap(), original);
}


#[test]
fn cursor_move_to_clicked_position_clamps_to_document_end() {
    let mut table =
        test_table("abc\ndef");

    table
        .move_cursor_to_line_column(99, 99)
        .unwrap();

    assert_eq!(table.cursor.position, table.len());
    assert_eq!(table.cursor.line, 1);
    assert_eq!(table.cursor.column, 3);
    assert!(!table.has_selection());
}


// ==========================================================================
// Editor tests
// ==========================================================================

#[test]
fn editor_insert_replaces_selection() {
    let mut editor =
        test_editor("Hello world");

    editor
        .document
        .move_cursor(6)
        .unwrap();

    editor
        .document
        .select_right()
        .unwrap();

    editor
        .document
        .select_right()
        .unwrap();

    editor
        .insert_text("Rust")
        .unwrap();

    assert_eq!(
        table_text(&editor.document),
        "Hello Rustrld"
    );
}


#[test]
fn editor_backspace_deletes_previous_character() {
    let mut editor =
        test_editor("Hello");

    editor
        .document
        .move_cursor(5)
        .unwrap();

    editor
        .backspace()
        .unwrap();

    assert_eq!(
        table_text(&editor.document),
        "Hell"
    );

    assert_eq!(
        editor.document.cursor.position,
        4
    );
}


#[test]
fn editor_delete_deletes_next_character() {
    let mut editor =
        test_editor("Hello");

    editor
        .document
        .move_cursor(0)
        .unwrap();

    editor
        .delete()
        .unwrap();

    assert_eq!(
        table_text(&editor.document),
        "ello"
    );

    assert_eq!(
        editor.document.cursor.position,
        0
    );
}


#[test]
fn editor_newline_inserts_newline() {
    let mut editor =
        test_editor("HelloWorld");

    editor
        .document
        .move_cursor(5)
        .unwrap();

    editor
        .newline()
        .unwrap();

    assert_eq!(
        table_text(&editor.document),
        "Hello\nWorld"
    );
}


// ==========================================================================
// UTF-8 editing tests
// ==========================================================================

#[test]
fn editor_backspace_removes_utf8_character() {
    let mut editor =
        test_editor("aé");

    editor
        .document
        .move_cursor(3)
        .unwrap();

    editor
        .backspace()
        .unwrap();

    assert_eq!(
        table_text(&editor.document),
        "a"
    );

    assert_eq!(
        editor.document.cursor.position,
        1
    );
}


#[test]
fn editor_delete_removes_utf8_character() {
    let mut editor =
        test_editor("aéb");

    editor
        .document
        .move_cursor(1)
        .unwrap();

    editor
        .delete()
        .unwrap();

    assert_eq!(
        table_text(&editor.document),
        "ab"
    );
}


// ==========================================================================
// Search key dispatch tests
// ==========================================================================

#[test]
fn search_delete_edits_the_query_not_the_document() {
    let mut editor =
        test_editor("document");

    let mut search = SearchUi::new();
    search.open(&mut editor.document).unwrap();
    search
        .set_query(&mut editor.document, "query")
        .unwrap();
    search.move_home();

    let result =
        handle_search_key(
            &mut search,
            &mut editor,
            Keycode::Delete,
            Mod::NOMOD,
            false,
        )
        .unwrap();

    assert!(matches!(
        result,
        SearchKeyResult::Consumed
    ));
    assert_eq!(search.query(), "uery");
    assert_eq!(table_text(&editor.document), "document");
}

#[test]
fn repeated_tab_does_not_cycle_search_mode_again() {
    let mut editor =
        test_editor("document");

    let mut search = SearchUi::new();
    search.open(&mut editor.document).unwrap();

    handle_search_key(
        &mut search,
        &mut editor,
        Keycode::Tab,
        Mod::NOMOD,
        false,
    )
    .unwrap();

    assert_eq!(
        search.mode(),
        crate::search::SearchMode::CaseInsensitive,
    );

    handle_search_key(
        &mut search,
        &mut editor,
        Keycode::Tab,
        Mod::NOMOD,
        true,
    )
    .unwrap();

    assert_eq!(
        search.mode(),
        crate::search::SearchMode::CaseInsensitive,
    );
}


// ==========================================================================
// Search and replace controller tests
// ==========================================================================

#[test]
fn undo_and_redo_reuse_file_backed_text() {
    let mut editor =
        test_editor("tail");

    let before =
        editor.document
            .edit_store_len();

    editor.insert("é🙂")
        .unwrap();

    let after_insert =
        editor.document
            .edit_store_len();

    assert_eq!(
        after_insert,
        before + "é🙂".len(),
    );

    for _ in 0..4 {
        editor.undo().unwrap();
        assert_eq!(
            editor.document
                .edit_store_len(),
            after_insert,
        );

        editor.redo().unwrap();
        assert_eq!(
            editor.document
                .edit_store_len(),
            after_insert,
        );
    }

    assert_eq!(
        table_text(&editor.document),
        "é🙂tail",
    );
}

#[test]
fn command_bar_drives_live_search_preview() {
    let mut editor =
        test_editor("Alpha beta alpha");

    let mut search_ui =
        SearchUi::new();

    let mut command_bar =
        CommandBar::new();

    command_bar.open(
        ":find alpha --ignore-case"
    );

    sync_command_search(
        &command_bar,
        &mut search_ui,
        &mut editor.document,
        None,
    )
    .unwrap();

    assert!(search_ui.is_active());
    assert_eq!(
        search_ui.mode(),
        SearchMode::CaseInsensitive,
    );
    assert_eq!(
        search_ui.current_match()
            .map(|found| {
                (found.start, found.end)
            }),
        Some((0, 5)),
    );
}

#[test]
fn non_search_command_closes_live_preview() {
    let mut editor =
        test_editor("alpha");

    let mut search_ui =
        SearchUi::new();

    let mut command_bar =
        CommandBar::new();

    command_bar.open(":find alpha");
    sync_command_search(
        &command_bar,
        &mut search_ui,
        &mut editor.document,
        None,
    )
    .unwrap();

    command_bar.open(":save");
    sync_command_search(
        &command_bar,
        &mut search_ui,
        &mut editor.document,
        None,
    )
    .unwrap();

    assert!(!search_ui.is_active());
    assert_eq!(
        search_ui.current_match(),
        None,
    );
}

#[test]
fn replace_current_shorter_unicode_undoes_and_redoes() {
    let mut editor =
        test_editor("é🙂 tail");
    let mut search = test_replace_ui(
        &mut editor,
        "é🙂",
        "λ",
        SearchMode::CaseSensitive,
    );

    assert_eq!(
        replace_current_match(&mut search, &mut editor)
            .unwrap(),
        1,
    );
    assert_eq!(table_text(&editor.document), "λ tail");
    assert_eq!(editor.document.cursor.position, 2);
    assert_eq!(editor.document.cursor.anchor, 2);
    assert_eq!(search.current_match(), None);
    assert_eq!(editor.undo_stack.len(), 1);

    editor.undo().unwrap();

    assert_eq!(table_text(&editor.document), "é🙂 tail");
    assert_eq!(editor.document.cursor.position, 0);
    assert_eq!(editor.document.cursor.anchor, 0);

    editor.redo().unwrap();

    assert_eq!(table_text(&editor.document), "λ tail");
    assert_eq!(editor.document.cursor.position, 2);
    assert_eq!(editor.document.cursor.anchor, 2);
}

#[test]
fn replace_current_longer_unicode_undoes_and_redoes() {
    let mut editor =
        test_editor("λ tail");
    let mut search = test_replace_ui(
        &mut editor,
        "λ",
        "é🙂",
        SearchMode::CaseSensitive,
    );

    assert_eq!(
        replace_current_match(&mut search, &mut editor)
            .unwrap(),
        1,
    );
    assert_eq!(table_text(&editor.document), "é🙂 tail");
    assert_eq!(editor.document.cursor.position, 6);
    assert_eq!(editor.document.cursor.anchor, 6);
    assert_eq!(search.current_match(), None);

    editor.undo().unwrap();

    assert_eq!(table_text(&editor.document), "λ tail");
    assert_eq!(editor.document.cursor.position, 0);
    assert_eq!(editor.document.cursor.anchor, 0);

    editor.redo().unwrap();

    assert_eq!(table_text(&editor.document), "é🙂 tail");
    assert_eq!(editor.document.cursor.position, 6);
    assert_eq!(editor.document.cursor.anchor, 6);
}

#[test]
fn replace_current_uses_case_insensitive_match_byte_range() {
    let mut editor =
        test_editor("K K");
    let mut search = test_replace_ui(
        &mut editor,
        "K",
        "x",
        SearchMode::CaseInsensitive,
    );

    let initial = search.current_match().unwrap();
    assert_eq!((initial.start, initial.end), (0, 1));

    assert_eq!(
        replace_current_match(&mut search, &mut editor)
            .unwrap(),
        1,
    );
    assert_eq!(table_text(&editor.document), "x K");

    let next = search.current_match().unwrap();
    assert_eq!((next.start, next.end), (2, 5));

    editor.undo().unwrap();
    assert_eq!(table_text(&editor.document), "K K");

    editor.redo().unwrap();
    assert_eq!(table_text(&editor.document), "x K");
}

#[test]
fn replace_all_uses_leftmost_non_overlapping_matches() {
    let mut editor =
        test_editor("aaaa");
    let mut search = test_replace_ui(
        &mut editor,
        "aa",
        "b",
        SearchMode::CaseSensitive,
    );

    assert_eq!(
        replace_all_matches(&mut search, &mut editor)
            .unwrap(),
        2,
    );
    assert_eq!(table_text(&editor.document), "bb");
    assert_eq!(editor.undo_stack.len(), 1);
    assert_eq!(search.last_replace_count(), Some(2));
}

#[test]
fn dense_replace_all_keeps_metadata_compact_and_maps_selection() {
    let mut editor =
        test_editor(
            &"a".repeat(100_000)
        );

    editor
        .set_cursor_and_anchor(
            50_000,
            25_000,
        )
        .unwrap();

    assert_eq!(
        editor
            .replace_all(
                &Searcher::new("a"),
                "zz",
            )
            .unwrap(),
        100_000,
    );

    assert_eq!(editor.document.len(), 200_000);
    assert!(
        editor.document.pieces.len() < 16,
        "dense replacement created {} pieces",
        editor.document.pieces.len(),
    );
    assert_eq!(editor.document.cursor.position, 100_000);
    assert_eq!(editor.document.cursor.anchor, 50_000);
    assert_eq!(editor.undo_stack.len(), 1);

    editor.undo().unwrap();

    assert_eq!(editor.document.len(), 100_000);
    assert_eq!(editor.document.cursor.position, 50_000);
    assert_eq!(editor.document.cursor.anchor, 25_000);

    editor.redo().unwrap();

    assert_eq!(editor.document.len(), 200_000);
    assert_eq!(editor.document.cursor.position, 100_000);
    assert_eq!(editor.document.cursor.anchor, 50_000);
}

#[test]
fn replace_all_does_not_reprocess_replacement_text() {
    let mut editor =
        test_editor("a a");
    let mut search = test_replace_ui(
        &mut editor,
        "a",
        "aa",
        SearchMode::CaseSensitive,
    );

    assert_eq!(
        replace_all_matches(&mut search, &mut editor)
            .unwrap(),
        2,
    );
    assert_eq!(table_text(&editor.document), "aa aa");
}

#[test]
fn replace_all_can_delete_every_match() {
    let mut editor =
        test_editor("x-x-x");
    let mut search = test_replace_ui(
        &mut editor,
        "x",
        "",
        SearchMode::CaseSensitive,
    );

    assert_eq!(
        replace_all_matches(&mut search, &mut editor)
            .unwrap(),
        3,
    );
    assert_eq!(table_text(&editor.document), "--");
    assert_eq!(editor.undo_stack.len(), 1);
}

#[test]
fn replace_all_is_one_history_step_and_restores_selection() {
    let mut editor =
        test_editor("cat dog cat");

    editor.document.move_cursor(4).unwrap();
    editor.document.select_right().unwrap();
    editor.document.select_right().unwrap();
    editor.document.select_right().unwrap();

    let searcher = Searcher::new("cat");

    assert_eq!(
        editor.replace_all(&searcher, "lion").unwrap(),
        2,
    );
    assert_eq!(table_text(&editor.document), "lion dog lion");
    assert_eq!(editor.undo_stack.len(), 1);
    assert_eq!(editor.redo_stack.len(), 0);
    assert_eq!(editor.document.cursor.position, 8);
    assert_eq!(editor.document.cursor.anchor, 5);
    assert_eq!(editor.document.selection_start(), 5);
    assert_eq!(editor.document.selection_end(), 8);

    editor.undo().unwrap();

    assert_eq!(table_text(&editor.document), "cat dog cat");
    assert_eq!(editor.undo_stack.len(), 0);
    assert_eq!(editor.redo_stack.len(), 1);
    assert_eq!(editor.document.cursor.position, 7);
    assert_eq!(editor.document.cursor.anchor, 4);
    assert_eq!(editor.document.selection_start(), 4);
    assert_eq!(editor.document.selection_end(), 7);

    editor.redo().unwrap();

    assert_eq!(table_text(&editor.document), "lion dog lion");
    assert_eq!(editor.undo_stack.len(), 1);
    assert_eq!(editor.redo_stack.len(), 0);
    assert_eq!(editor.document.cursor.position, 8);
    assert_eq!(editor.document.cursor.anchor, 5);
}

#[test]
fn no_op_replace_all_preserves_history_and_redo() {
    let mut editor =
        test_editor("abc");

    editor
        .replace_range(
            SearchResult { start: 0, end: 1 },
            "A",
        )
        .unwrap();
    editor.undo().unwrap();

    assert_eq!(table_text(&editor.document), "abc");
    assert_eq!(editor.undo_stack.len(), 0);
    assert_eq!(editor.redo_stack.len(), 1);

    assert_eq!(
        editor
            .replace_all(&Searcher::new("missing"), "x")
            .unwrap(),
        0,
    );
    assert_eq!(editor.undo_stack.len(), 0);
    assert_eq!(editor.redo_stack.len(), 1);

    assert_eq!(
        editor
            .replace_all(&Searcher::new("a"), "a")
            .unwrap(),
        0,
    );
    assert_eq!(editor.undo_stack.len(), 0);
    assert_eq!(editor.redo_stack.len(), 1);

    let zero_width =
        Searcher::with_mode("^", SearchMode::Regex)
            .unwrap();

    assert_eq!(
        editor.replace_all(&zero_width, "").unwrap(),
        0,
    );
    assert_eq!(table_text(&editor.document), "abc");
    assert_eq!(editor.undo_stack.len(), 0);
    assert_eq!(editor.redo_stack.len(), 1);

    editor.redo().unwrap();
    assert_eq!(table_text(&editor.document), "Abc");
}

#[test]
fn multiline_replacement_updates_line_and_cursor_state() {
    let mut editor =
        test_editor("head TOKEN tail");

    editor
        .replace_range(
            SearchResult { start: 5, end: 10 },
            "one\ntwo",
        )
        .unwrap();

    assert_eq!(
        table_text(&editor.document),
        "head one\ntwo tail",
    );
    assert_eq!(editor.document.line_count().unwrap(), 2);
    assert_eq!(editor.document.cursor.position, 12);
    assert_eq!(editor.document.cursor.anchor, 12);
    assert_eq!(editor.document.cursor.line, 1);
    assert_eq!(editor.document.cursor.column, 3);
    assert_eq!(
        editor.document.line_column_at(12).unwrap(),
        (1, 3),
    );
}

#[test]
fn replace_current_advances_without_wrapping_or_repeating() {
    let mut editor =
        test_editor("x x");
    let mut search = test_replace_ui(
        &mut editor,
        "x",
        "xx",
        SearchMode::CaseSensitive,
    );

    assert_eq!(
        replace_current_match(&mut search, &mut editor)
            .unwrap(),
        1,
    );
    assert_eq!(table_text(&editor.document), "xx x");

    let second = search.current_match().unwrap();
    assert_eq!((second.start, second.end), (3, 4));

    assert_eq!(
        replace_current_match(&mut search, &mut editor)
            .unwrap(),
        1,
    );
    assert_eq!(table_text(&editor.document), "xx xx");
    assert_eq!(search.current_match(), None);

    assert_eq!(
        replace_current_match(&mut search, &mut editor)
            .unwrap(),
        0,
    );
    assert_eq!(table_text(&editor.document), "xx xx");
    assert_eq!(editor.undo_stack.len(), 2);
}

#[test]
fn replacement_without_a_following_match_refreshes_the_editor_cursor() {
    assert!(replacement_requires_cursor_refresh(
        SearchKeyResult::ReplaceCurrent,
        false,
    ));
    assert!(replacement_requires_cursor_refresh(
        SearchKeyResult::ReplaceAll,
        false,
    ));

    assert!(!replacement_requires_cursor_refresh(
        SearchKeyResult::ReplaceCurrent,
        true,
    ));
    assert!(!replacement_requires_cursor_refresh(
        SearchKeyResult::Consumed,
        false,
    ));
}

#[test]
fn regex_zero_width_replace_all_terminates_with_exact_output() {
    let mut editor =
        test_editor("a\nb");
    let mut search = test_replace_ui(
        &mut editor,
        "^",
        ">",
        SearchMode::Regex,
    );

    assert_eq!(
        replace_all_matches(&mut search, &mut editor)
            .unwrap(),
        2,
    );
    assert_eq!(table_text(&editor.document), ">a\n>b");
    assert_eq!(editor.undo_stack.len(), 1);
    assert_eq!(search.current_match(), None);
    assert_eq!(search.last_replace_count(), Some(2));

    editor.undo().unwrap();
    assert_eq!(table_text(&editor.document), "a\nb");

    editor.redo().unwrap();
    assert_eq!(table_text(&editor.document), ">a\n>b");
}

#[test]
fn terminal_locations_support_unix_and_windows_paths() {
    assert_eq!(
        parse_location("src/main.rs:42"),
        Some(("src/main.rs", 42, None)),
    );

    assert_eq!(
        parse_location("src/main.rs:42:7"),
        Some(("src/main.rs", 42, Some(7))),
    );

    assert_eq!(
        parse_location(r"C:\work\src\main.rs:42:7"),
        Some((r"C:\work\src\main.rs", 42, Some(7))),
    );
}

#[test]
fn view_mode_keeps_document_unchanged() {
    let mut editor = test_editor("unchanged");
    editor.read_only = true;

    editor.insert("edited").unwrap();
    editor.delete().unwrap();
    editor.undo().unwrap();

    assert_eq!(
        table_text(&editor.document),
        "unchanged",
    );
    assert!(!editor.dirty);
}

#[test]
fn open_file_detection_prevents_duplicate_pane_documents() {
    let path = temporary_path();
    fs::write(&path, b"pane").unwrap();

    let mut editor =
        Editor::new(EditorConfig::default())
            .unwrap();
    editor.open(
        path.to_str().unwrap()
    ).unwrap();

    assert!(file_is_open_in(
        path.to_str().unwrap(),
        &editor,
    ));

    let different = temporary_path();
    assert!(!file_is_open_in(
        different.to_str().unwrap(),
        &editor,
    ));

    fs::remove_file(path).unwrap();
}

#[test]
fn navigation_select_all_replaces_and_undo_restores_utf8_selection() {
    let mut editor = test_editor("árvíz\n東京🙂");
    editor.execute(Command::SelectAll, 5).unwrap();
    assert_eq!(editor.document.selection_start(), 0);
    assert_eq!(editor.document.selection_end(), "árvíz\n東京🙂".len());
    assert_eq!(editor.document.cursor_line_column().unwrap(), (1, 3));
    assert_eq!((editor.document.cursor.anchor_line, editor.document.cursor.anchor_column), (0, 0));
    assert!(!editor.dirty);
    assert!(editor.undo_stack.is_empty());
    editor.insert("replacement").unwrap();
    editor.undo().unwrap();
    assert_eq!(editor.document.text().unwrap(), "árvíz\n東京🙂");
    assert_eq!(editor.document.selection_end(), "árvíz\n東京🙂".len());
}

#[test]
fn navigation_words_cross_unicode_punctuation_whitespace_and_piece_boundaries() {
    let mut editor = test_editor("héllo_");
    editor.document.insert(editor.document.len(), "世界,\t\r\n next🙂").unwrap();
    editor.document.move_cursor(0).unwrap();
    for prefix in ["héllo_世界", "héllo_世界,\t\r\n ", "héllo_世界,\t\r\n next", "héllo_世界,\t\r\n next🙂"] {
        editor.execute(Command::MoveWordRight, 5).unwrap();
        assert_eq!(editor.document.cursor.position, prefix.len());
        assert!(!editor.document.has_selection());
    }
    for prefix in ["héllo_世界,\t\r\n next", "héllo_世界,\t\r\n ", "héllo_世界", ""] {
        editor.execute(Command::SelectWordLeft, 5).unwrap();
        assert_eq!(editor.document.cursor.position, prefix.len());
        assert_eq!(editor.document.cursor.anchor, editor.document.len());
    }
    editor.execute(Command::MoveWordLeft, 5).unwrap();
    assert!(!editor.document.has_selection());
    editor.execute(Command::SelectWordRight, 5).unwrap();
    assert_eq!(editor.document.selection_end(), "héllo_世界".len());
    assert_eq!(editor.document.cursor.anchor, 0);
}

#[test]
fn navigation_pages_preserve_column_anchor_and_clamp_to_document() {
    let mut editor = test_editor("abcdef\nx\n東京🙂\nx\nabcdef\nx\nabcdef");
    editor.document.move_cursor(5).unwrap();
    editor.execute(Command::SelectPageDown, 2).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (2, 3));
    assert_eq!(editor.document.cursor.anchor, 5);
    editor.execute(Command::SelectPageDown, 2).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (4, 5));
    editor.execute(Command::SelectPageUp, 2).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (2, 3));
    assert_eq!((editor.document.cursor.anchor_line, editor.document.cursor.anchor_column), (0, 5));
    editor.execute(Command::PageDown, usize::MAX).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (6, 5));
    assert!(!editor.document.has_selection());
    editor.execute(Command::PageUp, usize::MAX).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (0, 5));
    editor.execute(Command::PageDown, 0).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (1, 1));
}

#[test]
fn navigation_home_end_keep_the_original_selection_anchor() {
    let mut editor = test_editor("abc\nárvíz");
    editor.document.move_cursor("abc\ná".len()).unwrap();
    editor.execute(Command::SelectHome, 5).unwrap();
    assert_eq!(editor.document.cursor.position, 4);
    editor.execute(Command::SelectEnd, 5).unwrap();
    assert_eq!(editor.document.cursor.position, editor.document.len());
    assert_eq!(editor.document.cursor.anchor, "abc\ná".len());
    assert_eq!((editor.document.cursor.anchor_line, editor.document.cursor.anchor_column), (1, 1));
}

#[test]
fn navigation_is_safe_for_empty_and_single_line_documents() {
    for text in ["", "🙂", "abc\n"] {
        let mut editor = test_editor(text);
        for command in [Command::SelectAll, Command::SelectWordLeft, Command::SelectWordRight,
            Command::SelectPageUp, Command::SelectPageDown, Command::SelectHome, Command::SelectEnd,
            Command::MoveWordLeft, Command::MoveWordRight, Command::PageUp, Command::PageDown] {
            editor.execute(command, 20).unwrap();
            assert!(editor.document.cursor.position <= text.len());
            assert!(text.is_char_boundary(editor.document.cursor.position));
        }
    }
}

#[test]
fn terminal_file_click_preserves_unsaved_document_in_other_pane() {
    let path = temporary_path();
    fs::write(&path, "clicked file").unwrap();
    let mut editor = test_editor("draft");
    editor.insert("unsaved ").unwrap();
    let original = editor.document.text().unwrap();
    let history_len = editor.undo_stack.len();
    let mut other = test_editor("");
    assert!(open_terminal_document(path.to_str().unwrap(), false, &mut editor, &mut other).unwrap());
    assert_eq!(editor.document.text().unwrap(), original);
    assert!(editor.dirty);
    assert_eq!(editor.undo_stack.len(), history_len);
    assert_eq!(other.document.text().unwrap(), "clicked file");
    assert_eq!(other.path.as_deref(), Some(path.as_path()));
    drop(other);
    fs::remove_file(path).unwrap();
}

#[test]
fn terminal_file_click_reuses_open_files_even_when_both_are_dirty() {
    let path = temporary_path();
    fs::write(&path, "disk version").unwrap();
    let mut editor = test_editor("");
    editor.open(path.to_str().unwrap()).unwrap();
    editor.insert("unsaved ").unwrap();
    let changed = editor.document.text().unwrap();
    let mut other = test_editor("");
    other.insert("another unsaved document").unwrap();
    assert!(!open_terminal_document(path.to_str().unwrap(), false, &mut editor, &mut other).unwrap());
    assert_eq!(editor.document.text().unwrap(), changed);
    assert!(editor.dirty);
    assert!(open_terminal_document(path.to_str().unwrap(), false, &mut other, &mut editor).unwrap());
    assert_eq!(editor.document.text().unwrap(), changed);
    drop(editor);
    fs::remove_file(path).unwrap();
}

#[test]
fn terminal_file_click_cannot_discard_two_unsaved_documents() {
    let mut editor = test_editor("");
    editor.insert("first draft").unwrap();
    let mut other = test_editor("");
    other.insert("second draft").unwrap();
    let error = open_terminal_document("new-file.txt", false, &mut editor, &mut other).unwrap_err();
    assert!(error.to_string().contains("Both panes have unsaved changes"));
    assert_eq!(editor.document.text().unwrap(), "first draft");
    assert_eq!(other.document.text().unwrap(), "second draft");
}

#[test]
fn terminal_failed_open_preserves_both_documents() {
    let mut editor = test_editor("");
    editor.insert("unsaved draft").unwrap();
    let mut other = test_editor("existing clean file");
    let missing = temporary_path();
    assert!(open_terminal_document(missing.to_str().unwrap(), false, &mut editor, &mut other).is_err());
    assert_eq!(editor.document.text().unwrap(), "unsaved draft");
    assert_eq!(other.document.text().unwrap(), "existing clean file");
}

#[test]
fn terminal_locations_jump_to_reported_lines_and_unicode_byte_columns() {
    let mut editor = test_editor("first\né🙂 fn main\nlast\n");
    move_to_terminal_location(&mut editor.document, 2, None, false).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (1, 0));
    move_to_terminal_location(&mut editor.document, 2, Some(8), true).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (1, 3));
    move_to_terminal_location(&mut editor.document, 2, Some(4), false).unwrap();
    assert_eq!(editor.document.cursor_line_column().unwrap(), (1, 3));
}

#[test]
fn recovery_opens_separate_work_and_save_as_acknowledges_without_clobbering() {
    let root = temporary_path().with_extension("recovery");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("original.cs"); std::fs::write(&source,"original").unwrap();
    let mut crashed = Editor::new(EditorConfig::default()).unwrap(); crashed.open(source.to_str().unwrap()).unwrap();
    crashed.document.enable_recovery_at(root.join("sessions"),Some(&source));
    crashed.insert_text("unsaved ").unwrap(); crashed.undo().unwrap(); crashed.redo().unwrap();
    assert!(crashed.document.take_recovery_warning().is_none()); drop(crashed);
    std::fs::write(&source,"changed outside").unwrap();
    let mut editor = Editor::new(EditorConfig::default()).unwrap();
    editor.insert_text("keep current text").unwrap();
    assert!(editor.recover_from(&root.join("sessions"),1).is_err());
    assert_eq!(editor.document.text().unwrap(),"keep current text");
    drop(editor);
    let mut editor=Editor::new(EditorConfig::default()).unwrap();
    assert!(!editor.recover_from(&root.join("sessions"),1).unwrap());
    assert!(editor.dirty); assert_eq!(editor.document.text().unwrap(),"unsaved original");
    assert_ne!(editor.path.as_deref(),Some(source.as_path()));
    let destination=root.join("chosen.cs"); editor.save_as(destination.to_str().unwrap(),false).unwrap();
    assert_eq!(std::fs::read_to_string(&destination).unwrap(),"unsaved original");
    assert_eq!(std::fs::read_to_string(&source).unwrap(),"changed outside");
    assert!(piece_table::recovery::list(&root.join("sessions")).unwrap().is_empty());
    drop(editor); std::fs::remove_dir_all(root).unwrap();
}
