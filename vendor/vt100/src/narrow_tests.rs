use crate::{Parser, ScrollbackSink};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct Rows(Vec<(u16, bool, Vec<u8>)>);
impl ScrollbackSink for Rows {
    fn push(&mut self, columns: u16, wrapped: bool, formatted: &[u8]) {
        self.0.push((columns, wrapped, formatted.to_vec()));
    }
}

#[test]
fn one_row_wrap_marks_outgoing_history_and_preserves_exact_output() {
    let rows = Arc::new(Mutex::new(Rows::default()));
    let mut parser = Parser::new(1, 3, 0);
    parser.screen_mut().set_scrollback_sink(rows.clone());
    parser.process(b"abcdefg\r\nh");
    let rows = rows.lock().unwrap();
    let decoded = rows
        .0
        .iter()
        .map(|(cols, wrapped, formatted)| {
            let mut row = Parser::new(1, *cols, 0);
            row.process(formatted);
            (row.screen().contents(), *wrapped)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        decoded,
        [
            ("abc".into(), true),
            ("def".into(), true),
            ("g".into(), false)
        ]
    );
    assert_eq!(parser.screen().contents(), "h");
    assert_eq!(parser.screen().cursor_position(), (0, 1));
    assert!(!parser.screen().row_wrapped(0));
}

#[test]
fn one_column_wide_glyphs_do_not_create_half_cells_and_resize_recovers() {
    let mut parser = Parser::new(1, 1, 0);
    parser.process("東京🙂".as_bytes());
    assert_eq!(parser.screen().contents(), "");
    assert_eq!(parser.screen().cursor_position(), (0, 0));
    parser.process("a\u{301}東b🙂".as_bytes());
    assert_eq!(parser.screen().contents(), "b");
    assert_eq!(parser.screen().cursor_position(), (0, 1));
    assert!(!parser.screen().cell(0, 0).unwrap().is_wide());
    assert!(!parser.screen().cell(0, 0).unwrap().is_wide_continuation());
    parser.screen_mut().set_size(1, 2);
    parser.process("\r東".as_bytes());
    assert_eq!(parser.screen().contents(), "東");
    assert!(parser.screen().cell(0, 0).unwrap().is_wide());
    assert!(parser.screen().cell(0, 1).unwrap().is_wide_continuation());
}
