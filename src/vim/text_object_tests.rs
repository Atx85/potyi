// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::{Clipboard, editor};
use super::*;

fn press(vim: &mut VimController, editor: &mut Editor, clipboard: &Clipboard, keys: &str) {
    for character in keys.chars() {
        let (key, shifted) = match character {
            '{' => (Keycode::LeftBracket, true),
            '}' => (Keycode::RightBracket, true),
            '[' => (Keycode::LeftBracket, false),
            ']' => (Keycode::RightBracket, false),
            '(' => (Keycode::_9, true),
            ')' => (Keycode::_0, true),
            '<' => (Keycode::Comma, true),
            '>' => (Keycode::Period, true),
            '"' => (Keycode::Apostrophe, true),
            '\'' => (Keycode::Apostrophe, false),
            '`' => (Keycode::Grave, false),
            '.' => (Keycode::Period, false),
            _ => (
                Keycode::from_name(&character.to_ascii_uppercase().to_string()).unwrap(),
                character.is_ascii_uppercase(),
            ),
        };
        let modifiers = if shifted { Mod::LSHIFTMOD } else { Mod::NOMOD };
        if shifted {
            vim.handle_key(editor, clipboard, Keycode::LShift, modifiers, false, 10)
                .unwrap();
        }
        let outcome = vim
            .handle_key(editor, clipboard, key, modifiers, false, 10)
            .unwrap();
        assert!(outcome.consumed, "{character} in {keys}");
        // Each printable SDL keydown is followed by a text-input event.
        vim.consume_suppressed_text_input();
    }
}
fn escape(vim: &mut VimController, editor: &mut Editor, clipboard: &Clipboard) {
    vim.handle_key(editor, clipboard, Keycode::Escape, Mod::NOMOD, false, 10)
        .unwrap();
}
fn selected(editor: &Editor) -> String {
    let start = editor.document.selection_start();
    let end = editor.document.selection_end();
    String::from_utf8(editor.document.read_range(start, end - start).unwrap()).unwrap()
}

