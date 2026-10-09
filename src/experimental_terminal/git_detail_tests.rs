// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::super::browser::{BrowseAction, Kind, decode_native, native_packing};
use super::*;
use std::{env, fs, time::Instant};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let mut random = [0; 8];
        getrandom::getrandom(&mut random).unwrap();
        let name: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let root = env::temp_dir().join(format!("potyi-git-view-test-{name}"));
        fs::create_dir(&root).unwrap();
        Self(root.canonicalize().unwrap())
    }
    fn git(&self, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(&self.0)
            .args([
                "-c",
                "user.name=Potyi Test",
                "-c",
                "user.email=potyi@example.invalid",
                "-c",
                "commit.gpgSign=false",
                "-c",
                "core.hooksPath=.no-hooks",
                "-c",
                "core.autocrlf=false",
            ])
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().into()
    }
    fn repository(&self) {
        self.git(&["init", "--quiet"]);
        fs::write(self.0.join("space 東京.rs"), "old\n").unwrap();
        self.git(&["add", "--", "space 東京.rs"]);
        self.git(&["commit", "--quiet", "-m", "first"]);
        fs::write(self.0.join("space 東京.rs"), "new\n").unwrap();
        self.git(&["commit", "--quiet", "-am", "second árvíz"]);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn drain(browser: &mut Browser) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while browser.working() {
        if !browser.poll() {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(Instant::now() < deadline, "Git view timed out");
    }
}
fn records(store: &SharedTranscript) -> Vec<Record> {
    let mut result = Vec::new();
    let mut index = 0;
    let mut store = store.lock().unwrap();
    while index < store.len() {
        let rows = store.read_rows(index, 128).unwrap();
        index += rows.len();
        result.extend(rows);
    }
    result
}
fn logical_text(rows: &[Record]) -> String {
    rows.iter()
        .map(|record| match &record.data {
            RecordData::Native(bytes) => decode_native(bytes).unwrap().text,
            RecordData::Header(text) | RecordData::Result { text, .. } => text.clone(),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn request_retains_repository_options_and_leaves_shell_and_other_git_alone() {
    let cwd = env::temp_dir().canonicalize().unwrap();
    let request = Request::from_command(
        "git -C 'folder with spaces' -c color.ui=always --no-pager log --oneline --graph",
        &cwd,
    )
    .unwrap();
    assert_eq!(request.repository.cwd, cwd);
    assert_eq!(
        request.repository.arguments,
        [
            "-C",
            "folder with spaces",
            "-c",
            "color.ui=always",
            "--no-pager"
        ]
    );
    assert_eq!(request.subcommand, "log");
    for command in [
        "git commit -am message",
        "git log | cat",
        "git log > log.txt",
        "git diff --ext-diff",
        "git show --textconv",
        "git log --output=log.txt",
        "git status -z",
        "git status --porcelain=v2",
        "git -C log",
        "echo git log",
    ] {
        assert!(Request::from_command(command, &cwd).is_none(), "{command}");
    }
    let request =
        Request::from_command("git diff --color=always -- 'space 東京.rs'", &cwd).unwrap();
    let command = request.command();
    let arguments: Vec<_> = command
        .get_args()
        .map(|argument| argument.to_str().unwrap())
        .collect();
    assert_eq!(
        arguments,
        [
            "--no-pager",
            "diff",
            "--color=always",
            "--no-ext-diff",
            "--no-textconv",
            "--color=never",
            "--",
            "space 東京.rs"
        ]
    );
}

#[test]
fn exact_hash_links_require_saved_git_context_and_a_complete_token() {
    let cwd = env::temp_dir().canonicalize().unwrap();
    let request = Request::from_command("git -C 'space 東京' log", &cwd).unwrap();
    let context = encode_context(&request.repository).unwrap();
    for (line, expected) in [
        ("* | abc1234 second", Some(4..11)),
        ("commit 0123456789abcdef", Some(7..23)),
        ("abcd", Some(0..4)),
        ("    abc1234 is a message", None),
        ("abc", None),
        ("abcd; command", None),
        ("abcdmore", None),
    ] {
        assert_eq!(hash_range(line), expected, "{line}");
    }
    let record = Record {
        id: 17,
        epoch: 3,
        source: 5,
        cwd: cwd.clone(),
        data: RecordData::Native(
            encode_row("* abc1234 second", Some((1, 2, 3)), &context).unwrap(),
        ),
    };
    let commit = GitDetail::commit_at(&record, 2).unwrap().unwrap();
    assert_eq!(commit.hash, "abc1234");
    assert_eq!(commit.directory(), cwd);
    assert_eq!(commit.repo.arguments, ["-C", "space 東京"]);
    assert!(GitDetail::commit_at(&record, 1).unwrap().is_none());
    assert!(GitDetail::commit_at(&record, 9).unwrap().is_none());
    let RecordData::Native(bytes) = &record.data else {
        unreachable!()
    };
    assert!(native_packing(bytes).is_none());
    assert_eq!(decode_native(bytes).unwrap().path, None);
    let ordinary = Record {
        data: RecordData::Native(encode_row("abc1234 ordinary", None, &[]).unwrap()),
        ..record.clone()
    };
    assert!(GitDetail::commit_at(&ordinary, 1).unwrap().is_none());
    let terminal = Record {
        data: RecordData::Terminal {
            cols: 80,
            wrapped: false,
            formatted: b"abc1234 terminal".to_vec(),
        },
        ..record
    };
    assert!(GitDetail::commit_at(&terminal, 1).unwrap().is_none());
}

#[test]
fn line_decoding_and_git_colors_are_chunk_independent_and_bounded() {
    let bytes=b"\x1b[31mcommit abc1234\x1b[0m\r\n\x1b]0;hidden\x07@@ hunk\n+\xe6\x9d\xb1\xe4\xba\xac\n-old\n\nlast";
    let collect = |chunks: Vec<&[u8]>| {
        let mut lines = Lines::new(1024);
        let mut colors = Colors::default();
        let mut result = Vec::new();
        for chunk in chunks {
            lines
                .push(chunk, |text| {
                    result.push((text.to_owned(), colors.line(text)));
                    Ok(())
                })
                .unwrap();
        }
        lines
            .finish(|text| {
                result.push((text.to_owned(), colors.line(text)));
                Ok(())
            })
            .unwrap();
        result
    };
    let whole = collect(vec![bytes]);
    assert_eq!(whole, collect(bytes.chunks(1).collect()));
    assert_eq!(whole[0], ("commit abc1234".into(), Some((225, 195, 120))));
    assert_eq!(whole[1].1, Some((120, 195, 235)));
    assert_eq!(whole[2], ("+東京".into(), Some((150, 220, 165))));
    assert_eq!(whole[3].1, Some((245, 145, 145)));
    assert_eq!(whole[4].0, "");
    assert_eq!(whole[5].0, "last");
    let mut lines = Lines::new(32);
    assert!(lines.push(&[b'x'; 33], |_| Ok(())).is_err());
    assert_eq!(lines.bytes.len(), 32);
    let mut colors = Colors::default();
    assert_eq!(
        colors.line("Changes to be committed:"),
        Some((225, 195, 120))
    );
    assert_eq!(colors.line("\tmodified: file"), Some((150, 220, 165)));
    assert_eq!(colors.line(" M file"), Some((245, 145, 145)));
    assert_eq!(colors.line("?? file"), Some((235, 185, 110)));
}

#[test]
fn log_links_open_full_saved_detail_without_touching_the_base_store() {
    let root = Fixture::new();
    root.repository();
    fs::create_dir(root.0.join("unrelated")).unwrap();
    let cwd = root.0.join("unrelated").canonicalize().unwrap();
    let store = Transcript::shared().unwrap();
    let wake: Wake = Arc::new(|| {});
    let mut browser =
        Browser::open_empty_with_transcript(&cwd, wake.clone(), store.clone()).unwrap();
    assert_eq!(
        browser
            .execute("git -C '..' log --oneline --graph")
            .unwrap(),
        BrowseAction::Changed
    );
    drain(&mut browser);
    assert_eq!(browser.status(), None);
    let base = records(&store);
    let row=base.iter().find(|record| matches!(&record.data,RecordData::Native(bytes) if decode_native(bytes).unwrap().text.contains("second árvíz"))).unwrap();
    let RecordData::Native(bytes) = &row.data else {
        unreachable!()
    };
    let text = decode_native(bytes).unwrap().text;
    let range = hash_range(&text).unwrap();
    let commit = GitDetail::commit_at(row, range.start).unwrap().unwrap();
    assert_eq!(commit.directory(), cwd);
    // A later native directory change does not rewrite the saved repository.
    browser.enter(&root.0).unwrap();
    drain(&mut browser);
    let before = records(&store);
    let detail = GitDetail::open(commit.clone(), wake).unwrap();
    let (mut detail, detail_store) = detail.into_parts();
    assert!(!Arc::ptr_eq(&store, &detail_store));
    drain(&mut detail);
    assert_eq!(detail.status(), None);
    let detail_rows = records(&detail_store);
    let detail_text = logical_text(&detail_rows);
    // Original commit detail hashes are text; only log hashes open a view.
    for row in &detail_rows {
        if let RecordData::Native(bytes) = &row.data {
            if decode_native(bytes).unwrap().text.starts_with("commit ") {
                assert!(GitDetail::commit_at(row, 7).unwrap().is_none());
            }
        }
    }
    for expected in [
        "Back / Alt+Left",
        "Author:",
        "Commit:",
        "second árvíz",
        "diff --git",
        "-old",
        "+new",
    ] {
        assert!(detail_text.contains(expected), "{expected}: {detail_text}");
    }
    assert!(
        detail_rows
            .iter()
            .filter_map(|record| match &record.data {
                RecordData::Native(bytes) => Some(decode_native(bytes).unwrap()),
                _ => None,
            })
            .any(|row| row.text == "+new" && row.color == Some((150, 220, 165)))
    );
    assert_eq!(records(&store), before);
    // Explicit Again reruns only this fixed commit view.
    detail.clear().unwrap();
    detail
        .execute_git_request(Request::for_commit(&commit).unwrap())
        .unwrap();
    drain(&mut detail);
    assert!(logical_text(&records(&detail_store)).contains("second árvíz"));
    assert_eq!(records(&store), before);
    assert!(base.iter().all(|record| record.cwd == cwd));
}

#[test]
fn native_git_diff_and_status_keep_their_colors_and_exit_status() {
    let root = Fixture::new();
    root.repository();
    fs::write(root.0.join("space 東京.rs"), "working\n").unwrap();
    fs::write(root.0.join("untracked.txt"), "new").unwrap();
    let store = Transcript::shared().unwrap();
    let mut browser =
        Browser::open_empty_with_transcript(&root.0, Arc::new(|| {}), store.clone()).unwrap();
    browser.execute("git diff --exit-code").unwrap();
    drain(&mut browser);
    assert!(browser.take_failure());
    let rows = records(&store);
    assert!(matches!(
        &rows.last().unwrap().data,
        RecordData::Result { status: 1, .. }
    ));
    let text = logical_text(&rows);
    assert!(text.contains("-new") && text.contains("+working"));
    browser.execute("git status --short").unwrap();
    drain(&mut browser);
    assert!(!browser.take_failure());
    let rows = records(&store);
    for (needle, color) in [(" M ", (245, 145, 145)), ("?? ", (235, 185, 110))] {
        assert!(rows.iter().any(|record|matches!(&record.data,RecordData::Native(bytes) if {let row=decode_native(bytes).unwrap();row.text.starts_with(needle)&&row.color==Some(color)})));
    }
    assert_eq!(
        browser.execute("git log | cat").unwrap(),
        BrowseAction::NotHandled
    );
}

#[test]
fn git_row_context_survives_header_eviction_and_metadata_is_validated() {
    let cwd = env::temp_dir().canonicalize().unwrap();
    let request =
        Request::from_command("git -C 'space 東京' --git-dir=metadata log", &cwd).unwrap();
    let context = encode_context(&request.repository).unwrap();
    let mut store = Transcript::new().unwrap();
    let source = store.next_source();
    let header = store.append_header(&cwd, source, "git log").unwrap();
    let filler = vec![b'x'; 64 * 1024];
    for _ in 0..140 {
        store.append_native(&cwd, source, &filler).unwrap();
    }
    assert!(store.read_anchor(header).is_err());
    let anchor = store
        .append_native(
            &cwd,
            source,
            &encode_row("abc1234 final", None, &context).unwrap(),
        )
        .unwrap();
    let commit = GitDetail::commit_at(&store.read_anchor(anchor).unwrap(), 0)
        .unwrap()
        .unwrap();
    assert_eq!(
        commit.repo.arguments,
        ["-C", "space 東京", "--git-dir=metadata"]
    );
    let mut invalid = context.clone();
    invalid[0] = 255;
    assert!(decode_context(&invalid, &cwd).is_err());
    assert!(decode_context(&context[..context.len() - 1], &cwd).is_err());
    let mut extended = context;
    extended.push(0);
    assert!(decode_context(&extended, &cwd).is_err());
    assert!(encode_row(&"x".repeat(MAX_ROW_BYTES), None, &[]).is_err());
}

#[cfg(unix)]
#[test]
fn cancellation_releases_full_queues_and_never_writes_into_a_replaced_view() {
    use std::os::unix::fs::PermissionsExt;
    let root = Fixture::new();
    let script = root.0.join("git");
    let pid = root.0.join("pid");
    fs::write(&script,"#!/bin/sh\nprintf '%s' \"$$\" > pid\nwhile :; do printf 'abc1234 repeated line with content content content content content\\n'; done\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let store = Transcript::shared().unwrap();
    let mut browser =
        Browser::open_empty_with_transcript(&root.0, Arc::new(|| {}), store.clone()).unwrap();
    let request = Request::from_command(&format!("'{}' log", script.display()), &root.0).unwrap();
    browser.execute_git_request(request).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pid.exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    // No polls: the producer fills the capped browser queue, then blocks.
    thread::sleep(Duration::from_millis(80));
    let started = Instant::now();
    browser.stop();
    assert!(started.elapsed() < Duration::from_millis(100));
    browser.execute("pwd").unwrap();
    let before = records(&store);
    let process: i32 = fs::read_to_string(pid).unwrap().parse().unwrap();
    while unsafe { libc::kill(process, 0) } == 0 {
        assert!(Instant::now() < deadline, "cancelled Git producer survived");
        thread::sleep(Duration::from_millis(2));
    }
    for _ in 0..10 {
        browser.poll();
    }
    assert_eq!(records(&store), before);
    let text = logical_text(&before);
    assert!(text.contains("[listing stopped]"));
    assert!(!text.contains("repeated line"));
}

#[test]
fn git_encoded_metadata_does_not_create_browsing_paths() {
    let cwd = env::temp_dir().canonicalize().unwrap();
    let request = Request::from_command("git log", &cwd).unwrap();
    let bytes = encode_row(
        "abc1234 subject",
        None,
        &encode_context(&request.repository).unwrap(),
    )
    .unwrap();
    let row = decode_native(&bytes).unwrap();
    assert_eq!(row.kind, Kind::Header);
    assert_eq!(row.path, None);
    assert!(native_packing(&bytes).is_none());
}

#[cfg(windows)]
mod windows_process_tree {
    use super::*;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use winapi::um::{
        processthreadsapi::OpenProcess, synchapi::WaitForSingleObject, winnt::SYNCHRONIZE,
    };

    fn powershell() -> PathBuf {
        PathBuf::from(env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe")
    }
    fn command(root: &Fixture, script: &Path) -> Command {
        let mut command = Command::new(powershell());
        command
            .current_dir(&root.0)
            .args(["-NoProfile", "-NonInteractive", "-File"])
            .arg(script);
        command
    }
    fn process_handle(path: &Path) -> OwnedHandle {
        let deadline = Instant::now() + Duration::from_secs(10);
        let pid = loop {
            if let Ok(text) = fs::read_to_string(path) {
                if let Ok(pid) = text.trim().parse::<u32>() {
                    break pid;
                }
            }
            assert!(
                Instant::now() < deadline,
                "fixture did not publish {}",
                path.display()
            );
            thread::sleep(Duration::from_millis(2));
        };
        let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
        assert!(
            !handle.is_null(),
            "fixture process exited before its handle was pinned"
        );
        unsafe { OwnedHandle::from_raw_handle(handle as _) }
    }
    fn assert_exited(process: &OwnedHandle) {
        assert_eq!(
            unsafe { WaitForSingleObject(process.as_raw_handle() as _, 5000) },
            0,
            "Job descendant survived cleanup"
        );
    }

    #[test]
    fn windows_git_process_preserves_crt_arguments_unicode_env_removal_and_actual_status() {
        let root = Fixture::new();
        let script = root.0.join("argv space 東京.ps1");
        fs::write(&script, "[Console]::OutputEncoding = [Text.Encoding]::UTF8\nforeach ($value in $args) { Write-Output ('ARG:' + $value + ':END') }\nWrite-Output ('ENV:' + $env:POTYI_NATIVE_GIT_TEST + ':END')\nif (Test-Path Env:PATH) { exit 9 }\nexit 37\n").unwrap();
        let mut command = command(&root, &script);
        command.args([
            "é space 東京",
            "quote\"inside",
            "trailing\\",
            "",
            "slash\\\"quote",
        ]);
        command
            .env("POTYI_NATIVE_GIT_TEST", "árvíz 東京")
            .env_remove("PATH");
        let mut process = Process::spawn(&mut command).unwrap();
        let stdout = process.0.stdout.take().unwrap();
        let stderr = process.0.stderr.take().unwrap();
        let (sender, receiver) = mpsc::sync_channel(QUEUED_CHUNKS);
        reader(stdout, 0, sender.clone()).unwrap();
        reader(stderr, 1, sender).unwrap();
        let mut text = [Vec::new(), Vec::new()];
        let mut ended = [false; 2];
        let mut status = None;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ended.iter().all(|end| *end) || status.is_none() {
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(ReadEvent::Bytes(stream, bytes)) => text[stream].extend(bytes),
                Ok(ReadEvent::End(stream)) => ended[stream] = true,
                Ok(ReadEvent::Error(error)) => panic!("{error}"),
                _ => (),
            }
            if status.is_none() {
                status = process.try_wait().unwrap();
            }
            assert!(Instant::now() < deadline);
        }
        assert_eq!(status, Some(37));
        let stdout = String::from_utf8(text[0].clone()).unwrap();
        for argument in [
            "é space 東京",
            "quote\"inside",
            "trailing\\",
            "",
            "slash\\\"quote",
        ] {
            assert!(stdout.contains(&format!("ARG:{argument}:END")), "{stdout}");
        }
        assert!(stdout.contains("ENV:árvíz 東京:END"));
    }

    #[test]
    fn windows_git_drop_kills_grandchildren_and_releases_full_reader_queues() {
        let root = Fixture::new();
        let script = root.0.join("producer.ps1");
        fs::write(&script, "[IO.File]::WriteAllText('parent.pid',[string]$PID)\n$p=Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList '-NoProfile','-NonInteractive','-Command','Start-Sleep 60' -NoNewWindow -PassThru\n[IO.File]::WriteAllText('child.pid',[string]$p.Id)\nwhile ($true) { Write-Output 'abc1234 repeated line with content content content' }\n").unwrap();
        let mut process = Process::spawn(&mut command(&root, &script)).unwrap();
        let (sender, receiver) = mpsc::sync_channel(QUEUED_CHUNKS);
        reader(process.0.stdout.take().unwrap(), 0, sender.clone()).unwrap();
        reader(process.0.stderr.take().unwrap(), 1, sender).unwrap();
        let parent = process_handle(&root.0.join("parent.pid"));
        let child = process_handle(&root.0.join("child.pid"));
        // Consumers deliberately do not drain the bounded queue. This matches
        // a replaced browser whose producer is blocked publishing a row.
        thread::sleep(Duration::from_millis(100));
        drop(process);
        assert_exited(&parent);
        assert_exited(&child);
        drop(receiver);
    }

    #[test]
    fn windows_git_main_exit_ends_descendant_held_pipes_before_completion() {
        let root = Fixture::new();
        let script = root.0.join("exit-producer.ps1");
        fs::write(&script, "$p=Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList '-NoProfile','-NonInteractive','-Command','Start-Sleep 60' -NoNewWindow -PassThru\n[IO.File]::WriteAllText('child.pid',[string]$p.Id)\nWrite-Output 'FINAL_MAIN_TAIL'\nStart-Sleep -Seconds 1\nexit 41\n").unwrap();
        let mut process = Process::spawn(&mut command(&root, &script)).unwrap();
        let (sender, receiver) = mpsc::sync_channel(QUEUED_CHUNKS);
        reader(process.0.stdout.take().unwrap(), 0, sender.clone()).unwrap();
        reader(process.0.stderr.take().unwrap(), 1, sender).unwrap();
        let child = process_handle(&root.0.join("child.pid"));
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut ended = [false; 2];
        let mut status = None;
        let mut text = Vec::new();
        while !ended.iter().all(|end| *end) || status.is_none() {
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(ReadEvent::Bytes(_, bytes)) => text.extend(bytes),
                Ok(ReadEvent::End(stream)) => ended[stream] = true,
                Ok(ReadEvent::Error(error)) => panic!("{error}"),
                _ => (),
            }
            if status.is_none() {
                status = process.try_wait().unwrap();
            }
            assert!(
                Instant::now() < deadline,
                "descendant kept a Git output pipe open"
            );
        }
        assert_eq!(status, Some(41));
        assert!(String::from_utf8_lossy(&text).contains("FINAL_MAIN_TAIL"));
        assert_exited(&child);
    }
}
