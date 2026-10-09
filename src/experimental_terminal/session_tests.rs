use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

fn shell(directory: &Path) -> Session {
    #[cfg(unix)]
    let mut command = CommandBuilder::new("/bin/sh");
    #[cfg(windows)]
    let mut command = CommandBuilder::new("cmd.exe");
    #[cfg(unix)]
    {
        command.arg("-i");
        command.env("PS1", "");
        command.env("ENV", "");
    }
    #[cfg(windows)]
    {
        command.args(["/D", "/Q"]);
        command.env("PROMPT", "$G");
    }
    Session::spawn(directory, command, 24, 100, Arc::new(|| {})).unwrap()
}

fn wait_for(session: &mut Session, expected: &str) {
    let start = Instant::now();
    loop {
        session.poll();
        let contents = if let Some(store) = &session.transcript {
            let mut contents = transcript_terminal_text(store).join("\n");
            if let Some((tail, _)) = session.plain_tail() {
                contents.push_str(tail);
            }
            contents
        } else {
            session.screen().contents()
        };
        if contents.contains(expected) {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "Missing {expected:?}; {contents:?}; {:?}",
            session.status
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn experimental_terminal_persistent_shell_accepts_interactive_input_and_keeps_environment() {
    let directory = std::env::current_dir().unwrap();
    let mut session = shell(&directory);
    #[cfg(unix)]
    {
        session.send(b"export POTYI_TERM_PROBE=kept; printf 'IN%s_READY\\n' PUT; read answer; printf 'ANS:%s:END\\n' \"$answer\"\r".to_vec()).unwrap();
        wait_for(&mut session, "INPUT_READY");
        session.send("héllo界\r".as_bytes().to_vec()).unwrap();
        wait_for(&mut session, "ANS:héllo界:END");
        session
            .send(b"printf 'VALUE:%s:END\\n' \"$POTYI_TERM_PROBE\"\r".to_vec())
            .unwrap();
    }
    #[cfg(windows)]
    {
        session
            .send(b"set POTYI_TERM_PROBE=kept\rset /p answer=INPUT_READY\r".to_vec())
            .unwrap();
        wait_for(&mut session, "INPUT_READY");
        session.send(b"hello\r".to_vec()).unwrap();
        session.send(b"echo ANS:%answer%:END\r".to_vec()).unwrap();
        wait_for(&mut session, "ANS:hello:END");
        session
            .send(b"echo VALUE:%POTYI_TERM_PROBE%:END\r".to_vec())
            .unwrap();
    }
    wait_for(&mut session, "VALUE:kept:END");
    session.send(b"exit\r".to_vec()).unwrap();
    let start = Instant::now();
    while !session.closed || !session.exited.load(Ordering::Acquire) {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    assert!(session.send(b"echo too late\r".to_vec()).is_err());
}

#[test]
fn experimental_terminal_cd_and_resize_reach_the_real_pty() {
    let directory = std::env::temp_dir().join(format!("potyi terminal {} é", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut session = shell(&std::env::current_dir().unwrap());
    session.resize(35, 91).unwrap();
    let size = session.master.as_ref().unwrap().get_size().unwrap();
    assert_eq!((size.rows, size.cols), (35, 91));
    #[cfg(unix)]
    {
        let path = directory.to_string_lossy().replace('\'', "'\\''");
        session
            .send(
                format!("cd '{path}'; printf 'DIR:'; pwd; printf 'SIZE:'; stty size\r")
                    .into_bytes(),
            )
            .unwrap();
        wait_for(&mut session, "SIZE:35 91");
    }
    #[cfg(windows)]
    session
        .send(format!("cd /D \"{}\"\recho DIR:%CD%\r", directory.display()).into_bytes())
        .unwrap();
    // macOS resolves /var to /private/var; compare the directory's unique name.
    wait_for(
        &mut session,
        directory.file_name().unwrap().to_str().unwrap(),
    );
    #[cfg(unix)]
    assert!(
        session
            .screen()
            .contents()
            .lines()
            .any(|line| line.starts_with("DIR:")
                && line.ends_with(directory.file_name().unwrap().to_str().unwrap()))
    );
    #[cfg(windows)]
    wait_for(&mut session, &format!("DIR:{}", directory.display()));
    drop(session);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn experimental_terminal_decodes_fragmented_unicode_overwrites_and_alternate_screen() {
    let mut parser = vt100::Parser::new_with_callbacks(3, 12, HISTORY_ROWS, Replies::default());
    let output = "old\rnew\x1b[31m界é\x1b[0m\x1b[6n";
    for byte in output.as_bytes() {
        parser.process(&[*byte]);
    }
    assert_eq!(parser.screen().contents(), "new界é");
    assert_eq!(
        parser.screen().cell(0, 3).unwrap().fgcolor(),
        vt100::Color::Idx(1)
    );
    assert_eq!(parser.callbacks().pending, vec![b"\x1b[1;7R".to_vec()]);
    parser.process(b"\x1b[?1049h\x1b[2J\x1b[Halternate");
    assert!(parser.screen().alternate_screen());
    assert_eq!(parser.screen().contents(), "alternate");
    parser.process(b"\x1b[?1049l");
    assert!(!parser.screen().alternate_screen());
    assert_eq!(parser.screen().contents(), "new界é");
    for index in 0..2000 {
        parser.process(format!("\r\n{index}").as_bytes());
    }
    parser.screen_mut().set_scrollback(usize::MAX);
    assert_eq!(parser.screen().scrollback(), HISTORY_ROWS);
}

#[test]
fn experimental_terminal_large_paste_is_rejected_atomically_and_empty_paste_is_idle() {
    let mut session = shell(&std::env::current_dir().unwrap());
    assert!(session.paste(&"x".repeat(MAX_PASTE_BYTES + 1)).is_err());
    assert!(session.paste("").is_ok());
    session.parser.process(b"\x1b[?2004h");
    assert!(session.screen().bracketed_paste());
    // Clipboard ESC bytes cannot terminate bracketed paste and inject commands.
    assert!(session.paste("safe\x1b[201~\0").is_ok());
}

#[test]
fn experimental_terminal_drop_terminates_and_reaps_its_shell() {
    let session = shell(&std::env::current_dir().unwrap());
    let exited = session.exited.clone();
    drop(session);
    let start = Instant::now();
    while !exited.load(Ordering::Acquire) {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Shell survived terminal closure"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn experimental_terminal_closing_a_flooded_session_releases_blocked_output_workers() {
    #[cfg(unix)]
    let mut command = CommandBuilder::new("/bin/sh");
    #[cfg(windows)]
    let mut command = CommandBuilder::new("cmd.exe");
    #[cfg(unix)]
    command.args([
        "-c",
        "while :; do printf 'flood-output-012345678901234567890123456789\\n'; done",
    ]);
    #[cfg(windows)]
    command.args([
        "/D",
        "/Q",
        "/C",
        "for /L %i in (1,1,1000000) do @echo flood-output-012345678901234567890123456789",
    ]);
    let session = Session::spawn(
        &std::env::current_dir().unwrap(),
        command,
        24,
        80,
        Arc::new(|| {}),
    )
    .unwrap();
    let exited = session.exited.clone();
    // Deliberately stop consuming output so producers hit queue backpressure.
    thread::sleep(Duration::from_millis(200));
    let start = Instant::now();
    drop(session);
    while !exited.load(Ordering::Acquire) {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Flooded shell survived terminal closure"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
#[test]
fn experimental_terminal_ctrl_c_interrupts_a_job_and_shell_remains_usable() {
    let mut session = shell(&std::env::current_dir().unwrap());
    session
        .send(b"printf 'RUN_%s\\n' START; sleep 30\r".to_vec())
        .unwrap();
    wait_for(&mut session, "RUN_START");
    thread::sleep(Duration::from_millis(100));
    session.send(vec![3]).unwrap();
    session.send(b"printf 'INT_%s\\n' OK\r".to_vec()).unwrap();
    wait_for(&mut session, "INT_OK");
}

#[test]
fn experimental_terminal_workers_coalesce_wakeups_and_quiet_session_has_no_work() {
    let count = Arc::new(AtomicUsize::new(0));
    let captured = count.clone();
    let directory = std::env::current_dir().unwrap();
    #[cfg(unix)]
    let mut command = CommandBuilder::new("/bin/sh");
    #[cfg(windows)]
    let mut command = CommandBuilder::new("cmd.exe");
    #[cfg(unix)]
    command.args(["-c", "printf 'WAKE_READY\\n'; sleep 1"]);
    #[cfg(windows)]
    command.args([
        "/D",
        "/Q",
        "/C",
        "echo WAKE_READY & ping -n 2 127.0.0.1 >NUL",
    ]);
    let mut session = Session::spawn(
        &directory,
        command,
        24,
        80,
        Arc::new(move || {
            captured.fetch_add(1, Ordering::Relaxed);
        }),
    )
    .unwrap();
    wait_for(&mut session, "WAKE_READY");
    while session.work_pending {
        session.poll();
    }
    let before = count.load(Ordering::Relaxed);
    thread::sleep(Duration::from_millis(50));
    assert!(!session.poll());
    assert!(!session.work_pending);
    assert_eq!(before, count.load(Ordering::Relaxed));
}

#[test]
fn experimental_terminal_history_uses_disk_and_only_decodes_visible_rows() {
    let mut session = shell(&std::env::current_dir().unwrap());
    session.resize(3, 12).unwrap();
    session.clear();
    session
        .parser
        .process("\x1b[31m界e\u{301}\x1b[0m\r\nLINE_B\r\nLINE_C\r\nLIVE_END".as_bytes());
    session.update_history();
    assert_eq!(session.history.as_ref().unwrap().lock().unwrap().len(), 1);
    session.scroll(1);
    assert!(
        session
            .screen()
            .contents()
            .starts_with("界e\u{301}\nLINE_B")
    );
    assert_eq!(
        session.screen().cell(0, 0).unwrap().fgcolor(),
        vt100::Color::Idx(1)
    );
    assert!(session.screen().hide_cursor());
    session.resize(3, 3).unwrap();
    assert_eq!(session.screen().cell(0, 2).unwrap().contents(), "e\u{301}");
    assert_eq!(session.screen().cell(1, 0).unwrap().contents(), "L");
    session.send(b"\r".to_vec()).unwrap();
    assert!(session.history_view.is_none());
    session.resize(3, 12).unwrap();
    session.clear();
    for index in 0..2000 {
        session
            .parser
            .process(format!("ROW_{index:04}\r\n").as_bytes());
    }
    session.update_history();
    assert!(session.history.as_ref().unwrap().lock().unwrap().len() > 512);
    assert_eq!(session.live_screen().scrollback(), 0);
    session.scroll(isize::MAX);
    assert!(session.screen().contents().starts_with("ROW_0000"));
    session.clear();
    assert_eq!(session.history.as_ref().unwrap().lock().unwrap().len(), 0);
    assert!(session.history_view.is_none());
}

#[test]
fn experimental_terminal_history_survives_reset_and_excludes_alternate_screen() {
    let history = Arc::new(Mutex::new(History::new().unwrap()));
    let mut parser = vt100::Parser::new(2, 8, 0);
    parser.screen_mut().set_scrollback_sink(history.clone());
    parser.process(b"abcdefghijklmnopQ");
    assert!(history.lock().unwrap().row(0).unwrap().wrapped);
    let before = history.lock().unwrap().len();
    parser.process(b"\x1b[?1049h1\r\n2\r\n3\r\n4");
    assert_eq!(history.lock().unwrap().len(), before);
    parser.process(b"\x1b[?1049l\x1bc");
    assert_eq!(history.lock().unwrap().len(), 0);
    parser.process(b"after\r\nreset\r\nend");
    assert_eq!(history.lock().unwrap().row(0).unwrap().bytes, b"after");
    parser.process(b"\x1b[3J\x1b[Hpost\r\nclear\r\nend");
    assert_eq!(history.lock().unwrap().len(), 1);
    assert!(
        history
            .lock()
            .unwrap()
            .row(0)
            .unwrap()
            .bytes
            .starts_with(b"post")
    );
}

#[test]
fn experimental_terminal_control_strings_have_fixed_storage_and_recover() {
    let mut parser = vt100::Parser::new_with_callbacks(2, 12, 0, Replies::default());
    // vte's fixed-buffer mode stores at most 1 KiB even without a terminator.
    assert!(std::mem::size_of_val(&parser) < 8192);
    parser.process(b"\x1b]777;");
    let chunk = [b'x'; 65536];
    for _ in 0..256 {
        parser.process(&chunk);
    }
    parser.process(b"\x07\x1b]777;potyi-ready\x07READY");
    assert!(parser.callbacks().ready);
    assert_eq!(parser.screen().contents(), "READY");
}

#[test]
fn experimental_terminal_large_control_counts_stop_at_the_screen_boundary() {
    for (large, bounded) in [
        (b"\x1b[65535@".as_slice(), b"\x1b[12@".as_slice()),
        (b"\x1b[65535L".as_slice(), b"\x1b[3L".as_slice()),
        (b"\x1b[65535T".as_slice(), b"\x1b[3T".as_slice()),
    ] {
        let mut actual = vt100::Parser::new(3, 12, 0);
        let mut expected = vt100::Parser::new(3, 12, 0);
        let setup = b"first\r\nsecond\r\nthird\x1b[H";
        actual.process(setup);
        expected.process(setup);
        actual.process(large);
        expected.process(bounded);
        actual.process(b"OK");
        expected.process(b"OK");
        for row in 0..3 {
            for col in 0..12 {
                assert_eq!(
                    actual.screen().cell(row, col),
                    expected.screen().cell(row, col)
                );
            }
        }
        assert_eq!(
            actual.screen().cursor_position(),
            expected.screen().cursor_position()
        );
    }
}

#[test]
fn experimental_terminal_disk_view_matches_cells_and_wraps_of_a_reference_parser() {
    let mut session = shell(&std::env::current_dir().unwrap());
    session.resize(3, 12).unwrap();
    session.clear();
    let mut reference = vt100::Parser::new(3, 12, 100);
    let output = "\x1b[31m界e\u{301}\x1b[0m\r\nabcdefghijklmnop\r\n\x1b[38;2;4;8;12;48;2;9;6;3;1;3mcoloured  \x1b[0m\r\nend\r\nmore\r\nlast";
    for byte in output.as_bytes() {
        session.parser.process(&[*byte]);
        reference.process(&[*byte]);
    }
    session.update_history();
    for offset in [0, 1, 2, 3, 4, 999] {
        session.history_offset = offset;
        session.refresh_history();
        reference.screen_mut().set_scrollback(offset);
        assert_eq!(session.screen().contents(), reference.screen().contents());
        for row in 0..3 {
            assert_eq!(
                session.screen().row_wrapped(row),
                reference.screen().row_wrapped(row)
            );
            for col in 0..12 {
                assert_eq!(
                    session.screen().cell(row, col),
                    reference.screen().cell(row, col),
                    "row {row}, col {col}, offset {offset}"
                );
            }
        }
    }
}

#[test]
fn experimental_terminal_disk_view_keeps_its_position_during_output_and_alternate_screen() {
    let mut session = shell(&std::env::current_dir().unwrap());
    session.resize(3, 12).unwrap();
    session.clear();
    session
        .parser
        .process(b"first\r\nsecond\r\nthird\r\nfourth");
    session.update_history();
    session.scroll(1);
    let before = session.screen().contents();
    session.parser.process(b"\r\nfifth\r\nsixth");
    session.update_history();
    assert_eq!(session.screen().contents(), before);
    session.parser.process(b"\x1b[?1049h\x1b[2J\x1b[Halternate");
    session.update_history();
    assert!(session.screen().alternate_screen());
    session.parser.process(b"\x1b[?1049l");
    session.update_history();
    assert_eq!(session.screen().contents(), before);
    session.parser.process(b"\x1b[3J");
    session.update_history();
    assert!(session.history_view.is_none());
    assert_eq!(
        session.screen().hide_cursor(),
        session.live_screen().hide_cursor()
    );
}

#[test]
fn experimental_terminal_line_controls_preserve_rows_outside_the_scroll_region() {
    for operation in [b'L', b'M'] {
        let mut parser = vt100::Parser::new(6, 8, 0);
        parser.process(b"ROW_0\r\nROW_1\r\nROW_2\r\nROW_3\r\nROW_4\r\nROW_5");
        let before = parser.screen().contents();
        // The cursor is outside the configured scrolling region.
        parser.process(b"\x1b[2;4r\x1b[5;1H");
        let control = format!("\x1b[65535{}", operation as char);
        parser.process(control.as_bytes());
        assert_eq!(parser.screen().contents(), before);
        // A large count inside the region cannot erase rows beyond its bottom.
        parser.process(b"\x1b[3;1H");
        parser.process(control.as_bytes());
        for row in [0, 1, 4, 5] {
            let actual = parser.screen().rows(0, 8).nth(row).unwrap();
            assert_eq!(actual, format!("ROW_{row}"));
        }
        assert!(!parser.screen().cell(2, 0).unwrap().has_contents());
        assert!(!parser.screen().cell(3, 0).unwrap().has_contents());
    }
}

fn managed_environment(bridge: &super::super::bridge::Bridge) -> Vec<(&'static str, String)> {
    let mut environment = bridge.environment().unwrap();
    let helper = std::env::var_os("POTYI_TERM_TEST_CLIENT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(format!("potyi{}", std::env::consts::EXE_SUFFIX))
        });
    assert!(
        helper.is_file(),
        "Build the terminal helper before managed tests: {}",
        helper.display()
    );
    environment[0].1 = helper.to_string_lossy().into_owned();
    environment
}

fn managed(directory: &Path) -> (Session, super::super::bridge::Bridge) {
    let bridge = super::super::bridge::Bridge::new(Arc::new(|| {})).unwrap();
    let environment = managed_environment(&bridge);
    let mut session =
        Session::open_with_environment(directory, 24, 100, Arc::new(|| {}), &environment).unwrap();
    #[cfg(unix)]
    assert!(session.start_command("echo too early", directory).is_err());
    let start = Instant::now();
    while !session.ready() {
        session.poll();
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "Managed startup failed: {:?}; {}",
            session.status,
            session.screen().contents()
        );
        thread::sleep(Duration::from_millis(5));
    }
    (session, bridge)
}
fn managed_completion(
    session: &mut Session,
    bridge: &super::super::bridge::Bridge,
) -> super::super::lifecycle::Completion {
    let start = Instant::now();
    loop {
        session.poll();
        while let Ok(request) = bridge.requests.try_recv() {
            if request.operation == "complete" {
                assert!(session.command_finished(
                    request.command_id.unwrap(),
                    request.exit_status.unwrap(),
                    request.directory
                ));
            }
        }
        if let Some(completion) = session.take_completion() {
            return completion;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "No completion: {:?}; {}",
            session.status,
            session.screen().contents()
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn managed_shared(
    directory: &Path,
    rows: u16,
    cols: u16,
) -> (Session, super::super::bridge::Bridge, SharedTranscript) {
    let bridge = super::super::bridge::Bridge::new(Arc::new(|| {})).unwrap();
    let environment = managed_environment(&bridge);
    let transcript = super::super::transcript::Transcript::shared().unwrap();
    let mut session = Session::open_with_transcript(
        directory,
        rows,
        cols,
        Arc::new(|| {}),
        &environment,
        transcript.clone(),
    )
    .unwrap();
    let start = Instant::now();
    while !session.ready() {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(5));
    }
    assert!(session.history.is_none());
    (session, bridge, transcript)
}

fn transcript_terminal_text(transcript: &SharedTranscript) -> Vec<String> {
    use super::super::transcript::RecordData;
    let mut transcript = transcript.lock().unwrap();
    let mut rows = Vec::new();
    for index in 0..transcript.len() {
        let record = transcript.read(index).unwrap();
        if matches!(record.data, RecordData::Terminal { .. }) {
            rows.push(
                super::super::output_selection::record_text(&record)
                    .unwrap()
                    .text,
            );
        }
    }
    rows
}

fn synthetic_capture(rows: u16, cols: u16) -> (vt100::Parser<Replies>, SharedTranscript) {
    let transcript = super::super::transcript::Transcript::shared().unwrap();
    let capture = Arc::new(Mutex::new(Capture {
        transcript: transcript.clone(),
        active: Some(Origin {
            relative_safe: true,
            source: 1,
            directory: std::env::temp_dir(),
        }),
        error: None,
        failed: false,
    }));
    let mut parser = vt100::Parser::new_with_callbacks(
        rows,
        cols,
        0,
        Replies {
            capture: Some(capture.clone()),
            expected_end: Some(("0123456789abcdef0123456789abcdef".into(), 1)),
            ..Replies::default()
        },
    );
    parser.screen_mut().set_scrollback_sink(capture);
    (parser, transcript)
}

#[test]
fn experimental_terminal_end_marker_recovers_fragmented_incomplete_controls_and_snapshots_once() {
    for unfinished in [b"\x1b]unfinished".as_slice(), b"\x1b[12;".as_slice()] {
        let (mut parser, transcript) = synthetic_capture(4, 30);
        parser.process(b"first\r\npartial");
        parser.process(unfinished);
        let marker = b"\x18\x1b]777;potyi-end;0123456789abcdef0123456789abcdef;1\x07";
        for byte in marker {
            parser.process(&[*byte]);
        }
        assert!(parser.callbacks().end_seen);
        assert_eq!(
            parser.callbacks().end_frame.as_ref().unwrap().contents(),
            "first\npartial"
        );
        let count = transcript.lock().unwrap().len();
        parser.process(b"trailing\r\nbytes\r\nignored\r\nspill\r\n");
        parser.process(marker);
        assert_eq!(transcript.lock().unwrap().len(), count);
        assert_eq!(
            parser.callbacks().end_frame.as_ref().unwrap().contents(),
            "first\npartial"
        );
    }
}

#[test]
fn experimental_terminal_unmatched_end_markers_and_vt_clears_preserve_shared_records() {
    let (mut parser, transcript) = synthetic_capture(3, 20);
    let epoch = transcript.lock().unwrap().epoch();
    transcript
        .lock()
        .unwrap()
        .append_header(&std::env::temp_dir(), 9, "native record")
        .unwrap();
    parser.process(b"one\r\ntwo\r\nthree\r\nfour\r\n");
    let count = transcript.lock().unwrap().len();
    assert!(count > 1);
    parser.process(b"\x1b[3J\x1bc");
    assert_eq!(transcript.lock().unwrap().len(), count);
    assert_eq!(transcript.lock().unwrap().epoch(), epoch);
    parser.process(b"\x18\x1b]777;potyi-end;foreign;1\x07");
    parser.process(b"\x18\x1b]777;potyi-end;0123456789abcdef0123456789abcdef;2\x07");
    assert!(!parser.callbacks().end_seen);
}

#[cfg(unix)]
#[test]
fn experimental_terminal_shared_transcript_waits_for_marker_and_seals_scrollout_tail_once() {
    use super::super::transcript::RecordData;
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge, transcript) = managed_shared(&directory, 4, 40);
    transcript
        .lock()
        .unwrap()
        .append_header(&directory, 0, "native before command")
        .unwrap();
    session.start_command("i=0; while [ $i -lt 50 ]; do printf 'ROW:%02d\\n' $i; i=$((i+1)); done; printf '\\n尾partial'", &directory).unwrap();
    // Authenticated IPC can arrive while all PTY bytes remain unparsed.
    let request = bridge
        .requests
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    assert_eq!(request.operation, "complete");
    assert!(session.command_finished(
        request.command_id.unwrap(),
        request.exit_status.unwrap(),
        request.directory
    ));
    assert!(session.running());
    assert!(session.take_completion().is_none());
    assert!(session.start_command("echo too early", &directory).is_err());
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    assert_eq!(session.live_tail_rows(), 0);
    let mut expected: Vec<String> = (0..50).map(|i| format!("ROW:{i:02}")).collect();
    expected.push(String::new());
    expected.push("尾partial".into());
    assert_eq!(transcript_terminal_text(&transcript), expected);
    let mut store = transcript.lock().unwrap();
    assert!(
        matches!(store.read(0).unwrap().data, RecordData::Header(ref s) if s == "native before command")
    );
    let last = store.len() - 1;
    assert!(matches!(
        store.read(last).unwrap().data,
        RecordData::Result { status: 0, .. }
    ));
}

#[cfg(unix)]
#[test]
fn experimental_terminal_shared_eof_seals_partial_primary_tail_and_releases_input() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge, transcript) = managed_shared(&directory, 4, 40);
    session
        .start_command(
            "printf 'EOF_界_partial'; while :; do sleep 30; done",
            &directory,
        )
        .unwrap();
    let start = Instant::now();
    while !session
        .plain_tail()
        .is_some_and(|(text, _)| text == "EOF_界_partial")
    {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    // An unexpected dispatcher failure must still use ordered reader EOF,
    // rather than pretending that a command completion marker was received.
    assert_eq!(
        unsafe { libc::kill(-session.shell_pid.unwrap(), libc::SIGKILL) },
        0
    );
    let start = Instant::now();
    while !session.closed || session.running() {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    assert!(session.take_completion().is_none());
    assert_eq!(transcript_terminal_text(&transcript), ["EOF_界_partial"]);
    assert!(session.start_command("echo too late", &directory).is_err());
    drop(bridge);
}

#[cfg(unix)]
#[test]
fn experimental_terminal_clear_discards_unfinished_plain_tail_without_resurrecting_old_text() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge, transcript) = managed_shared(&directory, 24, 40);
    let gate = std::env::temp_dir().join(format!("potyi-clear-tail-gate-{}", std::process::id()));
    let _ = std::fs::remove_file(&gate);
    let quoted = gate.to_string_lossy().replace('\'', "'\\''");
    session.start_command(&format!("printf PRE_CLEAR_PARTIAL; while [ ! -f '{quoted}' ]; do sleep 0.01; done; printf 'AFTER_CLEAR\\n'"), &directory).unwrap();
    let start = Instant::now();
    while !session
        .plain_tail()
        .is_some_and(|(text, _)| text == "PRE_CLEAR_PARTIAL")
    {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    assert!(session.running());
    let old_epoch = transcript.lock().unwrap().epoch();
    transcript.lock().unwrap().clear().unwrap();
    session.clear();
    assert!(session.plain_tail().is_none());
    std::fs::write(&gate, "go").unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    assert_eq!(transcript_terminal_text(&transcript), ["AFTER_CLEAR"]);
    assert_eq!(transcript.lock().unwrap().epoch(), old_epoch + 1);
    std::fs::remove_file(gate).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_selection_keeps_plain_future_tail_ids_after_newline_commits_to_disk() {
    use super::super::{
        output_selection::{Selection, TextSource},
        output_source::Source,
        transcript::Anchor,
    };
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge, transcript) = managed_shared(&directory, 24, 40);
    let gate =
        std::env::temp_dir().join(format!("potyi-selection-tail-gate-{}", std::process::id()));
    let _ = std::fs::remove_file(&gate);
    let quoted = gate.to_string_lossy().replace('\'', "'\\''");
    session.start_command(&format!("printf 'TAIL:18\\nTAIL:19'; while [ ! -f '{quoted}' ]; do sleep 0.01; done; printf '\\n'"), &directory).unwrap();
    let start = Instant::now();
    while !session
        .plain_tail()
        .is_some_and(|(text, _)| text == "TAIL:19")
    {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    let mut selection = Selection::default();
    let future;
    {
        let mut source = Source {
            store: &transcript,
            session: &session,
        };
        let bounds = source.bounds();
        future = bounds.end - 1;
        let first = Anchor {
            epoch: bounds.epoch,
            record_id: future - 1,
            utf8_byte_offset: 0,
        };
        let last = Anchor {
            epoch: bounds.epoch,
            record_id: future,
            utf8_byte_offset: "TAIL:19".len(),
        };
        selection.apply(&mut source, first, false).unwrap();
        selection.apply(&mut source, last, true).unwrap();
        assert_eq!(
            selection.copy(&mut source, false, false).unwrap(),
            "TAIL:18\nTAIL:19"
        );
    }
    assert!(transcript.lock().unwrap().read_id(future).is_err());
    std::fs::write(&gate, "go").unwrap();
    while session.running() {
        session.poll();
        while let Ok(request) = bridge.requests.try_recv() {
            if request.operation == "complete" {
                assert!(session.command_finished(
                    request.command_id.unwrap(),
                    request.exit_status.unwrap(),
                    request.directory
                ));
            }
        }
        let mut source = Source {
            store: &transcript,
            session: &session,
        };
        selection.clamp(&mut source).unwrap();
        assert_eq!(
            selection.copy(&mut source, false, false).unwrap(),
            "TAIL:18\nTAIL:19"
        );
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(session.take_completion().unwrap().status, 0);
    let record = transcript.lock().unwrap().read_id(future).unwrap();
    assert_eq!(
        super::super::output_selection::record_text(&record)
            .unwrap()
            .text,
        "TAIL:19"
    );
    std::fs::remove_file(gate).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_clear_keeps_active_command_and_captures_future_output_in_new_epoch() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge, transcript) = managed_shared(&directory, 4, 40);
    assert!(!session.supports_input());
    let gate = std::env::temp_dir().join(format!("potyi-clear-active-gate-{}", std::process::id()));
    let _ = std::fs::remove_file(&gate);
    let quoted = gate.to_string_lossy().replace('\'', "'\\''");
    session.start_command(&format!("printf 'PRE_CLEAR_READY\\n'; while [ ! -f '{quoted}' ]; do sleep 0.01; done; printf 'AFTER_CLEAR:hello:END\\n'"), &directory).unwrap();
    wait_for(&mut session, "PRE_CLEAR_READY");
    let old_epoch = transcript.lock().unwrap().epoch();
    transcript.lock().unwrap().clear().unwrap();
    session.clear();
    assert!(session.running());
    std::fs::write(&gate, "go").unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    let text = transcript_terminal_text(&transcript).join("\n");
    assert!(!text.contains("PRE_CLEAR"));
    assert!(text.contains("AFTER_CLEAR:hello:END"));
    assert_eq!(transcript.lock().unwrap().epoch(), old_epoch + 1);
    std::fs::remove_file(gate).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_prompt_keeps_stdin_owned_by_running_command() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&directory);
    session
        .start_command(
            "printf 'INPUT_%s\\n' READY; read answer; printf 'ANSWER:%s:END\\n' \"$answer\"",
            &directory,
        )
        .unwrap();
    wait_for(&mut session, "INPUT_READY");
    assert!(session.running());
    assert!(session.start_command("echo too early", &directory).is_err());
    session.send("héllo界\r".as_bytes().to_vec()).unwrap();
    let completion = managed_completion(&mut session, &bridge);
    assert_eq!(completion.status, 0);
    assert_eq!(completion.directory, directory.canonicalize().unwrap());
    assert!(!session.running());
    wait_for(&mut session, "ANSWER:héllo界:END");
    assert!(!session.screen().contents().contains("POTYI_TERM_CLIENT"));
    assert!(!session.command_finished(completion.id, 0, directory));
}
#[cfg(unix)]
#[test]
fn experimental_terminal_managed_completion_preserves_environment_and_verified_cwd() {
    let directory = std::env::temp_dir().join(format!("potyi managed {} é '$", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let original = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&original);
    let path = directory.to_string_lossy().replace('\'', "'\\''");
    session
        .start_command(
            &format!("export POTYI_MANAGED_PROBE=kept; cd '{path}'; false # trailing comment"),
            &original,
        )
        .unwrap();
    let completion = managed_completion(&mut session, &bridge);
    assert_eq!(completion.status, 1);
    assert_eq!(completion.directory, directory.canonicalize().unwrap());
    session
        .start_command(
            "printf 'VALUE:%s:END\\n' \"$POTYI_MANAGED_PROBE\"\nprintf 'MULTI:%s:END\\n' line",
            &completion.directory,
        )
        .unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    wait_for(&mut session, "VALUE:kept:END");
    wait_for(&mut session, "MULTI:line:END");
    drop(session);
    drop(bridge);
    std::fs::remove_dir_all(directory).unwrap();
}
#[cfg(unix)]
#[test]
fn experimental_terminal_managed_interrupt_has_an_explicit_completion_and_remains_usable() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&directory);
    session
        .start_command("printf 'RUN_%s\\n' START; sleep 30", &directory)
        .unwrap();
    wait_for(&mut session, "RUN_START");
    thread::sleep(Duration::from_millis(100));
    session.send(vec![3]).unwrap();
    let completion = managed_completion(&mut session, &bridge);
    assert_ne!(completion.status, 0);
    assert!(!session.running());
    session
        .start_command("printf 'AFTER_%s\\n' INTERRUPT", &completion.directory)
        .unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    wait_for(&mut session, "AFTER_INTERRUPT");
}

#[cfg(unix)]
fn stopped_session_accepts_helpers_and_next_command(
    session: &mut Session,
    bridge: &super::super::bridge::Bridge,
    directory: &Path,
    stopped_id: u64,
) {
    assert!(session.ready());
    assert!(!session.running());
    session
        .start_command(
            "edit 'after stop.rs'; view 'after stop.rs'; printf 'AFTER_%s\\n' STOP",
            directory,
        )
        .unwrap();
    assert!(!session.command_finished(stopped_id, 0, directory.to_path_buf()));
    let start = Instant::now();
    let mut operations = Vec::new();
    let completion = loop {
        session.poll();
        while let Ok(request) = bridge.requests.try_recv() {
            if request.operation == "complete" {
                // A late completion from the killed shell cannot finish the
                // fresh command: its command ID is never reused after restart.
                if request.command_id.unwrap() <= stopped_id {
                    continue;
                }
                assert!(session.command_finished(
                    request.command_id.unwrap(),
                    request.exit_status.unwrap(),
                    request.directory
                ));
            } else if matches!(request.operation.as_str(), "edit" | "view") {
                assert_eq!(request.path, "after stop.rs");
                operations.push(request.operation);
            }
        }
        if let Some(completion) = session.take_completion() {
            break completion;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Replacement shell did not accept helpers"
        );
        thread::sleep(Duration::from_millis(5));
    };
    assert!(completion.id > stopped_id);
    assert_eq!(completion.status, 0);
    assert_eq!(operations, ["edit", "view"]);
    wait_for(session, "AFTER_STOP");
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_stop_before_output_cannot_be_lost_in_dispatch() {
    let directory = std::env::current_dir().unwrap();
    for delay in [Duration::ZERO, Duration::from_millis(40)]
        .into_iter()
        .cycle()
        .take(8)
    {
        let (mut session, bridge) = managed(&directory);
        session
            .start_command("sleep 30; printf 'SHOULD_NOT_%s\\n' FINISH", &directory)
            .unwrap();
        if !delay.is_zero() {
            thread::sleep(delay);
        }
        let start = Instant::now();
        session.send(vec![3]).unwrap();
        session.send(vec![3]).unwrap();
        let completion = managed_completion(&mut session, &bridge);
        assert_eq!(completion.status, 130);
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "Stop waited for the command to exit normally"
        );
        assert!(
            !session.screen().contents().contains("SHOULD_NOT_FINISH"),
            "{}",
            session.screen().contents()
        );
        stopped_session_accepts_helpers_and_next_command(
            &mut session,
            &bridge,
            &completion.directory,
            completion.id,
        );
    }
}

#[cfg(unix)]
#[test]
fn experimental_terminal_production_stop_before_output_never_runs_a_following_command() {
    let directory = std::env::current_dir().unwrap();
    for delay in [Duration::ZERO, Duration::from_millis(40)]
        .into_iter()
        .cycle()
        .take(8)
    {
        let (mut session, bridge, transcript) = managed_shared(&directory, 24, 100);
        session
            .start_command(
                "trap '' HUP INT; sleep 30; printf 'SHOULD_NOT_%s\\n' FINISH",
                &directory,
            )
            .unwrap();
        if !delay.is_zero() {
            thread::sleep(delay);
        }
        let start = Instant::now();
        session.send(vec![3]).unwrap();
        session.send(vec![3]).unwrap();
        let completion = managed_completion(&mut session, &bridge);
        assert_eq!(completion.status, 130);
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "Stop waited for normal exit"
        );
        let output = transcript_terminal_text(&transcript).join("\n");
        assert!(!output.contains("SHOULD_NOT_FINISH"), "{output}");
        stopped_session_accepts_helpers_and_next_command(
            &mut session,
            &bridge,
            &completion.directory,
            completion.id,
        );
    }
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_stop_kills_a_command_that_ignores_interrupt() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge, transcript) = managed_shared(&directory, 3, 32);
    session
        .start_command(
            "trap '' INT; printf 'IGNORE_%s\\n' READY; while :; do sleep 30; done",
            &directory,
        )
        .unwrap();
    wait_for(&mut session, "IGNORE_READY");
    let start = Instant::now();
    session.send(vec![3]).unwrap();
    let completion = managed_completion(&mut session, &bridge);
    assert_eq!(completion.status, 130);
    assert!(start.elapsed() < Duration::from_secs(3));
    let mut transcript = transcript.lock().unwrap();
    let mut results = 0;
    let mut output = String::new();
    for index in 0..transcript.len() {
        let record = transcript.read(index).unwrap();
        match record.data {
            super::super::transcript::RecordData::Terminal {
                cols, formatted, ..
            } => {
                if cols == 0 {
                    output.push_str(super::super::output_colors::plain_text(&formatted).unwrap());
                } else {
                    let mut parser = vt100::Parser::new(1, cols, 0);
                    parser.process(&formatted);
                    output.push_str(&parser.screen().contents());
                }
            }
            super::super::transcript::RecordData::Result {
                status,
                relative_safe,
                ..
            } => {
                assert_eq!(status, 130);
                assert!(!relative_safe);
                results += 1;
            }
            _ => {}
        }
    }
    assert_eq!(output.matches("IGNORE_READY").count(), 1);
    assert_eq!(results, 1);
    drop(transcript);
    // The old shell's ignored INT trap must not leak into the replacement.
    stopped_session_accepts_helpers_and_next_command(
        &mut session,
        &bridge,
        &completion.directory,
        completion.id,
    );
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_stop_during_syntax_check_never_runs_the_body() {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::env::temp_dir().join(format!("potyi-stop-preflight-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let wrapper = directory.join("sh");
    let gate = directory.join("preflight-ready");
    let gate_literal = gate.to_string_lossy().replace('\'', "'\\''");
    std::fs::write(&wrapper, format!("#!/bin/sh\nif [ \"$1\" = -n ]; then : > '{gate_literal}'; sleep 30; fi\nexec /bin/sh \"$@\"\n")).unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let bridge = super::super::bridge::Bridge::new(Arc::new(|| {})).unwrap();
    let environment = managed_environment(&bridge);
    let mut session = Session::open_managed_shell(
        wrapper.clone().into_os_string(),
        &directory,
        24,
        100,
        Arc::new(|| {}),
        &environment,
        None,
    )
    .unwrap();
    let start = Instant::now();
    while !session.ready() {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    session
        .start_command("printf 'BODY_MUST_NOT_RUN\\n'", &directory)
        .unwrap();
    let start = Instant::now();
    while !gate.is_file() {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    let start = Instant::now();
    session.send(vec![3]).unwrap();
    let completion = managed_completion(&mut session, &bridge);
    assert_eq!(completion.status, 130);
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(!session.screen().contents().contains("BODY_MUST_NOT_RUN"));
    // Remove only this test wrapper's deliberate preflight block.
    std::fs::write(&wrapper, "#!/bin/sh\nexec /bin/sh \"$@\"\n").unwrap();
    stopped_session_accepts_helpers_and_next_command(
        &mut session,
        &bridge,
        &completion.directory,
        completion.id,
    );
    drop(session);
    drop(bridge);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_nonlogin_startup_keeps_user_environment_and_path() {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::env::temp_dir().join(format!("potyi-startup-parity-{}", std::process::id()));
    let bin = directory.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let tool = bin.join("potyi-startup-tool");
    std::fs::write(&tool, "#!/bin/sh\nprintf 'STARTUP_TOOL_READY\\n'\n").unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    let bin_literal = bin.to_string_lossy().replace('\'', "'\\''");
    let startup =
        format!("export POTYI_STARTUP_PARITY=kept\nexport PATH='{bin_literal}':\"$PATH\"\n");
    std::fs::write(directory.join(".zshenv"), &startup).unwrap();
    std::fs::write(directory.join("bash-env"), &startup).unwrap();
    for file in [".zprofile", ".zshrc", ".bash_profile", ".bashrc"] {
        std::fs::write(
            directory.join(file),
            "export POTYI_UNEXPECTED_PROFILE=loaded\n",
        )
        .unwrap();
    }
    let mut checked = 0;
    for (shell, variable, value) in [
        ("/bin/zsh", "ZDOTDIR", directory.clone()),
        ("/bin/bash", "BASH_ENV", directory.join("bash-env")),
    ] {
        if !Path::new(shell).is_file() {
            continue;
        }
        let bridge = super::super::bridge::Bridge::new(Arc::new(|| {})).unwrap();
        let mut environment = managed_environment(&bridge);
        environment.push((variable, value.to_string_lossy().into_owned()));
        environment.push(("HOME", directory.to_string_lossy().into_owned()));
        let mut session = Session::open_managed_shell(
            shell.into(),
            &directory,
            24,
            100,
            Arc::new(|| {}),
            &environment,
            None,
        )
        .unwrap();
        let start = Instant::now();
        while !session.ready() {
            session.poll();
            assert!(start.elapsed() < Duration::from_secs(5));
            thread::sleep(Duration::from_millis(5));
        }
        session.start_command("printf 'STARTUP:%s:PROFILE:%s\\n' \"$POTYI_STARTUP_PARITY\" \"${POTYI_UNEXPECTED_PROFILE-unset}\"; potyi-startup-tool", &directory).unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        wait_for(&mut session, "STARTUP:kept:PROFILE:unset");
        wait_for(&mut session, "STARTUP_TOOL_READY");
        session
            .start_command(
                "printf 'SECOND:%s\\n' \"$POTYI_STARTUP_PARITY\"",
                &directory,
            )
            .unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        wait_for(&mut session, "SECOND:kept");
        checked += 1;
    }
    assert!(
        checked > 0,
        "No supported shell available for startup parity"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_production_stdin_is_eof_for_read_and_grep() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge, transcript) = managed_shared(&directory, 24, 100);
    let start = Instant::now();
    session
        .start_command(
            "read answer; printf 'READ_STATUS:%s:END\\n' \"$?\"; grep potyi-no-input-pattern",
            &directory,
        )
        .unwrap();
    let completion = managed_completion(&mut session, &bridge);
    assert_eq!(completion.status, 1);
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "Noninteractive stdin waited for user input"
    );
    let output = transcript_terminal_text(&transcript).join("\n");
    assert!(output.contains("READ_STATUS:1:END"));
    assert!(!output.contains("potyi-no-input-pattern"));
}

#[cfg(unix)]
#[test]
fn experimental_terminal_production_exit_exec_environment_trap_and_cd_are_isolated() {
    let directory =
        std::env::temp_dir().join(format!("potyi-child-isolation-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("subdirectory")).unwrap();
    let directory = directory.canonicalize().unwrap();
    let bridge = super::super::bridge::Bridge::new(Arc::new(|| {})).unwrap();
    let environment = managed_environment(&bridge);
    let transcript = super::super::transcript::Transcript::shared().unwrap();
    let mut session = Session::open_managed_shell(
        "/bin/sh".into(),
        &directory,
        24,
        120,
        Arc::new(|| {}),
        &environment,
        Some(transcript.clone()),
    )
    .unwrap();
    let start = Instant::now();
    while !session.ready() {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    for (command, expected) in [
        ("exit 7", 7),
        ("exec printf 'EXEC_READY\\n'", 0),
        (
            "export POTYI_CHILD_ISOLATED=changed; trap '' INT; cd subdirectory; printf 'CHILD_DIRECTORY:'; pwd",
            0,
        ),
        (
            "printf 'NEXT_ENV:%s:CWD:' \"${POTYI_CHILD_ISOLATED-unset}\"; pwd; printf 'TRAPS_BEGIN\\n'; trap; printf 'TRAPS_END\\n'",
            0,
        ),
    ] {
        session.start_command(command, &directory).unwrap();
        let completion = managed_completion(&mut session, &bridge);
        assert_eq!(completion.status, expected, "{command}");
        assert_eq!(completion.directory, directory);
        assert!(!session.closed);
        assert!(session.ready());
    }
    let output = transcript_terminal_text(&transcript).join("\n");
    assert!(output.contains("EXEC_READY"));
    assert!(output.contains(&format!(
        "CHILD_DIRECTORY:{}",
        directory.join("subdirectory").display()
    )));
    assert!(output.contains(&format!("NEXT_ENV:unset:CWD:{}", directory.display())));
    assert!(
        output.contains("TRAPS_BEGIN\nTRAPS_END"),
        "Child INT trap leaked to the next command: {output}"
    );
    drop(session);
    drop(bridge);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_production_startup_runs_once_per_command_and_captures_its_output() {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::env::temp_dir().join(format!("potyi-child-startup-{}", std::process::id()));
    let bin = directory.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let tool = bin.join("potyi-startup-child-tool");
    std::fs::write(&tool, "#!/bin/sh\nprintf 'STARTUP_CHILD_TOOL_READY\\n'\n").unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    let bin_literal = bin.to_string_lossy().replace('\'', "'\\''");
    for profile in [".zprofile", ".zshrc", ".bash_profile", ".bashrc"] {
        std::fs::write(
            directory.join(profile),
            "export POTYI_UNEXPECTED_PROFILE=loaded\n",
        )
        .unwrap();
    }
    let mut checked = 0;
    for (shell, variable, startup_name) in [
        ("/bin/zsh", "ZDOTDIR", ".zshenv"),
        ("/bin/bash", "BASH_ENV", "bash-env"),
    ] {
        if !Path::new(shell).is_file() {
            continue;
        }
        let counter = directory.join(format!("count-{}", variable));
        let _ = std::fs::remove_file(&counter);
        let count_literal = counter.to_string_lossy().replace('\'', "'\\''");
        let startup = format!(
            "if [ -f '{count_literal}' ]; then n=$(cat '{count_literal}'); else n=0; fi\nn=$((n+1))\nprintf '%s' \"$n\" > '{count_literal}'\nprintf 'STARTUP_SIDE_EFFECT:%s\\n' \"$n\"\nexport POTYI_STARTUP_PARITY=kept\nexport PATH='{bin_literal}':\"$PATH\"\n"
        );
        std::fs::write(directory.join(startup_name), startup).unwrap();
        let bridge = super::super::bridge::Bridge::new(Arc::new(|| {})).unwrap();
        let mut environment = managed_environment(&bridge);
        let value = if variable == "ZDOTDIR" {
            directory.clone()
        } else {
            directory.join(startup_name)
        };
        environment.push((variable, value.to_string_lossy().into_owned()));
        environment.push(("HOME", directory.to_string_lossy().into_owned()));
        let transcript = super::super::transcript::Transcript::shared().unwrap();
        let mut session = Session::open_managed_shell(
            shell.into(),
            &directory,
            24,
            120,
            Arc::new(|| {}),
            &environment,
            Some(transcript.clone()),
        )
        .unwrap();
        let start = Instant::now();
        while !session.ready() {
            session.poll();
            assert!(start.elapsed() < Duration::from_secs(5));
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            !counter.exists(),
            "Internal driver ran user startup for {shell}"
        );
        for expected in [1, 2] {
            session.start_command("printf 'STARTUP_VAR:%s:PROFILE:%s\\n' \"$POTYI_STARTUP_PARITY\" \"${POTYI_UNEXPECTED_PROFILE-unset}\"; potyi-startup-child-tool", &directory).unwrap();
            assert_eq!(managed_completion(&mut session, &bridge).status, 0);
            assert_eq!(
                std::fs::read_to_string(&counter).unwrap(),
                expected.to_string(),
                "Startup reran during syntax checking for {shell}"
            );
        }
        let output = transcript_terminal_text(&transcript).join("\n");
        assert_eq!(output.matches("STARTUP_SIDE_EFFECT:1").count(), 1);
        assert_eq!(output.matches("STARTUP_SIDE_EFFECT:2").count(), 1);
        assert_eq!(output.matches("STARTUP_VAR:kept:PROFILE:unset").count(), 2);
        assert_eq!(output.matches("STARTUP_CHILD_TOOL_READY").count(), 2);
        checked += 1;
    }
    assert!(
        checked > 0,
        "No supported shell available for startup parity"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_production_accepts_an_unknown_shell_name_with_original_c_contract() {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::env::temp_dir().join(format!("potyi-unknown-shell-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let wrapper = directory.join("custom-command-shell");
    // Reject syntax-preflight options and any injected helper/source syntax.
    // This proves the fallback respects an arbitrary shell's -c contract.
    std::fs::write(&wrapper, "#!/bin/sh\n[ \"$1\" = -c ] || exit 91\ncase \"$2\" in *POTYI_DRIVER_DIRECTORY*|*running-command*|*helpers*) exit 92 ;; esac\nprintf 'CUSTOM_STARTUP_READY\\n'\nexec /bin/sh \"$@\"\n").unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let bridge = super::super::bridge::Bridge::new(Arc::new(|| {})).unwrap();
    let environment = managed_environment(&bridge);
    let transcript = super::super::transcript::Transcript::shared().unwrap();
    let mut session = Session::open_managed_shell(
        wrapper.into_os_string(),
        &directory,
        24,
        120,
        Arc::new(|| {}),
        &environment,
        Some(transcript.clone()),
    )
    .unwrap();
    let start = Instant::now();
    while !session.ready() {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    for (command, status) in [
        ("printf 'CUSTOM:%s\\n' 'quoted space'; exit 7", 7),
        ("printf 'unterminated", 2),
        ("printf 'CUSTOM_NEXT_READY\\n'", 0),
    ] {
        session.start_command(command, &directory).unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, status);
        assert!(!session.closed);
    }
    let output = transcript_terminal_text(&transcript).join("\n");
    assert!(output.contains("CUSTOM:quoted space"));
    assert!(output.contains("CUSTOM_NEXT_READY"));
    assert_eq!(output.matches("CUSTOM_STARTUP_READY").count(), 3);
    drop(session);
    drop(bridge);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn experimental_terminal_clear_midway_through_internal_marker_keeps_completion_and_hides_nonce() {
    let directory = std::env::current_dir().unwrap();
    for split in [1, 2, 3, 24, 50] {
        let (mut session, _bridge, transcript) = managed_shared(&directory, 4, 40);
        session.start_command("sleep 30", &directory).unwrap();
        let id = session.lifecycle.as_ref().unwrap().current_id().unwrap();
        let nonce = session.lifecycle.as_ref().unwrap().nonce().to_owned();
        let marker = format!("\x18\x1b]777;potyi-end;{nonce};{id}\x07");
        let split = split.min(marker.len() - 1);
        session.process_output(b"pre-clear partial");
        session.process_output(&marker.as_bytes()[..split]);
        assert!(!session.parser.callbacks().end_seen);
        let old_epoch = transcript.lock().unwrap().epoch();
        transcript.lock().unwrap().clear().unwrap();
        session.clear();
        session.process_output(&marker.as_bytes()[split..]);
        assert!(
            session.parser.callbacks().end_seen,
            "Boundary lost at split {split}"
        );
        assert!(session.command_finished(id, 0, directory.canonicalize().unwrap()));
        session.poll();
        assert_eq!(session.take_completion().unwrap().status, 0);
        let output = transcript_terminal_text(&transcript).join("\n");
        assert!(
            output.is_empty(),
            "Marker fragments leaked after Clear at {split}: {output:?}"
        );
        assert!(!output.contains(&nonce));
        assert_eq!(transcript.lock().unwrap().epoch(), old_epoch + 1);
    }
}
#[cfg(unix)]
#[test]
fn experimental_terminal_managed_exit_never_claims_idle_before_session_closes() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&directory);
    session.start_command("exit", &directory).unwrap();
    let start = Instant::now();
    while !session.closed {
        session.poll();
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    assert!(session.take_completion().is_none());
    assert!(session.start_command("echo too late", &directory).is_err());
    drop(bridge);
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_syntax_errors_complete_without_losing_the_shell() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&directory);
    session
        .start_command("printf 'unterminated", &directory)
        .unwrap();
    assert_ne!(managed_completion(&mut session, &bridge).status, 0);
    assert!(!session.closed);
    session
        .start_command("printf 'SYNTAX_%s\\n' RECOVERED", &directory)
        .unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    wait_for(&mut session, "SYNTAX_RECOVERED");
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_helper_failure_closes_instead_of_guessing_idle() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&directory);
    #[cfg(unix)]
    let command = "export POTYI_TERM_CLIENT=/potyi-no-such-helper";
    #[cfg(windows)]
    let command = "set POTYI_TERM_CLIENT=Z:\\potyi-no-such-helper.exe";
    session.start_command(command, &directory).unwrap();
    let start = Instant::now();
    while !session.closed {
        session.poll();
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Helper failure left session waiting: {}",
            session.screen().contents()
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert!(session.take_completion().is_none());
    assert!(session.start_command("echo too late", &directory).is_err());
    drop(bridge);
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_leftover_stdin_cannot_replay_or_poison_dispatch() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&directory);
    session.start_command("printf 'BODY_ENTER_%s\\n' READY; read answer; printf 'BODY_EXIT:%s:END\\n' \"$answer\"",&directory).unwrap();
    wait_for(&mut session, "BODY_ENTER_READY");
    session.send(b"hello\r1\rpartial-tail".to_vec()).unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(150) {
        session.poll();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        session
            .screen()
            .contents()
            .matches("BODY_ENTER_READY")
            .count(),
        1
    );
    session
        .start_command("printf 'AFTER_RESIDUE_%s\\n' READY", &directory)
        .unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    wait_for(&mut session, "AFTER_RESIDUE_READY");
    assert_eq!(
        session
            .screen()
            .contents()
            .matches("AFTER_RESIDUE_READY")
            .count(),
        1
    );
    // Even a previously valid packet can only consume its numbered file once.
    let nonce = session.lifecycle.as_ref().unwrap().nonce();
    session
        .send(format!("\\r{nonce}:1\\r").replace("\\r", "\r").into_bytes())
        .unwrap();
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(100) {
        session.poll();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        session
            .screen()
            .contents()
            .matches("BODY_ENTER_READY")
            .count(),
        1
    );
    session
        .start_command("printf 'AFTER_REPLAY_%s\\n' READY", &directory)
        .unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    wait_for(&mut session, "AFTER_REPLAY_READY");
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_raw_tty_is_restored_before_next_dispatch() {
    let directory = std::env::current_dir().unwrap();
    let (mut session, bridge) = managed(&directory);
    session
        .start_command("stty raw -echo; printf 'RAW_%s\\n' DONE", &directory)
        .unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    session
        .start_command("printf 'AFTER_RAW_%s\\n' READY", &directory)
        .unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    wait_for(&mut session, "AFTER_RAW_READY");
}

#[cfg(unix)]
#[test]
fn experimental_terminal_managed_dispatch_survives_changed_path_and_cd_function() {
    let original = std::env::current_dir().unwrap();
    let target = std::env::temp_dir().join(format!("potyi dispatch cwd {}", std::process::id()));
    std::fs::create_dir_all(&target).unwrap();
    let (mut session, bridge) = managed(&original);
    session.start_command("cd(){ return 0; }; export PATH=/potyi-no-such-command-path; printf 'ENV_%s\\n' CHANGED",&original).unwrap();
    assert_eq!(managed_completion(&mut session, &bridge).status, 0);
    session
        .start_command(
            "printf 'PATH:%s:END\\n' \"$PATH\"; printf 'AFTER_PATH_%s\\n' READY",
            &target,
        )
        .unwrap();
    let completion = managed_completion(&mut session, &bridge);
    assert_eq!(completion.status, 0);
    assert_eq!(completion.directory, target.canonicalize().unwrap());
    wait_for(&mut session, "PATH:/potyi-no-such-command-path:END");
    wait_for(&mut session, "AFTER_PATH_READY");
    drop(session);
    drop(bridge);
    std::fs::remove_dir_all(target).unwrap();
}
#[test]
fn experimental_terminal_pipe_text_keeps_each_stream_utf8_complete_and_normalizes_lf() {
    let mut stdout = PipeText::default();
    let mut stderr = PipeText::default();
    assert_eq!(stdout.feed(b"OUT:\xe7\x95", false), b"OUT:");
    assert_eq!(
        stderr.feed("ERR:é\r".as_bytes(), false),
        "ERR:é\r".as_bytes()
    );
    assert_eq!(stdout.feed(b"\x8c\n", false), "界\r\n".as_bytes());
    assert_eq!(stderr.feed(b"\nEND\n", false), b"\nEND\r\n");
    assert_eq!(stdout.feed(b"\xf0\x9f", false), b"");
    assert_eq!(stdout.feed(&[], true), "�".as_bytes());
}

#[cfg(windows)]
mod windows_managed {
    use super::*;

    #[test]
    fn experimental_terminal_canonical_windows_cwd_runs_git_in_the_requested_unicode_directory() {
        struct Directory(PathBuf);
        impl Drop for Directory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let mut random = [0; 8];
        getrandom::getrandom(&mut random).unwrap();
        let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let fixture = Directory(
            std::env::temp_dir().join(format!("potyi cwd {suffix} é 界 with spaces % & ^")),
        );
        std::fs::create_dir(&fixture.0).unwrap();
        let canonical = fixture.0.canonicalize().unwrap();
        assert!(
            matches!(canonical.components().next(), Some(std::path::Component::Prefix(prefix))
            if matches!(prefix.kind(), std::path::Prefix::VerbatimDisk(_)))
        );
        let ordinary = windows_command::cmd_directory(&canonical).unwrap();
        let (mut session, bridge, transcript) = managed_shared(&canonical, 24, 128);
        session.windows.as_mut().unwrap().environment.push((
            "POTYI_CMD_CWD_REGRESSION_EXPECTED".into(),
            ordinary.to_str().unwrap().to_owned(),
        ));
        session
            .windows
            .as_mut()
            .unwrap()
            .environment
            .push(("CD".into(), r"Z:\deliberately-shadowed-cwd".into()));
        // Guard Git inside CMD itself, so a cwd regression cannot initialize
        // the Windows directory or any other unintended directory.
        // Clear inherited Git overrides in this fresh CMD child only. The
        // test must not use an external object store, template or config file.
        // An inherited CD variable can shadow CMD's dynamic %CD%; clear it
        // on an earlier line so the guard is expanded after the deletion.
        let mut command = String::from("set \"CD=\"\n");
        for variable in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_COMMON_DIR",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_TEMPLATE_DIR",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
            "GIT_CONFIG_SYSTEM",
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_NOSYSTEM",
            "GIT_NAMESPACE",
            "GIT_INDEX_FILE",
            "GIT_CEILING_DIRECTORIES",
        ] {
            command.push_str(&format!("set \"{variable}=\"\n"));
        }
        command.push_str(concat!(
            "set \"GIT_CONFIG_NOSYSTEM=1\"\n",
            "set \"GIT_CONFIG_SYSTEM=NUL\"\nset \"GIT_CONFIG_GLOBAL=NUL\"\n",
            "if /I not \"%CD%\"==\"%POTYI_CMD_CWD_REGRESSION_EXPECTED%\" (echo POTYI_CWD_MISMATCH & exit /b 91)",
            " & echo POTYI_CWD:\"%CD%\"",
            " & git --git-dir=\"%POTYI_CMD_CWD_REGRESSION_EXPECTED%\\.git\"",
            " --work-tree=\"%POTYI_CMD_CWD_REGRESSION_EXPECTED%\"",
            " init --quiet --template= --initial-branch=feature/terminal-cwd",
        ));
        session.start_command(&command, &canonical).unwrap();
        let completion = managed_completion(&mut session, &bridge);
        let text = transcript_terminal_text(&transcript).join("\n");
        assert_eq!(completion.status, 0, "{text}");
        assert_eq!(completion.directory, canonical);
        assert!(canonical.join(".git").is_dir(), "{text}");
        assert!(
            text.contains(&format!("POTYI_CWD:\"{}\"", ordinary.display())),
            "{text}"
        );
        assert!(!text.contains("POTYI_CWD_MISMATCH\n"), "{text}");
        assert!(!text.contains("UNC paths are not supported"), "{text}");
        assert!(!text.contains("Defaulting to Windows directory"), "{text}");
        drop(session);
        drop(bridge);
    }

    #[test]
    fn experimental_terminal_unsupported_windows_cwd_fails_before_command_dispatch() {
        struct Directory(PathBuf);
        impl Drop for Directory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let mut random = [0; 8];
        getrandom::getrandom(&mut random).unwrap();
        let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let fixture = Directory(std::env::temp_dir().join(format!("potyi cwd reject {suffix}")));
        std::fs::create_dir(&fixture.0).unwrap();
        let canonical = fixture.0.canonicalize().unwrap();
        let (mut session, bridge, transcript) = managed_shared(&canonical, 24, 128);
        let records_before = transcript.lock().unwrap().len();
        for target in [
            canonical.join("tail."),
            canonical.join("tail "),
            canonical.join("x".repeat(150)).join("y".repeat(110)),
        ] {
            std::fs::create_dir_all(&target).unwrap();
            let error = session
                .start_command("echo MUST_NOT_EXECUTE", &target)
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::Unsupported);
            assert!(!session.running());
            assert!(session.take_completion().is_none());
            assert_eq!(transcript.lock().unwrap().len(), records_before);
        }
        session
            .start_command("echo AFTER_CWD_REJECTION", &canonical)
            .unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        let text = transcript_terminal_text(&transcript).join("\n");
        assert!(text.contains("AFTER_CWD_REJECTION"), "{text}");
        assert!(!text.contains("MUST_NOT_EXECUTE"), "{text}");
        drop(session);
        drop(bridge);
    }

    fn visible_console_windows() -> std::collections::HashSet<usize> {
        use winapi::um::winuser::{EnumWindows, GetClassNameW, IsWindowVisible};
        unsafe extern "system" fn collect(
            window: winapi::shared::windef::HWND,
            data: isize,
        ) -> i32 {
            if unsafe { IsWindowVisible(window) } != 0 {
                let mut class = [0u16; 128];
                let count =
                    unsafe { GetClassNameW(window, class.as_mut_ptr(), class.len() as i32) };
                let class = String::from_utf16_lossy(&class[..count.max(0) as usize]);
                if matches!(
                    class.as_str(),
                    "ConsoleWindowClass" | "CASCADIA_HOSTING_WINDOW_CLASS"
                ) {
                    unsafe {
                        (&mut *(data as *mut std::collections::HashSet<usize>))
                            .insert(window as usize);
                    }
                }
            }
            1
        }
        let mut result = std::collections::HashSet::new();
        assert_ne!(
            unsafe { EnumWindows(Some(collect), &mut result as *mut _ as isize) },
            0
        );
        result
    }

    #[test]
    fn pipe_commands_keep_direct_cmd_syntax_unicode_and_launch_cwd_without_persistent_env() {
        let directory =
            std::env::temp_dir().join(format!("potyi cmd {} é % & ^", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let (mut session, bridge) = managed(&directory);
        assert!(session.ready());
        session.start_command("for /L %i in (1,1,3) do @echo LOOP:%i:界é & echo \"quoted ^& text\" & echo ERR:界é 1>&2 & set POTYI_MANAGED_WINDOWS_PROBE=kept & exit /b 17", &directory).unwrap();
        let completion = managed_completion(&mut session, &bridge);
        assert_eq!(completion.status, 17);
        assert_eq!(completion.directory, directory.canonicalize().unwrap());
        assert!(!session.closed);
        let text = session.screen().contents();
        for expected in ["LOOP:1:界é", "LOOP:3:界é", "ERR:界é", "quoted"] {
            assert!(text.contains(expected), "{text}");
        }
        session.start_command("if defined POTYI_MANAGED_WINDOWS_PROBE (exit /b 9) else (echo FRESH_ENV & exit /b 0)", &directory).unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        assert!(session.screen().contents().contains("FRESH_ENV"));
        session
            .start_command("echo FIRST_LINE\necho SECOND_LINE", &directory)
            .unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        assert!(session.screen().contents().contains("FIRST_LINE"));
        assert!(session.screen().contents().contains("SECOND_LINE"));
        session
            .start_command(
                "echo PERCENT:100% & echo \"AMP:a&b\" & exit /b 23",
                &directory,
            )
            .unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 23);
        assert!(session.screen().contents().contains("PERCENT:100%"));
        assert!(session.screen().contents().contains("AMP:a&b"));
        drop(session);
        drop(bridge);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn pipe_completion_requires_parsing_both_eofs_and_all_backlogged_rows() {
        use super::super::super::transcript::RecordData;
        let directory = std::env::current_dir().unwrap();
        let (mut session, bridge, transcript) = managed_shared(&directory, 24, 128);
        let padding = "界é".repeat(20);
        session.start_command(&format!("for /L %i in (1,1,6000) do @echo OUT:%i:{padding} & for /L %j in (1,1,6000) do @echo ERR:%j:{padding} 1>&2"), &directory).unwrap();
        assert!(session.start_command("echo OVERLAP", &directory).is_err());
        assert!(session.send(b"not interactive\r".to_vec()).is_err());
        // The producer fills the bounded queue, but completion must stay
        // unavailable until the UI has parsed every queued byte and both EOFs.
        thread::sleep(Duration::from_millis(100));
        assert!(session.running());
        assert!(session.take_completion().is_none());
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        let text = transcript_terminal_text(&transcript).join("\n");
        assert!(text.contains("OUT:1:"));
        assert!(text.contains("OUT:6000:"));
        assert!(text.contains("ERR:1:"));
        assert!(text.contains("ERR:6000:"));
        assert!(
            !text.contains('�'),
            "split UTF-8 was interleaved across pipes"
        );
        let mut store = transcript.lock().unwrap();
        let results = (0..store.len())
            .filter(|index| matches!(store.read(*index).unwrap().data, RecordData::Result { .. }))
            .count();
        assert_eq!(results, 1);
    }

    #[test]
    fn pipe_stop_reaps_grandchild_and_next_command_can_start() {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use winapi::um::{
            processthreadsapi::OpenProcess, synchapi::WaitForSingleObject, winnt::SYNCHRONIZE,
        };
        let directory =
            std::env::temp_dir().join(format!("potyi-windows-stop-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let pid_file = directory.join("child.pid");
        let (mut session, bridge) = managed(&directory);
        let visible_before = visible_console_windows();
        let pid_path =
            super::super::super::lifecycle::windows_directory(pid_file.to_str().unwrap())
                .replace('\'', "''");
        session.start_command(&format!("powershell -NoProfile -NonInteractive -Command \"$p=Start-Process powershell -ArgumentList '-NoProfile','-NonInteractive','-Command','Start-Sleep 60' -NoNewWindow -PassThru; [IO.File]::WriteAllText('{pid_path}',[string]$p.Id); Write-Output 'STOP_READY'; Start-Sleep 60\""), &directory).unwrap();
        wait_for(&mut session, "STOP_READY");
        assert!(
            visible_console_windows().is_subset(&visible_before),
            "command opened a visible console window"
        );
        let pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let child = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
        assert!(!child.is_null(), "grandchild was not created");
        let child = unsafe { OwnedHandle::from_raw_handle(child as _) };
        session.send(vec![3]).unwrap();
        assert_ne!(managed_completion(&mut session, &bridge).status, 0);
        assert_eq!(
            unsafe { WaitForSingleObject(child.as_raw_handle() as _, 5000) },
            0,
            "grandchild was not reaped"
        );
        session
            .start_command("echo AFTER_STOP", &directory)
            .unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        assert!(session.screen().contents().contains("AFTER_STOP"));
        drop(child);
        std::fs::remove_file(&pid_file).unwrap();
        session.start_command(&format!("powershell -NoProfile -NonInteractive -Command \"$p=Start-Process powershell -ArgumentList '-NoProfile','-NonInteractive','-Command','Start-Sleep 60' -NoNewWindow -PassThru; [IO.File]::WriteAllText('{pid_path}',[string]$p.Id); Write-Output 'DROP_READY'; while ($true) {{ Write-Output 'FLOOD_LINE_界é' }}\""), &directory).unwrap();
        wait_for(&mut session, "DROP_READY");
        let pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let child = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
        assert!(!child.is_null());
        let child = unsafe { OwnedHandle::from_raw_handle(child as _) };
        thread::sleep(Duration::from_millis(100));
        drop(session);
        assert_eq!(
            unsafe { WaitForSingleObject(child.as_raw_handle() as _, 5000) },
            0,
            "drop left a flooded grandchild alive"
        );
        drop(child);
        drop(bridge);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn oversized_windows_environment_fails_cleanly_without_truncating_command() {
        let directory = std::env::current_dir().unwrap();
        let (mut session, bridge) = managed(&directory);
        let command = "a".repeat(32_768);
        assert!(session.start_command(&command, &directory).is_err());
        assert!(!session.running());
        session
            .start_command("echo AFTER_LIMIT", &directory)
            .unwrap();
        assert_eq!(managed_completion(&mut session, &bridge).status, 0);
        assert!(session.screen().contents().contains("AFTER_LIMIT"));
    }
}
