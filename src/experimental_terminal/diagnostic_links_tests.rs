// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::super::transcript::Transcript;
use super::*;

struct Fixture {
    directory: PathBuf,
    store: SharedTranscript,
    provenance: Provenance,
}
impl Fixture {
    fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("potyi-diagnostic-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let store = Transcript::shared().unwrap();
        let provenance = {
            let mut transcript = store.lock().unwrap();
            let source = transcript.next_source();
            transcript
                .append_header(&directory, source, "compiler")
                .unwrap();
            Provenance {
                epoch: transcript.epoch(),
                source,
                cwd: directory.clone(),
                relative_safe: true,
            }
        };
        Self {
            directory,
            store,
            provenance,
        }
    }
    fn file(&self, name: &str) -> PathBuf {
        let path = self.directory.join(name);
        std::fs::write(&path, "test").unwrap();
        path
    }
    fn row(&self, text: &str, wrapped: bool) -> Anchor {
        self.store
            .lock()
            .unwrap()
            .append_terminal(
                &self.directory,
                self.provenance.source,
                256,
                wrapped,
                text.as_bytes(),
            )
            .unwrap()
    }
    fn finish(&self, cwd: &Path, safe: bool) {
        self.store
            .lock()
            .unwrap()
            .append_result_with_provenance(cwd, self.provenance.source, 0, "Exit 0", safe)
            .unwrap();
    }
    fn lookup(&self, mut anchor: Anchor, byte: usize) -> io::Result<Option<FileLink>> {
        anchor.utf8_byte_offset = byte;
        resolve(
            anchor,
            &self.store,
            &mut DiskSource::new(self.store.clone()),
            &self.provenance,
            &AtomicBool::new(false),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn quoted_and_spaced_unicode_diagnostics_keep_numeric_suffix_and_click_range() {
    let f = Fixture::new("quoted");
    let path = f.file("é notes.rs");
    let a = f.row("error: \"é notes.rs\":12:3: failed", false);
    let b = f.row("é notes.rs:18:2: failed", false);
    let c = f.row("warning: 'é notes.rs:21:4' failed", false);
    let d = f.row("é notes.rs(9,5): error", false);
    f.finish(&f.directory, true);
    for (anchor, byte, line, column) in [(a, 8, 12, 3), (b, 4, 18, 2), (c, 12, 21, 4), (d, 4, 9, 5)]
    {
        let link = f
            .lookup(anchor, byte)
            .unwrap()
            .unwrap_or_else(|| panic!("no link at record {} byte {byte}", anchor.record_id));
        assert_eq!(link.path, path);
        assert_eq!(link.line, Some(line));
        assert_eq!(link.column, Some(column));
    }
    assert!(
        f.lookup(a, 0).unwrap().is_none(),
        "error prefix is outside the filename"
    );
    assert!(
        f.lookup(a, 27).unwrap().is_none(),
        "message text is outside the filename"
    );
    assert!(
        f.lookup(a, "error: \"é notes.rs\":12:3: failed".len())
            .unwrap()
            .is_none(),
        "padding must never activate a link"
    );
    let e = f.row("é notes.rs:12:3abc", false);
    f.finish(&f.directory, true);
    assert!(
        f.lookup(e, 4).unwrap().is_none(),
        "numbers embedded in text are not a diagnostic location"
    );
}

#[test]
fn physical_soft_wrap_and_visual_byte_anchors_resolve_as_one_line() {
    let f = Fixture::new("wrap");
    let path = f.file("long file é.rs");
    let a = f.row("error: \"long ", true);
    let b = f.row("file é.rs\":32", true);
    let c = f.row(":7: failed", false);
    f.finish(&f.directory, true);
    for (anchor, byte) in [(a, 9), (b, 2), (c, 1)] {
        let link = f.lookup(anchor, byte).unwrap().unwrap();
        assert_eq!(link.path, path);
        assert_eq!((link.line, link.column), (Some(32), Some(7)));
    }
}

#[cfg(unix)]
#[test]
fn existing_literal_colon_filename_takes_precedence_over_location_parsing() {
    let f = Fixture::new("colon");
    let path = f.file("literal:12:3");
    f.file("literal");
    let a = f.row("literal:12:3", false);
    f.finish(&f.directory, true);
    let link = f.lookup(a, 2).unwrap().unwrap();
    assert_eq!(link.path, path);
    assert_eq!((link.line, link.column), (None, None));
}

#[test]
fn relative_diagnostics_require_completed_matching_origin_and_safety_flag() {
    let f = Fixture::new("provenance");
    f.file("file.rs");
    let a = f.row("file.rs:2:4", false);
    assert!(f.lookup(a, 2).unwrap().is_none());
    f.finish(&f.directory, false);
    assert!(f.lookup(a, 2).unwrap().is_none());
    let g = Fixture::new("cwd");
    g.file("file.rs");
    let b = g.row("file.rs:2:4", false);
    g.finish(&f.directory, true);
    assert!(g.lookup(b, 2).unwrap().is_none());
    let h = Fixture::new("safe");
    h.file("file.rs");
    let c = h.row("file.rs:2:4", false);
    h.finish(&h.directory, true);
    assert!(h.lookup(c, 2).unwrap().is_some());
    let mut provenance = h.provenance.clone();
    provenance.relative_safe = false;
    assert!(
        resolve(
            Anchor {
                utf8_byte_offset: 2,
                ..c
            },
            &h.store,
            &mut DiskSource::new(h.store.clone()),
            &provenance,
            &AtomicBool::new(false)
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn absolute_live_diagnostics_work_but_source_boundaries_never_join() {
    let f = Fixture::new("absolute");
    let path = f.file("absolute.rs");
    let text = format!("{}:8:2", path.display());
    let a = f.row(&text, false);
    let link = f.lookup(a, 2).unwrap().unwrap();
    assert_eq!(link.path, path);
    assert_eq!((link.line, link.column), (Some(8), Some(2)));
    f.file("combined.rs");
    let b = f.row("combined", true);
    let other = f.store.lock().unwrap().next_source();
    f.store
        .lock()
        .unwrap()
        .append_terminal(&f.directory, other, 256, false, b".rs:3:1")
        .unwrap();
    f.finish(&f.directory, true);
    assert!(f.lookup(b, 2).unwrap().is_none());
}

#[test]
fn cancellation_clear_stale_unicode_offset_and_hard_newlines_are_safe() {
    let f = Fixture::new("cancel");
    f.file("é.rs");
    let a = f.row("é.rs:1:2", false);
    f.finish(&f.directory, true);
    assert!(f.lookup(a, 1).unwrap().is_none(), "cannot split UTF-8");
    assert!(
        resolve(
            a,
            &f.store,
            &mut DiskSource::new(f.store.clone()),
            &f.provenance,
            &AtomicBool::new(true)
        )
        .is_err()
    );
    f.store.lock().unwrap().clear().unwrap();
    assert!(f.lookup(a, 0).unwrap().is_none());
    let g = Fixture::new("hardline");
    g.file("file.rs");
    let b = g.row("file", false);
    g.row(".rs:1:2", false);
    g.finish(&g.directory, true);
    assert!(g.lookup(b, 1).unwrap().is_none());
}

#[test]
fn logical_line_size_and_row_limits_reject_partial_filename_fragments() {
    struct Huge {
        bounds: Bounds,
        reads: usize,
    }
    impl TextSource for Huge {
        fn bounds(&self) -> Bounds {
            self.bounds
        }
        fn text(&mut self, _: u64) -> io::Result<RecordText> {
            self.reads += 1;
            Ok(RecordText {
                text: "x".repeat(512),
                join_next: true,
                omitted: false,
                terminated: false,
            })
        }
    }
    let f = Fixture::new("bounded");
    let mut source = Huge {
        bounds: Bounds {
            epoch: f.provenance.epoch,
            first: 1000,
            end: 2000,
        },
        reads: 0,
    };
    let click = Anchor {
        epoch: source.bounds.epoch,
        record_id: 1500,
        utf8_byte_offset: 1,
    };
    assert!(
        resolve(
            click,
            &f.store,
            &mut source,
            &f.provenance,
            &AtomicBool::new(false)
        )
        .unwrap()
        .is_none()
    );
    assert!(source.reads <= MAX_LINE_ROWS + 1);
    assert!(source.reads <= MAX_LINE_BYTES / 512 + 1);
}

#[test]
fn job_returns_one_result_and_has_no_permanent_index() {
    let f = Fixture::new("job");
    let path = f.file("job.rs");
    let a = f.row("job.rs:4:2", false);
    f.finish(&f.directory, true);
    let mut job = Job::start(
        a,
        f.store.clone(),
        None,
        f.provenance.clone(),
        Arc::new(|| {}),
    );
    let start = std::time::Instant::now();
    let result = loop {
        if let Some(result) = job.poll() {
            break result.unwrap().unwrap();
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert_eq!(result.path, path);
    assert!(!job.pending());
    assert!(job.poll().is_none());
}

fn set_command(f: &Fixture, command: &str) {
    f.store.lock().unwrap().clear().unwrap();
    // All contextual fixtures below rebuild their own current provenance.
    f.store
        .lock()
        .unwrap()
        .append_header(&f.directory, f.provenance.source, command)
        .unwrap();
}

#[test]
fn grep_single_file_and_rg_byte_columns_use_retained_command_context() {
    for (name, command, output, expected_column, byte_column) in [
        (
            "grep-context",
            "grep -n needle 'é notes.rs'",
            "12:needle:34:text",
            None,
            false,
        ),
        (
            "rg-context",
            "rg --column needle 'é notes.rs'",
            "12:4:needle",
            Some(4),
            true,
        ),
        (
            "rg-no-column",
            "rg -n needle 'é notes.rs'",
            "é notes.rs:12:4:text",
            None,
            true,
        ),
    ] {
        let mut f = Fixture::new(name);
        let path = f.file("é notes.rs");
        set_command(&f, command);
        f.provenance.epoch = f.store.lock().unwrap().epoch();
        let a = f.row(output, false);
        f.finish(&f.directory, true);
        let link = f.lookup(a, output.len() - 2).unwrap().unwrap();
        assert_eq!(link.path, path);
        assert_eq!(
            (link.line, link.column, link.byte_column),
            (Some(12), expected_column, byte_column)
        );
    }
    let mut f = Fixture::new("grep-pipe");
    f.file("é notes.rs");
    set_command(&f, "grep -n needle 'é notes.rs' | cat");
    f.provenance.epoch = f.store.lock().unwrap().epoch();
    let a = f.row("12:needle", false);
    f.finish(&f.directory, true);
    assert!(
        f.lookup(a, 4).unwrap().is_none(),
        "pipeline cannot infer an omitted filename"
    );
}

#[test]
fn rust_panic_secondary_and_workspace_paths_keep_location_ranges_and_character_columns() {
    let mut f = Fixture::new("cargo-context");
    let member = f.directory.join("member");
    std::fs::create_dir_all(&member).unwrap();
    let path = f.file("workspace é.rs");
    f.store.lock().unwrap().clear().unwrap();
    f.provenance.epoch = f.store.lock().unwrap().epoch();
    f.provenance.cwd = member.clone();
    f.store
        .lock()
        .unwrap()
        .append_header(&member, f.provenance.source, "cargo check")
        .unwrap();
    let mut anchors = Vec::new();
    for line in [
        "  --> workspace é.rs:42:7",
        "  ::: workspace é.rs:42:7",
        "thread 'main' panicked at workspace é.rs:42:7:",
    ] {
        let a = f
            .store
            .lock()
            .unwrap()
            .append_terminal(&member, f.provenance.source, 256, false, line.as_bytes())
            .unwrap();
        anchors.push((a, line));
    }
    f.finish(&member, true);
    for (a, line) in anchors {
        let offset = line.find("workspace").unwrap();
        let link = f.lookup(a, offset).unwrap().unwrap();
        assert_eq!(link.path, path);
        assert_eq!(
            (link.line, link.column, link.byte_column),
            (Some(42), Some(7), false)
        );
        assert!(
            f.lookup(a, 0).unwrap().is_none(),
            "marker/panic prefix is outside the location"
        );
    }
}

#[test]
fn cached_underlining_is_syntactic_bounded_and_keeps_legacy_command_rules() {
    for (command, line, token) in [
        (
            "cargo check",
            "  --> missing 東京.rs:42:7",
            "missing 東京.rs:42:7",
        ),
        ("cargo check", "  ::: missing.rs:42:7", "missing.rs:42:7"),
        (
            "cargo run",
            "thread 'main' panicked at missing.rs:42:7:",
            "missing.rs:42:7",
        ),
        ("gcc main.c", "missing.c:4:2: error", "missing.c:4:2: error"),
        ("grep -n needle missing.rs", "42:needle", "42:needle"),
        (
            "rg --column needle missing.rs",
            "42:7:needle",
            "42:7:needle",
        ),
    ] {
        let context = CommandContext::new(command).clone();
        let range = context.underline(line).unwrap();
        assert_eq!(&line[range], token, "{command}: {line}");
        assert!(context.underline(&"x".repeat(MAX_LINE_BYTES + 1)).is_none());
    }
    for (command, line) in [
        ("grep needle missing.rs", "42:needle"),
        ("grep -n needle missing.rs | cat", "42:needle"),
        ("grep -n needle first.rs second.rs", "42:needle"),
        ("grep -n needle '*.rs'", "42:needle"),
        ("cargo check", " --> missing.rs:0:7"),
        ("cargo check", " --> <anon>:42:7"),
        ("gcc main.c", "missing.c:no:2"),
    ] {
        assert!(
            CommandContext::new(command).underline(line).is_none(),
            "{command}: {line}"
        );
    }
    assert!(
        CommandContext::new(&"x".repeat(MAX_LINE_BYTES + 1))
            .underline("42:needle")
            .is_none()
    );
}

#[test]
fn compiler_fallback_is_missing_only_literal_and_at_most_thirty_two_ancestors() {
    let mut f = Fixture::new("cargo-fallback-bound");
    let mut member = f.directory.clone();
    for _ in 0..33 {
        member.push("m");
    }
    std::fs::create_dir_all(&member).unwrap();
    let root_only = f.file("root-only.rs");
    let near = member.parent().unwrap().join("near.rs");
    let exact = member.join("near.rs");
    std::fs::write(&near, "ancestor").unwrap();
    std::fs::write(&exact, "exact").unwrap();
    f.store.lock().unwrap().clear().unwrap();
    f.provenance.epoch = f.store.lock().unwrap().epoch();
    f.provenance.cwd = member.clone();
    f.store
        .lock()
        .unwrap()
        .append_header(&member, f.provenance.source, "cargo check")
        .unwrap();
    let mut anchors = Vec::new();
    for text in [
        "--> root-only.rs:1:1",
        "--> near.rs:1:1",
        "--> ../root-only.rs:1:1",
    ] {
        anchors.push(
            f.store
                .lock()
                .unwrap()
                .append_terminal(&member, f.provenance.source, 256, false, text.as_bytes())
                .unwrap(),
        );
    }
    f.finish(&member, true);
    assert!(
        f.lookup(anchors[0], 4).unwrap().is_none(),
        "33rd ancestor is outside bounded fallback"
    );
    assert_eq!(
        f.lookup(anchors[1], 4).unwrap().unwrap().path,
        exact,
        "existing exact path takes precedence"
    );
    assert!(
        f.lookup(anchors[2], 4).unwrap().is_none(),
        "explicit parent components remain literal"
    );
    assert!(root_only.is_file());
}

#[test]
fn current_search_context_survives_real_header_eviction_without_retaining_old_sources() {
    for (name, command, output, column, byte_column) in [
        (
            "grep-context-eviction",
            "grep -n needle 'é notes.rs'",
            "12:needle",
            None,
            false,
        ),
        (
            "rg-context-eviction",
            "rg --column needle 'é notes.rs'",
            "12:4:needle",
            Some(4),
            true,
        ),
    ] {
        let mut f = Fixture::new(name);
        let path = f.file("é notes.rs");
        f.store.lock().unwrap().clear().unwrap();
        f.provenance.epoch = f.store.lock().unwrap().epoch();
        let header = f
            .store
            .lock()
            .unwrap()
            .append_header(&f.directory, f.provenance.source, command)
            .unwrap();
        // A valid formatted physical row: inert padding consumes disk budget
        // without printing more cells than the row's declared width.
        let mut filler = vec![0; MAX_LINE_BYTES];
        filler[MAX_LINE_BYTES - 1] = b'x';
        {
            let mut store = f.store.lock().unwrap();
            for _ in 0..140 {
                store
                    .append_terminal(&f.directory, f.provenance.source, 256, false, &filler)
                    .unwrap();
            }
            assert!(
                header.record_id < store.base_id(),
                "header must actually evict from 8MiB ring"
            );
        }
        let anchor = f.row(output, false);
        f.finish(&f.directory, true);
        let link = f.lookup(anchor, output.len() - 1).unwrap().unwrap();
        assert_eq!(link.path, path);
        assert_eq!(
            (link.line, link.column, link.byte_column),
            (Some(12), column, byte_column)
        );
        let mut store = f.store.lock().unwrap();
        let next_source = store.next_source();
        store
            .append_header(&f.directory, next_source, "printf unrelated")
            .unwrap();
        drop(store);
        assert!(
            f.lookup(anchor, output.len() - 1).unwrap().is_none(),
            "evicted historical header cannot borrow a newer command context"
        );
    }
}

// Wait until the worker has queued its answer without allowing UI publication.
// This makes the worker-finished-to-publication boundary deterministic.
fn finished_worker(fixture: &Fixture, click: Anchor) -> Job {
    let ready = Arc::new(AtomicBool::new(false));
    let signal = ready.clone();
    let job = Job::start(
        click,
        fixture.store.clone(),
        None,
        fixture.provenance.clone(),
        Arc::new(move || signal.store(true, Ordering::Release)),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !ready.load(Ordering::Acquire) {
        assert!(
            std::time::Instant::now() < deadline,
            "Diagnostic worker timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    job
}
fn last_result(fixture: &Fixture) -> Anchor {
    let store = fixture.store.lock().unwrap();
    Anchor {
        epoch: store.epoch(),
        record_id: store.base_id() + store.len() as u64 - 1,
        utf8_byte_offset: 0,
    }
}

#[test]
fn completed_relative_lookup_rechecks_a_late_result_veto_before_publication() {
    let f = Fixture::new("publication-veto");
    f.file("notes é.rs");
    let click = f.row("notes é.rs:12:4", false);
    f.finish(&f.directory, true);
    let mut job = finished_worker(&f, click);
    let result = last_result(&f);
    f.store.lock().unwrap().invalidate_result(result).unwrap();
    assert!(job.poll().unwrap().unwrap().is_none());
    assert!(!job.pending());
    assert!(job.poll().is_none());
}

#[test]
fn completed_absolute_lookup_remains_valid_after_a_relative_result_veto() {
    let f = Fixture::new("publication-absolute");
    let path = f.file("notes é.rs");
    let click = f.row(&format!("{}:12:4", path.display()), false);
    f.finish(&f.directory, true);
    let mut job = finished_worker(&f, click);
    let result = last_result(&f);
    f.store.lock().unwrap().invalidate_result(result).unwrap();
    let link = job.poll().unwrap().unwrap().unwrap();
    assert_eq!(link.path, path);
    assert_eq!((link.line, link.column), (Some(12), Some(4)));
}

#[test]
fn completed_lookup_is_discarded_if_clear_precedes_publication() {
    let f = Fixture::new("publication-clear");
    f.file("file.rs");
    let click = f.row("file.rs:2:3", false);
    f.finish(&f.directory, true);
    let mut job = finished_worker(&f, click);
    f.store.lock().unwrap().clear().unwrap();
    assert!(job.poll().unwrap().unwrap().is_none());
}

#[test]
fn completed_lookup_is_discarded_if_its_row_is_evicted_before_publication() {
    let f = Fixture::new("publication-eviction");
    f.file("file.rs");
    let click = f.row("file.rs:2:3", false);
    f.finish(&f.directory, true);
    let mut job = finished_worker(&f, click);
    let fill = "x".repeat(64 * 1024);
    {
        let mut store = f.store.lock().unwrap();
        let source = store.next_source();
        for _ in 0..150 {
            store.append_header(&f.directory, source, &fill).unwrap();
        }
        assert!(store.base_id() > click.record_id);
    }
    assert!(job.poll().unwrap().unwrap().is_none());
}
