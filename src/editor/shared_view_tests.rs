use super::*;

#[test]
fn shared_views_have_one_layout_index_history_and_file_state() {
    let mut a = Editor::new(EditorConfig::default()).unwrap();
    a.insert_text(&"line\n".repeat(4096)).unwrap();
    a.document.line_count().unwrap();
    for _ in 0..1000 {
        a.insert_text("x").unwrap();
    }
    let (mut b, copies) = crate::benchmarks::scaling::with_counters(|| {
        let mut b = a.duplicate_view();
        a.insert_text("tail").unwrap();
        Editor::synchronize_views(&mut a, &mut b).unwrap();
        b.undo().unwrap();
        Editor::synchronize_views(&mut b, &mut a).unwrap();
        a.redo().unwrap();
        Editor::synchronize_views(&mut a, &mut b).unwrap();
        b
    });
    assert!(Rc::ptr_eq(&a.state, &b.state));
    assert_eq!(a.document.pieces().as_ptr(), b.document.pieces().as_ptr());
    assert_eq!(
        a.undo_stack.entries().as_ptr(),
        b.undo_stack.entries().as_ptr()
    );
    assert_eq!(a.undo_stack.len(), 1002);
    assert_eq!(copies.copied_piece_records, 0);
    assert_eq!(copies.copied_line_records, 0);
    assert_eq!(copies.copied_history_entries, 0);
    a.set_path(Some(PathBuf::from("shared.txt")));
    a.set_dirty(false);
    b.set_read_only(true);
    assert_eq!(b.path(), a.path());
    assert!(!b.is_dirty());
    assert!(a.is_read_only());
    let text = a.document.text().unwrap();
    a.insert_text("forbidden").unwrap();
    assert_eq!(a.document.text().unwrap(), text);
    b.set_read_only(false);
    drop(a);
    b.document.move_cursor(b.document.len()).unwrap();
    b.insert_text("survives").unwrap();
    assert!(b.is_dirty());
    assert!(b.document.text().unwrap().ends_with("tailsurvives"));
}

#[test]
fn stale_view_edits_refresh_the_cursor_before_recording_shared_history() {
    let mut a = Editor::new(EditorConfig::default()).unwrap();
    a.insert_text("abc").unwrap();
    let mut b = a.duplicate_view();
    b.document.move_cursor(1).unwrap();
    a.document.move_cursor(0).unwrap();
    a.insert_text("X").unwrap();
    b.insert_text("Y").unwrap();
    assert_eq!(a.document.text().unwrap(), "XaYbc");
    a.undo().unwrap();
    assert_eq!(b.document.text().unwrap(), "Xabc");
    b.redo().unwrap();
    assert_eq!(b.document.cursor.position, 3);
    assert_eq!(a.document.text().unwrap(), "XaYbc");
}

fn pair() -> (Editor, Editor) {
    let mut a = Editor::new(EditorConfig::default()).unwrap();
    a.insert_text("first\né🦀 last\n").unwrap();
    let b = a.duplicate_view();
    (a, b)
}

#[test]
fn shared_views_keep_cursors_and_selections_but_share_edits_and_history() {
    let (mut a, mut b) = pair();
    b.document.move_cursor(6).unwrap();
    b.document.cursor.anchor = 12;
    b.document.cursor.anchor_line = 1;
    b.document.cursor.anchor_column = 2;
    a.document.move_cursor(0).unwrap();
    a.insert_text("header\n").unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert_eq!(a.document.text().unwrap(), b.document.text().unwrap());
    assert_eq!(b.document.cursor.position, 13);
    assert_eq!(b.document.cursor.line, 2);
    assert_eq!(b.document.cursor.anchor, 19);
    assert_eq!(b.document.cursor.anchor_column, 2);
    assert_eq!(a.document.cursor.position, 7);
    b.document.move_cursor(b.document.len()).unwrap();
    b.insert_text("tail").unwrap();
    Editor::synchronize_views(&mut b, &mut a).unwrap();
    a.undo().unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert!(!b.document.text().unwrap().ends_with("tail"));
    b.undo().unwrap();
    Editor::synchronize_views(&mut b, &mut a).unwrap();
    assert_eq!(a.document.text().unwrap(), "first\né🦀 last\n");
    a.redo().unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    b.redo().unwrap();
    Editor::synchronize_views(&mut b, &mut a).unwrap();
    assert_eq!(a.document.text().unwrap(), "header\nfirst\né🦀 last\ntail");
    assert_eq!(a.document.text().unwrap(), b.document.text().unwrap());
}

#[test]
fn shared_views_support_snapshot_undo_and_outlive_the_original_view() {
    let (mut a, mut b) = pair();
    let original = a.document.text().unwrap();
    a.apply_formatted(&mut io::Cursor::new("formatted 🦀\n"))
        .unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    b.undo().unwrap();
    Editor::synchronize_views(&mut b, &mut a).unwrap();
    assert_eq!(a.document.text().unwrap(), original);
    a.redo().unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    drop(a);
    b.document.move_cursor(b.document.len()).unwrap();
    b.insert_text("still editable").unwrap();
    assert_eq!(b.document.text().unwrap(), "formatted 🦀\nstill editable");
    b.undo().unwrap();
    b.undo().unwrap();
    assert_eq!(b.document.text().unwrap(), original);
}

#[test]
fn shared_views_preserve_middle_selection_across_compound_edits_and_grouped_undo() {
    let mut a = Editor::new(EditorConfig::default()).unwrap();
    a.insert_text("aaa é🦀 ccc").unwrap();
    let mut b = a.duplicate_view();
    let position = "aaa é".len();
    let anchor = "aaa é🦀".len();
    b.set_cursor_and_anchor(position, anchor).unwrap();
    let group = a.begin_history_group();
    a.document.move_cursor(0).unwrap();
    a.insert_text("X").unwrap();
    a.document.move_cursor(a.document.len()).unwrap();
    a.insert_text("Y").unwrap();
    a.end_history_group(group);
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert_eq!(b.document.text().unwrap(), "Xaaa é🦀 cccY");
    assert_eq!(b.document.cursor.position, position + 1);
    assert_eq!(b.document.cursor.anchor, anchor + 1);
    assert_eq!(b.document.cursor.column, 6);
    assert_eq!(b.document.cursor.anchor_column, 7);
    a.undo().unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert_eq!(b.document.cursor.position, position);
    assert_eq!(b.document.cursor.anchor, anchor);
    a.redo().unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert_eq!(b.document.cursor.position, position + 1);
    assert_eq!(b.document.cursor.anchor, anchor + 1);
}

#[test]
fn shared_views_preserve_middle_selection_across_replace_all_and_snapshot_undo() {
    let original = "old é🦀 middle old";
    let mut a = Editor::new(EditorConfig::default()).unwrap();
    a.insert_text(original).unwrap();
    let mut b = a.duplicate_view();
    let position = original.find("middle").unwrap();
    b.set_cursor_and_anchor(position, position + "middle".len())
        .unwrap();
    assert_eq!(
        a.replace_all(&Searcher::new("old"), "replacement").unwrap(),
        2
    );
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    let shift = "replacement".len() - "old".len();
    assert_eq!(b.document.cursor.position, position + shift);
    assert_eq!(b.document.cursor.anchor, position + shift + "middle".len());
    a.undo().unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert_eq!(b.document.cursor.position, position);
    assert_eq!(b.document.cursor.anchor, position + "middle".len());
    a.redo().unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert_eq!(b.document.cursor.position, position + shift);
    assert_eq!(b.document.cursor.anchor, position + shift + "middle".len());
}
