// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::super::{
    browser::{Browser, decode_native, native_name_range},
    layout::{Fragment, Layout, Metrics},
    output_selection::record_text,
    transcript::{RecordData, SharedTranscript, Transcript},
};
use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
fn cwd() -> PathBuf {
    std::env::temp_dir().canonicalize().unwrap()
}
fn tone(store: &SharedTranscript, id: u64) -> Option<Tone> {
    let record = store.lock().unwrap().read_id(id).unwrap();
    let RecordData::Terminal { formatted, .. } = record.data else {
        panic!("terminal required")
    };
    stored_style(&formatted).0
}
fn view(store: SharedTranscript, cols: u16) -> (Layout, Vec<Fragment>) {
    let mut layout = Layout::new(store, Arc::new(|| {})).unwrap();
    layout.configure(Metrics {
        columns: cols,
        tab_width: 4,
    });
    let rows = visible(&mut layout);
    (layout, rows)
}
fn visible(layout: &mut Layout) -> Vec<Fragment> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        layout.visible(0, 160).unwrap();
        layout.poll().unwrap();
        let rows = layout.visible(0, 160).unwrap();
        if layout.complete() && layout.cached_start() == Some(0) && !layout.pending() {
            return rows;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
}
fn style(rows: &[Fragment], needle: &str) -> super::super::layout::Style {
    for row in rows {
        if let Some(byte) = row.text.find(needle) {
            if let Some(segment) = row
                .segments
                .iter()
                .find(|segment| segment.range.contains(&byte))
            {
                return segment.style;
            }
        }
    }
    panic!(
        "missing {needle:?} in {:?}",
        rows.iter().map(|row| &row.text).collect::<Vec<_>>()
    )
}
#[test]
fn plain_diagnostics_keep_annotations_colored_and_source_excerpt_neutral() {
    let shared = Transcript::shared().unwrap();
    let mut ids = Vec::new();
    {
        let mut t = shared.lock().unwrap();
        let s = t.next_source();
        t.append_header(&cwd(), s, "cargo check").unwrap();
        for text in [
            "error[E0308]: mismatch",
            "  --> src/main.rs:3:7",
            "3 | let value = 1;",
            "  |       ^^^",
            "  = note: explain",
            "help: change the value",
            "3 - old",
            "3 + new",
            "",
            "warning: unused",
            "  | ^^^",
            "",
            "ordinary output",
        ] {
            ids.push(
                t.append_terminal(&cwd(), s, 80, false, text.as_bytes())
                    .unwrap()
                    .record_id,
            );
        }
    }
    let expected = [
        Some(Tone::Removed),
        Some(Tone::Hunk),
        None,
        Some(Tone::Removed),
        Some(Tone::Hunk),
        Some(Tone::Added),
        Some(Tone::Removed),
        Some(Tone::Added),
        None,
        Some(Tone::Warning),
        Some(Tone::Warning),
        None,
        None,
    ];
    for (id, expected) in ids.into_iter().zip(expected) {
        assert_eq!(tone(&shared, id), expected);
    }
}
#[test]
fn non_git_diff_colors_do_not_leak_into_the_next_command() {
    let shared = Transcript::shared().unwrap();
    let mut first = Vec::new();
    let last;
    {
        let mut t = shared.lock().unwrap();
        let source = t.next_source();
        t.append_header(&cwd(), source, "diff old new").unwrap();
        for text in [
            "1c1",
            "< old",
            "> new",
            "--- before",
            "+++ after",
            "@@ -1 +1 @@",
        ] {
            first.push(
                t.append_terminal(&cwd(), source, 80, false, text.as_bytes())
                    .unwrap()
                    .record_id,
            );
        }
        let next = t.next_source();
        t.append_header(&cwd(), next, "printf '+ordinary'").unwrap();
        last = t
            .append_terminal(&cwd(), next, 80, false, b"+ordinary")
            .unwrap()
            .record_id;
    }
    for (id, expected) in first.into_iter().zip([
        Tone::Hunk,
        Tone::Removed,
        Tone::Added,
        Tone::Header,
        Tone::Header,
        Tone::Hunk,
    ]) {
        assert_eq!(tone(&shared, id), Some(expected));
    }
    assert_eq!(tone(&shared, last), None);
}
#[test]
fn split_error_heading_reclassifies_all_retained_prefix_rows_without_changing_text() {
    let shared = Transcript::shared().unwrap();
    let mut ids = Vec::new();
    {
        let mut t = shared.lock().unwrap();
        let source = t.next_source();
        t.append_header(&cwd(), source, "compiler").unwrap();
        let text = "error: failed";
        for (index, ch) in text.chars().enumerate() {
            ids.push(
                t.append_terminal(
                    &cwd(),
                    source,
                    1,
                    index + 1 < text.len(),
                    ch.to_string().as_bytes(),
                )
                .unwrap()
                .record_id,
            );
        }
    }
    let mut text = String::new();
    for id in ids {
        assert_eq!(tone(&shared, id), Some(Tone::Removed));
        let record = shared.lock().unwrap().read_id(id).unwrap();
        text.push_str(&record_text(&record).unwrap().text);
    }
    assert_eq!(text, "error: failed");
}
#[test]
fn wrapped_rust_location_underlines_only_location_and_keeps_utf8_offsets() {
    let shared = Transcript::shared().unwrap();
    let ids;
    {
        let mut t = shared.lock().unwrap();
        let s = t.next_source();
        t.append_header(&cwd(), s, "cargo check").unwrap();
        let a = t
            .append_terminal(&cwd(), s, 80, true, "  --> src/東京 ".as_bytes())
            .unwrap();
        let b = t
            .append_terminal(&cwd(), s, 80, false, b"notes.rs:3:7")
            .unwrap();
        ids = [a.record_id, b.record_id];
    }
    for (id, expected) in ids
        .into_iter()
        .zip([6.."  --> src/東京 ".len(), 0.."notes.rs:3:7".len()])
    {
        let record = shared.lock().unwrap().read_id(id).unwrap();
        let RecordData::Terminal { formatted, .. } = record.data else {
            panic!()
        };
        assert_eq!(stored_style(&formatted).1, Some(expected));
    }
    let (_, rows) = view(shared, 80);
    assert!(!style(&rows, "-->").underline);
    assert!(style(&rows, "src/").underline);
    assert!(style(&rows, "notes.rs").underline);
}
#[test]
fn ansi_diagnostics_use_exact_legacy_colors_and_normal_text_styles() {
    let shared = Transcript::shared().unwrap();
    {
        let mut t = shared.lock().unwrap();
        let source = t.next_source();
        t.append_header(&cwd(), source, "compiler").unwrap();
        t.append_terminal(
            &cwd(),
            source,
            80,
            false,
            b"error: \x1b[1;3;4;33;44mexplicit\x1b[0m plain",
        )
        .unwrap();
    }
    let (_, rows) = view(shared, 80);
    let explicit = style(&rows, "explicit");
    assert_eq!(explicit.fg, vt100::Color::Rgb(245, 145, 145));
    assert_eq!(explicit.bg, vt100::Color::Default);
    assert!(
        !explicit.bold
            && !explicit.italic
            && !explicit.underline
            && !explicit.dim
            && !explicit.inverse
    );
    assert_eq!(style(&rows, "plain").fg, vt100::Color::Rgb(245, 145, 145));
}
#[test]
fn live_tail_reclassifies_saved_prefix_and_underlines_wrapped_location_before_completion() {
    let shared = Transcript::shared().unwrap();
    let source;
    {
        let mut t = shared.lock().unwrap();
        source = t.next_source();
        t.append_header(&cwd(), source, "compiler").unwrap();
        t.append_terminal(&cwd(), source, 80, true, b"er").unwrap();
    }
    let (mut layout, _) = view(shared.clone(), 80);
    let mut parser = vt100::Parser::new(2, 80, 0);
    parser.process(b"ror: failed");
    layout.set_tail(parser.screen(), 0, 1, source).unwrap();
    let rows = visible(&mut layout);
    assert_eq!(style(&rows, "error").fg, vt100::Color::Rgb(245, 145, 145));
    assert_eq!(
        tone(&shared, 1),
        None,
        "preview must not mutate disk metadata"
    );
}
#[test]
fn clear_forgets_annotation_context_but_keeps_active_diff_family() {
    let mut colors = Colors::default();
    colors.begin(1, "compiler");
    colors.row(1, "error: failed", false, None);
    colors.clear();
    assert_eq!(colors.row(1, " | ^^^", false, None).current.tone, None);
    colors.begin(2, "diff old new");
    colors.row(2, "< old", false, None);
    colors.clear();
    assert_eq!(
        colors.row(2, "> new", false, None).current.tone,
        Some(Tone::Added)
    );
}
#[test]
fn line_prefix_and_span_storage_remain_bounded_and_partial_styles_do_not_leak() {
    let mut colors = Colors::default();
    colors.begin(1, "compiler");
    for id in 0..400 {
        colors.row(1, &"x".repeat(1024), true, Some(id));
        let (p, l, r) = colors.storage();
        assert!(p <= PREFIX_BYTES && l <= LINE_BYTES && r <= LINE_ROWS);
    }
    assert_eq!(
        colors.row(1, "error: tail", false, None).current.underline,
        None
    );
    assert_eq!(colors.row(2, " | ^^^", false, None).current.tone, None);
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let mut token = [0; 8];
        getrandom::getrandom(&mut token).unwrap();
        let token = token.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let path = std::env::temp_dir().join(format!("potyi-style-{token}"));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn native_short_and_long_underlines_match_exact_names_and_exclude_binary_rows() {
    let directory = Directory::new();
    fs::write(directory.0.join(" leading é.rs"), "text").unwrap();
    fs::write(directory.0.join("d 42 hello"), "text").unwrap();
    fs::write(directory.0.join("binary.bin"), b"\0").unwrap();
    let shared = Transcript::shared().unwrap();
    let mut browser =
        Browser::open_empty_with_transcript(&directory.0, Arc::new(|| {}), shared.clone()).unwrap();
    for command in ["ls -1", "ls -l"] {
        browser.execute(command).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while browser.working() {
            browser.poll();
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        let mut t = shared.lock().unwrap();
        for i in 0..t.len() {
            let record = t.read(i).unwrap();
            let RecordData::Native(bytes) = record.data else {
                continue;
            };
            let row = decode_native(&bytes).unwrap();
            if row.path.as_ref() == Some(&directory.0.join("binary.bin")) {
                assert!(native_name_range(&bytes).unwrap().is_none());
            } else if row.path.as_ref() == Some(&directory.0.join(" leading é.rs")) {
                let range = native_name_range(&bytes).unwrap().unwrap();
                assert_eq!(&row.text[range], " leading é.rs");
            } else if row.path.as_ref() == Some(&directory.0.join("d 42 hello")) {
                let range = native_name_range(&bytes).unwrap().unwrap();
                assert_eq!(&row.text[range], "d 42 hello");
            }
        }
    }
    let (_, rows) = view(shared, 80);
    assert!(style(&rows, "leading").underline);
    assert!(!style(&rows, "binary.bin").underline);
}

#[test]
fn live_wrapped_location_underlines_saved_prefix_without_filesystem_probes() {
    let shared = Transcript::shared().unwrap();
    let source;
    {
        let mut t = shared.lock().unwrap();
        source = t.next_source();
        t.append_header(&cwd(), source, "cargo check").unwrap();
        t.append_terminal(&cwd(), source, 80, true, "  --> src/東京 ".as_bytes())
            .unwrap();
    }
    let (mut layout, _) = view(shared.clone(), 80);
    let mut parser = vt100::Parser::new(2, 80, 0);
    parser.process(b"notes.rs:3:7");
    layout.set_tail(parser.screen(), 0, 1, source).unwrap();
    let rows = visible(&mut layout);
    assert!(!style(&rows, "-->").underline);
    assert!(style(&rows, "src/").underline);
    assert!(style(&rows, "notes.rs").underline);
    let record = shared.lock().unwrap().read_id(1).unwrap();
    let RecordData::Terminal { formatted, .. } = record.data else {
        panic!()
    };
    assert!(
        stored_style(&formatted).1.is_none(),
        "preview is an immutable bounded overlay"
    );
}

#[test]
fn saved_git_log_underlines_only_the_commit_hash() {
    let directory = Directory::new();
    fs::write(directory.0.join("file.rs"), "test").unwrap();
    for arguments in [
        vec!["init", "--quiet"],
        vec!["add", "--", "file.rs"],
        vec!["commit", "--quiet", "-m", "message stays plain"],
    ] {
        let output = std::process::Command::new("git")
            .current_dir(&directory.0)
            .args([
                "-c",
                "user.name=Style Test",
                "-c",
                "user.email=style@example.invalid",
                "-c",
                "commit.gpgSign=false",
                "-c",
                "core.hooksPath=.no-hooks",
            ])
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let shared = Transcript::shared().unwrap();
    let mut browser =
        Browser::open_empty_with_transcript(&directory.0, Arc::new(|| {}), shared.clone()).unwrap();
    browser.execute("git log --oneline").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while browser.working() {
        browser.poll();
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    let (_, rows) = view(shared, 80);
    let row = rows
        .iter()
        .find(|row| row.text.contains("message stays plain"))
        .unwrap();
    let hash = row.text.split_whitespace().next().unwrap();
    assert!(style(&rows, hash).underline);
    assert!(!style(&rows, "message stays plain").underline);
}

#[test]
fn rejected_terminal_rows_do_not_change_storage_or_later_color_context() {
    let shared = Transcript::shared().unwrap();
    let mut t = shared.lock().unwrap();
    let source = t.next_source();
    t.append_header(&cwd(), source, "cargo check").unwrap();
    let before = (t.len(), t.serial(), t.base_id());
    let mut oversized = b"warning: rejected".to_vec();
    oversized.resize(64 * 1024 + 1, b' ');
    assert!(
        t.append_terminal(&cwd(), source, 80, false, &oversized)
            .is_err()
    );
    assert!(
        t.append_terminal(
            std::path::Path::new("relative"),
            source,
            80,
            false,
            b"warning: rejected"
        )
        .is_err()
    );
    let huge_directory = cwd().join("x".repeat(256 * 1024));
    assert!(
        t.append_terminal(&huge_directory, source, 80, false, b"warning: rejected")
            .is_err()
    );
    assert_eq!((t.len(), t.serial(), t.base_id()), before);
    let anchor = t
        .append_terminal(&cwd(), source, 80, false, b"  | ordinary")
        .unwrap();
    drop(t);
    assert_eq!(tone(&shared, anchor.record_id), None);
}

#[test]
fn exact_legacy_palette_and_native_git_colors_apply_only_to_names() {
    use super::super::browser::{Kind, native_entry_range};
    for (tone, rgb) in [
        (Tone::Added, (150, 220, 165)),
        (Tone::Removed, (245, 145, 145)),
        (Tone::Header, (225, 195, 120)),
        (Tone::Hunk, (120, 195, 235)),
        (Tone::Warning, (235, 185, 110)),
    ] {
        assert_eq!(tone.rgb(), rgb);
    }
    for kind in [
        Kind::Text,
        Kind::Binary,
        Kind::Directory,
        Kind::Unreadable,
        Kind::Header,
    ] {
        assert_eq!(kind.color(), (215, 220, 215));
    }
    let directory = Directory::new();
    fs::write(directory.0.join("text.rs"), "text").unwrap();
    fs::write(directory.0.join("binary.bin"), b"\0").unwrap();
    fs::create_dir(directory.0.join("folder")).unwrap();
    let source = Transcript::shared().unwrap();
    let mut browser =
        Browser::open_empty_with_transcript(&directory.0, Arc::new(|| {}), source.clone()).unwrap();
    browser.execute("ls -l").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while browser.working() {
        browser.poll();
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    let target = Transcript::shared().unwrap();
    let mut expected = Vec::new();
    let mut source = source.lock().unwrap();
    for index in 0..source.len() {
        let record = source.read(index).unwrap();
        let RecordData::Native(mut bytes) = record.data else {
            continue;
        };
        let row = decode_native(&bytes).unwrap();
        if row.path.is_none() {
            continue;
        }
        assert_eq!(row.color, Some((215, 220, 215)));
        let name = native_entry_range(&bytes).unwrap().unwrap();
        // Persist a Modified filename decoration; Layout must not color its
        // size prefix or perform any filesystem query to reconstruct styling.
        bytes[10] = 1;
        bytes[11..14].copy_from_slice(&[225, 185, 115]);
        let id = target
            .lock()
            .unwrap()
            .append_native(&directory.0, 1, &bytes)
            .unwrap()
            .record_id;
        expected.push((id, name, matches!(row.kind, Kind::Text | Kind::Directory)));
    }
    drop(source);
    let (_, rows) = view(target, 80);
    for (id, name, underline) in expected {
        let row = rows
            .iter()
            .find(|row| row.segments.iter().any(|s| s.record_id == id))
            .unwrap();
        let prefix = row
            .segments
            .iter()
            .find(|s| s.start.utf8_byte_offset == 0)
            .unwrap();
        assert_eq!(prefix.style, super::super::layout::Style::default());
        assert_eq!(prefix.color, None);
        let filename = row
            .segments
            .iter()
            .find(|s| s.start.utf8_byte_offset <= name.start && name.start < s.end.utf8_byte_offset)
            .unwrap();
        assert_eq!(filename.style.fg, vt100::Color::Rgb(225, 185, 115));
        assert_eq!(filename.color, Some((225, 185, 115)));
        assert_eq!(filename.style.underline, underline);
    }
}

#[test]
fn native_git_classifier_uses_exact_legacy_prefix_and_section_rules() {
    let mut section = None;
    for (line, expected) in [
        ("On branch main", Some(Tone::Header)),
        ("Changes to be committed:", Some(Tone::Header)),
        ("\tnew file: tracked.rs", Some(Tone::Added)),
        ("Changes not staged for commit:", Some(Tone::Header)),
        ("\tmodified: tracked.rs", Some(Tone::Removed)),
        ("Untracked files:", Some(Tone::Header)),
        ("\tuntracked.rs", Some(Tone::Warning)),
        ("A  tracked.rs", Some(Tone::Added)),
        (" M tracked.rs", Some(Tone::Removed)),
        ("UU tracked.rs", Some(Tone::Warning)),
        ("?? untracked.rs", Some(Tone::Warning)),
        ("commit x", Some(Tone::Header)),
        ("* deadbeef", None),
        ("* deadbeef subject", Some(Tone::Header)),
        ("diff --git a/file b/file", Some(Tone::Header)),
        ("@@ -1 +1 @@", Some(Tone::Hunk)),
        ("+added", Some(Tone::Added)),
        ("-removed", Some(Tone::Removed)),
        (" context", None),
        ("fatal: failed", Some(Tone::Removed)),
    ] {
        assert_eq!(
            git_line(line, &mut section),
            expected.map(Tone::rgb),
            "{line}"
        );
    }
}

#[test]
fn legacy_ansi_neutral_style_is_shared_by_saved_output_and_live_tail() {
    let shared = Transcript::shared().unwrap();
    let source;
    {
        let mut t = shared.lock().unwrap();
        source = t.next_source();
        t.append_header(&cwd(), source, "printf colors").unwrap();
        t.append_terminal(
            &cwd(),
            source,
            80,
            false,
            b"\x1b[1;3;4;7;31;44msaved\x1b[0m",
        )
        .unwrap();
        let record = t.read_id(1).unwrap();
        let RecordData::Terminal { formatted, .. } = record.data else {
            panic!()
        };
        let mut parser = vt100::Parser::new(1, 80, 0);
        parser.process(&formatted);
        assert!(
            parser.screen().cell(0, 0).unwrap().bold(),
            "raw data remains VT formatted"
        );
    }
    let (mut layout, rows) = view(shared, 80);
    assert_eq!(
        style(&rows, "saved"),
        super::super::layout::Style::default()
    );
    let mut parser = vt100::Parser::new(2, 80, 0);
    parser.process(b"\x1b[1;3;4;7;31;44mlive\x1b[0m");
    layout.set_tail(parser.screen(), 0, 1, source).unwrap();
    let rows = visible(&mut layout);
    assert_eq!(style(&rows, "live"), super::super::layout::Style::default());
}

#[test]
fn completion_metadata_keeps_only_the_final_legacy_empty_eof_row() {
    let shared = Transcript::shared().unwrap();
    {
        let mut t = shared.lock().unwrap();
        for command in ["printf first", "$ printf second"] {
            let source = t.next_source();
            t.append_header(&cwd(), source, command).unwrap();
            t.append_terminal(&cwd(), source, 80, false, b"output")
                .unwrap();
            t.append_result(&cwd(), source, 0, "Done").unwrap();
        }
    }
    let (layout, rows) = view(shared.clone(), 80);
    assert_eq!(layout.len(), 5);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["$ printf first", "output", "$ printf second", "output", ""]
    );
    assert_eq!(
        shared.lock().unwrap().len(),
        6,
        "proof records stay on disk"
    );
    shared.lock().unwrap().clear().unwrap();
    shared
        .lock()
        .unwrap()
        .append_result(&cwd(), 1, 0, "Done")
        .unwrap();
    let (layout, rows) = view(shared, 80);
    assert_eq!(layout.len(), 1);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].text.is_empty());
}

#[test]
fn appending_after_completed_projection_replaces_eof_without_a_blank_between_commands() {
    let shared = Transcript::shared().unwrap();
    {
        let mut store = shared.lock().unwrap();
        let source = store.next_source();
        store.append_header(&cwd(), source, "first").unwrap();
        store
            .append_terminal(&cwd(), source, 80, false, b"first output")
            .unwrap();
        store.append_result(&cwd(), source, 0, "Done").unwrap();
    }
    let (mut layout, initial) = view(shared.clone(), 80);
    assert_eq!(
        initial
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
        ["$ first", "first output", ""]
    );
    {
        let mut store = shared.lock().unwrap();
        let source = store.next_source();
        store.append_header(&cwd(), source, "second").unwrap();
        store
            .append_terminal(&cwd(), source, 80, false, b"second output")
            .unwrap();
        store.append_result(&cwd(), source, 0, "Done").unwrap();
    }
    let rows = visible(&mut layout);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["$ first", "first output", "$ second", "second output", ""]
    );
    assert_eq!(layout.len(), 5);
}

#[test]
fn native_fixed_columns_survive_reflow_and_preserve_filename_style_ranges() {
    let directory = Directory::new();
    fs::write(directory.0.join("a.rs"), "text").unwrap();
    fs::write(directory.0.join("b.rs"), "text").unwrap();
    let shared = Transcript::shared().unwrap();
    let mut browser =
        Browser::open_empty_with_transcript(&directory.0, Arc::new(|| {}), shared.clone()).unwrap();
    browser.set_columns(80);
    browser.execute("ls").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while browser.working() {
        browser.poll();
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    let (mut layout, wide) = view(shared.clone(), 80);
    let line = wide
        .iter()
        .find(|row| row.text.contains("a.rs") && row.text.contains("b.rs"))
        .expect("stored columns share one row");
    assert!(line.text.contains("a.rs  ") || line.text.contains("b.rs  "));
    assert!(style(&wide, "a.rs").underline);
    let name = line
        .segments
        .iter()
        .find(|segment| line.text[segment.range.clone()].contains("a.rs"))
        .unwrap();
    assert!(name.style.underline);
    assert!(line.segments.iter().any(|segment| !segment.style.underline
        && line.text[segment.range.clone()].chars().all(|ch| ch == ' ')));
    let disk_rows = {
        let mut store = shared.lock().unwrap();
        let len = store.len();
        (0..len)
            .filter_map(|index| {
                let record = store.read(index).unwrap();
                let RecordData::Native(bytes) = record.data else {
                    return None;
                };
                let row = decode_native(&bytes).unwrap();
                row.path.map(|_| (bytes, row.text))
            })
            .collect::<Vec<_>>()
    };
    assert!(disk_rows.iter().all(|(bytes, _)| bytes[14] & 16 != 0));
    let saved = disk_rows
        .iter()
        .map(|(bytes, text)| {
            let mut s = text.clone();
            s.extend(std::iter::repeat_n(' ', bytes[15] as usize));
            s
        })
        .collect::<String>();
    assert_eq!(line.text, saved);
    layout.configure(Metrics {
        columns: 10,
        tab_width: 4,
    });
    let narrow = visible(&mut layout);
    let names = narrow
        .iter()
        .filter(|row| row.segments.iter().any(|segment| segment.native))
        .map(|row| row.text.as_str())
        .collect::<String>();
    assert_eq!(
        names, saved,
        "resize reflows persisted whitespace without repacking filenames"
    );
}

#[test]
fn failed_command_footer_projects_eof_then_removes_it_on_append() {
    let shared = Transcript::shared().unwrap();
    {
        let mut store = shared.lock().unwrap();
        let source = store.next_source();
        store.append_header(&cwd(), source, "false").unwrap();
        store.append_result(&cwd(), source, 7, "Exit 7").unwrap();
    }
    let (mut layout, rows) = view(shared.clone(), 80);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["$ false", "[finished with exit code 7]", ""]
    );
    {
        let mut store = shared.lock().unwrap();
        let source = store.next_source();
        store.append_header(&cwd(), source, "next").unwrap();
        store.append_result(&cwd(), source, 0, "Done").unwrap();
    }
    let rows = visible(&mut layout);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["$ false", "[finished with exit code 7]", "$ next", ""]
    );
}