#[test]
fn text_object_delete_supports_every_pair_alias_and_quote() {
    for (text, target, keys, expected) in [
        ("left one right", "ne", "diw", "left  right"),
        ("left foo.bar right", "bar", "diW", "left  right"),
        ("left one right", "ne", "daw", "left right"),
        ("left foo.bar right", "bar", "daW", "left right"),
        ("a (two) z", "two", "di(", "a () z"),
        ("a (two) z", "two", "di)", "a () z"),
        ("a (two) z", "two", "dib", "a () z"),
        ("a (two) z", "two", "dab", "a  z"),
        ("a {two} z", "two", "di{", "a {} z"),
        ("a {two} z", "two", "di}", "a {} z"),
        ("a {two} z", "two", "diB", "a {} z"),
        ("a {two} z", "two", "daB", "a  z"),
        ("a [two] z", "two", "di[", "a [] z"),
        ("a [two] z", "two", "di]", "a [] z"),
        ("a [two] z", "two", "da]", "a  z"),
        ("a <two> z", "two", "di<", "a <> z"),
        ("a <two> z", "two", "di>", "a <> z"),
        ("a <two> z", "two", "da<", "a  z"),
        ("a 'two' z", "two", "di'", "a '' z"),
        ("a `two` z", "two", "di`", "a `` z"),
        ("a \"two\" z", "two", "di\"", "a \"\" z"),
        ("a \"two\" z", "two", "da\"", "a z"),
        ("a \"two\" z", "two", "d2i\"", "a  z"),
        (r#"let s = "a \"b\" c";"#, "b", "di\"", "let s = \"\";"),
        ("one café_two three", "fé", "diw", "one  three"),
    ] {
        let mut editor = editor(text);
        editor
            .document
            .move_cursor(text.find(target).unwrap())
            .unwrap();
        let mut vim = VimController::new();
        press(&mut vim, &mut editor, &Clipboard::default(), keys);
        assert_eq!(
            editor.document.text().unwrap(),
            expected,
            "{keys} in {text}"
        );
        assert_eq!(vim.mode(), VimMode::Normal);
        editor.undo().unwrap();
        assert_eq!(editor.document.text().unwrap(), text, "undo {keys}");
        editor.redo().unwrap();
        assert_eq!(editor.document.text().unwrap(), expected, "redo {keys}");
    }
}

#[test]
fn text_object_counts_nest_and_multiply() {
    for (text, target, keys, expected) in [
        ("a {one {two} three} z", "two", "d2i{", "a {} z"),
        ("a {one {two} three} z", "two", "2da{", "a  z"),
        ("one two three four five", "one", "2d2aw", "five"),
        ("one two three", "one", "d2iw", "two three"),
        ("one two three", "one", "d3iw", " three"),
    ] {
        let mut editor = editor(text);
        editor
            .document
            .move_cursor(text.find(target).unwrap())
            .unwrap();
        press(
            &mut VimController::new(),
            &mut editor,
            &Clipboard::default(),
            keys,
        );
        assert_eq!(editor.document.text().unwrap(), expected, "{keys}");
    }
}

#[test]
fn text_object_change_is_one_undo_step_including_empty_and_multiline_blocks() {
    for (text, target, keys, expected) in [
        ("one two", "ne", "ciw", "NEW two"),
        ("call(one)", "one", "ci(", "call(NEW)"),
        ("call()", ")", "ci(", "call(NEW)"),
        ("let s = \"\";", "\"", "ci\"", "let s = \"NEW\";"),
        (
            "fn() {\n  one\n  two\n}\nnext",
            "one",
            "ci{",
            "fn() {\nNEW\n}\nnext",
        ),
        ("fn() {\n}\nnext", "}", "ci{", "fn() {\nNEW\n}\nnext"),
    ] {
        let mut editor = editor(text);
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();
        editor
            .document
            .move_cursor(text.find(target).unwrap())
            .unwrap();
        press(&mut vim, &mut editor, &clipboard, keys);
        assert_eq!(vim.mode(), VimMode::Insert, "{keys} in {text}");
        editor.insert("NEW").unwrap();
        vim.record_text("NEW");
        escape(&mut vim, &mut editor, &clipboard);
        assert_eq!(
            editor.document.text().unwrap(),
            expected,
            "{keys} in {text}"
        );
        assert_eq!(editor.undo_stack.len(), 1);
        editor.undo().unwrap();
        assert_eq!(editor.document.text().unwrap(), text);
    }
}

#[test]
fn text_object_dot_recomputes_objects_at_the_new_cursor() {
    let mut editor = editor("one longer_word");
    let mut vim = VimController::new();
    let clipboard = Clipboard::default();
    press(&mut vim, &mut editor, &clipboard, "ciw");
    editor.insert("x").unwrap();
    vim.record_text("x");
    escape(&mut vim, &mut editor, &clipboard);
    press(&mut vim, &mut editor, &clipboard, "w.");
    assert_eq!(editor.document.text().unwrap(), "x x");
    editor.undo().unwrap();
    assert_eq!(editor.document.text().unwrap(), "x longer_word");

    let mut editor = super::tests::editor("(first) (a much longer second)");
    press(&mut vim, &mut editor, &clipboard, "di(");
    editor.document.move_cursor(5).unwrap();
    press(&mut vim, &mut editor, &clipboard, ".");
    assert_eq!(editor.document.text().unwrap(), "() ()");
}

#[test]
fn text_object_yank_and_visual_selection_preserve_whole_objects() {
    let mut editor = editor("before {one {two} three} after");
    let mut vim = VimController::new();
    let clipboard = Clipboard::default();
    editor.document.move_cursor(14).unwrap();
    press(&mut vim, &mut editor, &clipboard, "yi{");
    assert_eq!(clipboard.text().unwrap(), "two");
    assert!(editor.undo_stack.is_empty());
    press(&mut vim, &mut editor, &clipboard, "vi{");
    assert_eq!(selected(&editor), "two");
    press(&mut vim, &mut editor, &clipboard, "i{");
    assert_eq!(selected(&editor), "one {two} three");
    press(&mut vim, &mut editor, &clipboard, "a{");
    assert_eq!(selected(&editor), "{one {two} three}");
    press(&mut vim, &mut editor, &clipboard, "d");
    assert_eq!(editor.document.text().unwrap(), "before  after");
    assert_eq!(vim.mode(), VimMode::Normal);

    let mut editor = super::tests::editor("one café three");
    editor.document.move_cursor(6).unwrap();
    press(&mut vim, &mut editor, &clipboard, "viwy");
    assert_eq!(clipboard.text().unwrap(), "café");
    assert_eq!(vim.mode(), VimMode::Normal);
}

#[test]
fn text_object_sentences_paragraphs_and_tags_edit_and_expand() {
    for (text, target, keys, expected) in [
        ("One thing.  Two things!", "thing", "dis", "  Two things!"),
        ("One thing.  Two things!", "thing", "das", "Two things!"),
        ("one\nline\n\nnext\n", "line", "dip", "\nnext\n"),
        ("one\nline\n\nnext\n", "line", "dap", "next\n"),
        ("<div><b>hi</b></div>", "hi", "dit", "<div><b></b></div>"),
        ("<div><b>hi</b></div>", "hi", "dat", "<div></div>"),
        ("<div><b>hi</b></div>", "hi", "d2it", "<div></div>"),
    ] {
        let mut editor = editor(text);
        editor
            .document
            .move_cursor(text.find(target).unwrap())
            .unwrap();
        press(
            &mut VimController::new(),
            &mut editor,
            &Clipboard::default(),
            keys,
        );
        assert_eq!(editor.document.text().unwrap(), expected, "{keys}");
    }
    let mut editor = editor("<div><b>hi</b></div>");
    let mut vim = VimController::new();
    let clipboard = Clipboard::default();
    editor.document.move_cursor(9).unwrap();
    press(&mut vim, &mut editor, &clipboard, "vit");
    assert_eq!(selected(&editor), "hi");
    press(&mut vim, &mut editor, &clipboard, "it");
    assert_eq!(selected(&editor), "<b>hi</b>");
    press(&mut vim, &mut editor, &clipboard, "it");
    assert_eq!(selected(&editor), "<b>hi</b>");
    press(&mut vim, &mut editor, &clipboard, "it");
    assert_eq!(selected(&editor), "<div><b>hi</b></div>");
}

#[test]
fn text_object_missing_matches_escape_and_read_only_are_safe() {
    for (text, keys) in [
        ("plain text", "ci{"),
        ("unclosed (text", "di("),
        ("\"unclosed", "ci\""),
    ] {
        let mut editor = editor(text);
        let mut vim = VimController::new();
        press(&mut vim, &mut editor, &Clipboard::default(), keys);
        assert_eq!(vim.mode(), VimMode::Normal);
        assert_eq!(editor.document.text().unwrap(), text);
        assert!(editor.undo_stack.is_empty());
    }
    let mut editor = editor("word (keep)");
    let mut vim = VimController::new();
    let clipboard = Clipboard::default();
    press(&mut vim, &mut editor, &clipboard, "ci");
    escape(&mut vim, &mut editor, &clipboard);
    press(&mut vim, &mut editor, &clipboard, "w");
    assert_eq!(editor.document.text().unwrap(), "word (keep)");
    editor.read_only = true;
    press(&mut vim, &mut editor, &clipboard, "ci(");
    assert_eq!(vim.mode(), VimMode::Normal);
    assert_eq!(editor.document.text().unwrap(), "word (keep)");
    assert!(editor.undo_stack.is_empty());
}

#[test]
fn text_object_empty_changes_repeat_and_do_not_add_blank_lines_to_cc() {
    let mut editor = editor("() ()");
    let mut vim = VimController::new();
    let clipboard = Clipboard::default();
    press(&mut vim, &mut editor, &clipboard, "ci(");
    editor.insert("x").unwrap();
    vim.record_text("x");
    escape(&mut vim, &mut editor, &clipboard);
    editor.document.move_cursor(5).unwrap();
    press(&mut vim, &mut editor, &clipboard, ".");
    assert_eq!(editor.document.text().unwrap(), "(x) (x)");
    editor.undo().unwrap();
    assert_eq!(editor.document.text().unwrap(), "(x) ()");

    let mut editor = super::tests::editor("\nnext");
    press(&mut vim, &mut editor, &clipboard, "cc");
    editor.insert("x").unwrap();
    vim.record_text("x");
    escape(&mut vim, &mut editor, &clipboard);
    assert_eq!(editor.document.text().unwrap(), "x\nnext");
}

#[test]
fn text_object_visual_quotes_extend_and_paragraphs_are_linewise() {
    let mut editor = editor("\"one\" \"two\"");
    let mut vim = VimController::new();
    let clipboard = Clipboard::default();
    editor.document.move_cursor(2).unwrap();
    press(&mut vim, &mut editor, &clipboard, "va\"");
    assert_eq!(selected(&editor), "\"one\" ");
    press(&mut vim, &mut editor, &clipboard, "a\"");
    assert_eq!(selected(&editor), "\"one\" \"two\"");
    escape(&mut vim, &mut editor, &clipboard);
    let mut editor = super::tests::editor("one\ntwo\n\nnext\n");
    press(&mut vim, &mut editor, &clipboard, "vipy");
    assert_eq!(clipboard.text().unwrap(), "one\ntwo\n");
    assert!(vim.register_linewise);
}

#[test]
fn text_object_multiline_change_preserves_crlf_and_repeats_into_empty_blocks() {
    let mut editor = editor("{\r\n  old\r\n}\r\n{\r\n}\r\n");
    let mut vim = VimController::new();
    let clipboard = Clipboard::default();
    editor.document.move_cursor(6).unwrap();
    press(&mut vim, &mut editor, &clipboard, "ci{");
    editor.insert("new").unwrap();
    vim.record_text("new");
    escape(&mut vim, &mut editor, &clipboard);
    assert_eq!(
        editor.document.text().unwrap(),
        "{\r\nnew\r\n}\r\n{\r\n}\r\n"
    );
    let second = editor.document.text().unwrap().rfind('{').unwrap();
    editor.document.move_cursor(second).unwrap();
    press(&mut vim, &mut editor, &clipboard, ".");
    assert_eq!(
        editor.document.text().unwrap(),
        "{\r\nnew\r\n}\r\n{\r\nnew\r\n}\r\n"
    );
    editor.undo().unwrap();
    assert_eq!(
        editor.document.text().unwrap(),
        "{\r\nnew\r\n}\r\n{\r\n}\r\n"
    );
}
