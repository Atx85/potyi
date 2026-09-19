use super::*;

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