#[test]
fn plain_command_rows_use_legacy_tab_width_trailing_spaces_and_exact_link_style() {
    let shared = Transcript::shared().unwrap();
    {
        let mut store = shared.lock().unwrap();
        let source = store.next_source();
        store.append_header(&cwd(), source, "cargo check").unwrap();
        store
            .append_plain(&cwd(), source, "a\ttrail  ", false)
            .unwrap();
        store
            .append_plain(&cwd(), source, "error[E0308]: mismatch  ", false)
            .unwrap();
        store
            .append_plain(&cwd(), source, "  --> src/東京 path.rs:3:7  ", false)
            .unwrap();
        store.append_result(&cwd(), source, 0, "Done").unwrap();
    }
    let (_, rows) = view(shared.clone(), 80);
    assert!(rows.iter().any(|row| row.text == "a   trail  "));
    assert_eq!(
        style(&rows, "error[E0308]").fg,
        vt100::Color::Rgb(245, 145, 145)
    );
    assert!(!style(&rows, "-->").underline);
    assert!(style(&rows, "path.rs").underline);
    let path = rows
        .iter()
        .find(|row| row.text.contains("path.rs"))
        .unwrap();
    assert!(path.text.ends_with("  "));
    assert!(!path.segments.last().unwrap().style.underline);
    let record = shared.lock().unwrap().read(1).unwrap();
    assert_eq!(record_text(&record).unwrap().text, "a\ttrail  ");
}
