// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let mut random = [0; 8];
        getrandom::getrandom(&mut random).unwrap();
        let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let path = env::temp_dir().join(format!("potyi-browser-test-{suffix}"));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn browser(root: &Fixture) -> Browser {
    Browser::new(&root.0, Arc::new(|| {})).unwrap()
}
fn drain(browser: &mut Browser) {
    // Other parity builders and reviewers run disk-heavy fixtures concurrently.
    // This is a hang guard; Stop responsiveness is asserted separately below.
    let deadline = Instant::now() + Duration::from_secs(30);
    while browser.job.is_some() {
        if !browser.poll() {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(Instant::now() < deadline, "Browser worker timed out");
    }
    assert!(!browser.pending());
}
fn all_rows(browser: &mut Browser) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut start = 0;
    while start < browser.len() {
        let batch = browser.rows(start, 128).unwrap();
        start += batch.len();
        rows.extend(batch);
    }
    rows
}
fn git_command(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .current_dir(root)
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
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn browser_default_listing_preserves_paths_hidden_parent_and_file_types() {
    let root = Fixture::new();
    fs::write(root.0.join("space name.txt"), "text").unwrap();
    fs::write(root.0.join("árvíz 東京.rs"), "fn main() {}\n").unwrap();
    fs::write(root.0.join(".hidden"), "hidden").unwrap();
    fs::write(root.0.join("binary.dat"), [0, 1, 2, 0xff]).unwrap();
    fs::create_dir(root.0.join("folder with spaces")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("missing-target", root.0.join("unreadable")).unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    let rows = all_rows(&mut browser);
    for (name, kind) in [
        ("space name.txt", Kind::Text),
        ("árvíz 東京.rs", Kind::Text),
        (".hidden", Kind::Text),
        ("binary.dat", Kind::Binary),
        ("folder with spaces", Kind::Directory),
    ] {
        let row = rows
            .iter()
            .find(|row| row.path.as_deref() == Some(root.0.join(name).as_path()))
            .unwrap();
        assert_eq!(row.kind, kind, "{name}");
        assert_eq!(row.color, Some(kind.color()));
        assert!(row.path.as_ref().unwrap().is_absolute());
    }
    assert!(
        rows.iter()
            .any(|row| row.text == "../" && row.path.as_deref() == root.0.parent())
    );
    #[cfg(unix)]
    assert_eq!(
        rows.iter()
            .find(|row| row.text == "unreadable")
            .unwrap()
            .kind,
        Kind::Unreadable
    );
    browser.execute("ls --hide-hidden").unwrap();
    let old_len = browser.len();
    drain(&mut browser);
    assert!(
        !browser
            .rows(old_len, 100)
            .unwrap()
            .iter()
            .any(|row| row.text == ".hidden")
    );
}

#[test]
fn browser_retained_links_survive_cd_and_navigation_checks_literal_locations() {
    let root = Fixture::new();
    fs::write(root.0.join("space name.txt"), "original").unwrap();
    fs::create_dir(root.0.join("other")).unwrap();
    fs::write(root.0.join("other/space name.txt"), "different").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    let index = all_rows(&mut browser)
        .iter()
        .position(|row| row.text == "space name.txt")
        .unwrap();
    assert_eq!(browser.execute("cd other").unwrap(), BrowseAction::Changed);
    drain(&mut browser);
    assert_eq!(browser.directory(), root.0.join("other"));
    assert_eq!(
        browser.rows(index, 1).unwrap()[0].path,
        Some(root.0.join("space name.txt"))
    );
    assert_eq!(
        browser.execute("view 'space name.txt:2:3'").unwrap(),
        BrowseAction::Open {
            path: root.0.join("other/space name.txt"),
            read_only: true,
            line: Some(2),
            column: Some(3)
        }
    );
    #[cfg(unix)]
    {
        fs::write(root.0.join("other/literal colon:2"), "literal").unwrap();
        assert_eq!(
            browser.execute("edit 'literal colon:2'").unwrap(),
            BrowseAction::Open {
                path: root.0.join("other/literal colon:2"),
                read_only: false,
                line: None,
                column: None
            }
        );
    }
    let directory = browser.directory().to_path_buf();
    browser.execute("cd missing-directory").unwrap();
    drain(&mut browser);
    assert_eq!(browser.directory(), directory);
    assert!(browser.status().is_some());
    assert!(browser.take_failure());
    assert!(!browser.take_failure());
    browser.refresh().unwrap();
    assert!(!browser.take_failure());
    drain(&mut browser);
}

#[test]
fn browser_native_commands_keep_existing_files_and_refresh_completion() {
    let root = Fixture::new();
    fs::write(root.0.join("existing.txt"), "keep these bytes").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    browser
        .execute("touch existing.txt 'new space é.txt'")
        .unwrap();
    drain(&mut browser);
    assert_eq!(
        fs::read(root.0.join("existing.txt")).unwrap(),
        b"keep these bytes"
    );
    assert!(root.0.join("new space é.txt").is_file());
    assert!(
        !browser
            .completion()
            .iter()
            .any(|entry| entry.path == root.0.join("new space é.txt"))
    );
    browser.execute("ls").unwrap();
    drain(&mut browser);
    assert!(
        browser
            .completion()
            .iter()
            .any(|entry| entry.path == root.0.join("new space é.txt"))
    );
    browser.execute("touch -c missing.txt").unwrap();
    drain(&mut browser);
    assert!(!root.0.join("missing.txt").exists());
    assert_eq!(
        browser.execute("'new space é.txt'").unwrap(),
        BrowseAction::Open {
            path: root.0.join("new space é.txt"),
            read_only: false,
            line: None,
            column: None
        }
    );
    let length = browser.len();
    browser.execute("pwd").unwrap();
    assert_eq!(
        browser.rows(length + 1, 1).unwrap()[0].text,
        display_path(&root.0)
    );
    assert_eq!(browser.execute("exit").unwrap(), BrowseAction::Editor);
    assert_eq!(browser.execute("editor").unwrap(), BrowseAction::Editor);
}

#[test]
fn browser_shell_expressions_and_executables_are_not_intercepted() {
    let root = Fixture::new();
    fs::write(root.0.join("script"), "echo original").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.0.join("script"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(windows)]
    fs::write(root.0.join("script.cmd"), "echo original").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    for command in [
        "ls | grep rs",
        "cd ..; pwd",
        "touch a && touch b",
        "ls > saved.txt",
        "printf 'hello'",
        "echo 'unterminated",
        "ls *.rs",
        "cd $HOME",
    ] {
        #[cfg(windows)]
        if command == "cd ..; pwd" || command == "ls *.rs" || command == "cd $HOME" {
            continue;
        }
        assert_eq!(
            browser.execute(command).unwrap(),
            BrowseAction::NotHandled,
            "{command}"
        );
    }
    #[cfg(unix)]
    {
        assert_eq!(
            browser.execute("cd \"$HOME\"").unwrap(),
            BrowseAction::NotHandled
        );
        assert_eq!(
            browser.execute("ls \"${HOME}\"").unwrap(),
            BrowseAction::NotHandled
        );
        assert_eq!(
            browser.execute("./script").unwrap(),
            BrowseAction::NotHandled
        );
        fs::write(root.0.join("sh"), "local text").unwrap();
        assert!(command_on_path("sh", &root.0));
        assert_eq!(browser.execute("sh").unwrap(), BrowseAction::NotHandled);
        assert!(matches!(
            browser.execute("./sh").unwrap(),
            BrowseAction::Open { .. }
        ));
    }
    #[cfg(windows)]
    assert_eq!(
        browser.execute("script.cmd").unwrap(),
        BrowseAction::NotHandled
    );
    assert!(!root.0.join("a").exists());
    assert!(!root.0.join("saved.txt").exists());
    assert!(browser.execute("ls --unsupported").is_err());
    assert!(browser.execute("edit 'unfinished").is_err());
}

#[test]
fn browser_cancelled_workers_cannot_apply_stale_rows_or_cwd() {
    let root = Fixture::new();
    fs::create_dir(root.0.join("large")).unwrap();
    fs::create_dir(root.0.join("next")).unwrap();
    for index in 0..1500 {
        fs::write(root.0.join(format!("large/old-{index:04}")), "old").unwrap();
    }
    fs::write(root.0.join("next/current.txt"), "new").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    let old_len = browser.len();
    browser.enter(&root.0.join("large")).unwrap();
    // Allow the old producer to fill its bounded queue, without consuming it.
    thread::sleep(Duration::from_millis(30));
    browser.enter(&root.0.join("next")).unwrap();
    drain(&mut browser);
    assert_eq!(browser.directory(), root.0.join("next"));
    let rows = browser.rows(old_len, 256).unwrap();
    assert!(rows.iter().any(|row| row.text == "current.txt"));
    assert!(!rows.iter().any(|row| row.text.starts_with("old-")));
    browser.enter(&root.0.join("large")).unwrap();
    thread::sleep(Duration::from_millis(30));
    let started = Instant::now();
    browser.stop();
    assert!(started.elapsed() < Duration::from_millis(100));
    assert!(!browser.running());
    browser.clear().unwrap();
    thread::sleep(Duration::from_millis(20));
    assert!(!browser.poll());
    assert_eq!(browser.len(), 0);
}

#[test]
fn browser_disk_store_and_completion_remain_bounded() {
    let root = Fixture::new();
    let mut browser = browser(&root);
    drain(&mut browser);
    browser.clear().unwrap();
    for index in 0..80_000 {
        browser
            .append(Row {
                text: format!("ROW_{index:06}_{}", "x".repeat(160)),
                path: Some(root.0.join(format!("path-{index:06}"))),
                kind: Kind::Text,
                color: Some(Kind::Text.color()),
            })
            .unwrap();
    }
    assert!(browser.len() <= 65_536);
    assert!(browser.len() < 80_000);
    assert_eq!(browser.store.lock().unwrap().buffer_capacity(), 120 * 1024);
    assert_eq!(
        browser.rows(browser.len() - 1, 1).unwrap()[0].path,
        Some(root.0.join("path-079999"))
    );
    assert_eq!(browser.rows(0, usize::MAX).unwrap().len(), MAX_READ_ROWS);
    browser.clear().unwrap();
    for index in 0..4500 {
        fs::write(root.0.join(format!("candidate-{index:04}")), "x").unwrap();
    }
    browser.refresh().unwrap();
    drain(&mut browser);
    assert!(browser.completion().len() <= MAX_COMPLETIONS);
    assert!(browser.completion_bytes <= MAX_COMPLETION_BYTES);
    assert!(browser.completion().len() < 4500);
}

#[test]
fn browser_git_colours_are_requested_after_names_and_do_not_lock_the_index() {
    let root = Fixture::new();
    git_command(&root.0, &["init", "-q"]);
    fs::write(root.0.join("changed é.txt"), "original\n").unwrap();
    fs::write(root.0.join(".gitignore"), "ignored.txt\n").unwrap();
    git_command(&root.0, &["add", "."]);
    git_command(&root.0, &["commit", "-qm", "initial"]);
    fs::write(root.0.join("changed é.txt"), "changed\n").unwrap();
    fs::write(root.0.join("untracked.txt"), "new\n").unwrap();
    fs::write(root.0.join("ignored.txt"), "cache\n").unwrap();
    let index = fs::read(root.0.join(".git/index")).unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    let rows = all_rows(&mut browser);
    for (name, status) in [
        ("changed é.txt", git::Status::Modified),
        ("untracked.txt", git::Status::Untracked),
        ("ignored.txt", git::Status::Ignored),
    ] {
        let row = rows
            .iter()
            .find(|row| row.path.as_deref() == Some(root.0.join(name).as_path()))
            .unwrap();
        assert_eq!(row.color, Some(status.color()), "{name}");
    }
    assert_eq!(fs::read(root.0.join(".git/index")).unwrap(), index);
    assert!(!root.0.join(".git/index.lock").exists());

    // Completed groups carry their own frozen colors. A new clean listing must
    // neither recolor the older modified row nor lose its original link.
    let old_record = {
        let mut store = browser.store.lock().unwrap();
        (0..store.len())
            .find_map(|index| {
                let record = store.read(index).unwrap();
                let RecordData::Native(bytes) = &record.data else {
                    return None;
                };
                (decode_native(bytes).unwrap().path == Some(root.0.join("changed é.txt")))
                    .then_some(record)
            })
            .unwrap()
    };
    fs::write(root.0.join("changed é.txt"), "original\n").unwrap();
    browser.refresh().unwrap();
    drain(&mut browser);
    let previous = browser
        .store
        .lock()
        .unwrap()
        .read_id(old_record.id)
        .unwrap();
    assert_eq!(previous.data, old_record.data);
    assert_eq!(
        browser.decorate_record(&previous).unwrap().color,
        Some(git::Status::Modified.color())
    );
    let new = all_rows(&mut browser)
        .into_iter()
        .rev()
        .find(|row| row.path == Some(root.0.join("changed é.txt")))
        .unwrap();
    assert_eq!(new.color, Some(Kind::Text.color()));
}

#[test]
fn browser_and_terminal_share_ordered_storage_and_single_clear_epoch() {
    let root = Fixture::new();
    fs::write(root.0.join("first.txt"), "first").unwrap();
    let shared = Transcript::shared().unwrap();
    let mut browser =
        Browser::open_with_transcript(&root.0, Arc::new(|| {}), shared.clone()).unwrap();
    drain(&mut browser);
    let native_len = browser.len();
    let shell_source = shared.lock().unwrap().next_source();
    let shell = shared
        .lock()
        .unwrap()
        .append_terminal(&root.0, shell_source, 80, false, b"shell output")
        .unwrap();
    assert_eq!(browser.len(), native_len + 1);
    assert_eq!(browser.rows(native_len, 1).unwrap()[0].text, "shell output");
    browser.execute("pwd").unwrap();
    let records = shared.lock().unwrap().read_rows(native_len, 4).unwrap();
    assert_eq!(records[0].id, shell.record_id);
    assert_ne!(records[1].source, shell_source);
    assert!(matches!(records[1].data, RecordData::Header(_)));
    assert!(matches!(
        records[3].data,
        RecordData::Result { status: 0, .. }
    ));
    let epoch = shared.lock().unwrap().epoch();
    shared.lock().unwrap().clear().unwrap();
    browser.cancel_for_clear();
    assert_eq!(shared.lock().unwrap().epoch(), epoch + 1);
    assert_eq!(browser.len(), 0);
    assert!(!browser.poll());
    assert!(shared.lock().unwrap().read_anchor(shell).is_err());
}

#[test]
fn synchronous_native_command_cancels_previous_source_before_starting() {
    let root = Fixture::new();
    fs::create_dir(root.0.join("other")).unwrap();
    fs::write(root.0.join("other/stale.txt"), "stale").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    browser.enter(&root.0.join("other")).unwrap();
    thread::sleep(Duration::from_millis(30));
    browser.execute("pwd").unwrap();
    assert!(!browser.working());
    assert_eq!(browser.directory(), root.0);
    assert!(!browser.poll());
    assert!(
        !all_rows(&mut browser)
            .iter()
            .any(|row| row.text == "stale.txt")
    );
}

#[test]
fn native_packing_metadata_keeps_modes_unicode_width_and_git_replacements() {
    let root = Fixture::new();
    fs::write(root.0.join("東京.txt"), "text").unwrap();
    fs::write(root.0.join("tiny"), "text").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    let first = {
        let mut store = browser.store.lock().unwrap();
        (0..store.len())
            .find_map(|index| {
                let record = store.read(index).unwrap();
                let RecordData::Native(bytes) = &record.data else {
                    return None;
                };
                (decode_native(bytes).unwrap().text == "東京.txt").then_some(record)
            })
            .unwrap()
    };
    let RecordData::Native(bytes) = &first.data else {
        unreachable!();
    };
    assert_eq!(native_packing(bytes), None);
    assert_ne!(bytes[14] & 16, 0);
    assert_eq!(decode_native(bytes).unwrap().text, "東京.txt");
    for command in ["ls -1", "ls --one-per-line", "ls -l"] {
        let start = browser.len();
        browser.execute(command).unwrap();
        drain(&mut browser);
        let mut store = browser.store.lock().unwrap();
        let records = store.read_rows(start, 100).unwrap();
        assert!(
            records
                .iter()
                .filter_map(|record| match &record.data {
                    RecordData::Native(bytes) => Some(bytes),
                    _ => None,
                })
                .all(|bytes| native_packing(bytes).is_none())
        );
    }
    let current = browser.store.lock().unwrap().read_id(first.id).unwrap();
    assert_eq!(current.data, first.data);
}

#[test]
fn browser_native_path_records_roundtrip_without_utf8_loss() {
    let row = Row {
        text: "é file".into(),
        path: Some(PathBuf::from("/tmp/é file")),
        kind: Kind::Text,
        color: Some((1, 2, 3)),
    };
    assert_eq!(decode(&encode(&row).unwrap()).unwrap(), row);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let row = Row {
            path: Some(PathBuf::from(std::ffi::OsString::from_vec(vec![
                b'/', b'x', 0xff,
            ]))),
            ..row
        };
        assert_eq!(decode(&encode(&row).unwrap()).unwrap(), row);
    }
    assert!(decode(&[0; 3]).is_err());
    assert!(
        encode(&Row {
            text: "x".repeat(MAX_ROW_BYTES),
            path: None,
            kind: Kind::Header,
            color: None
        })
        .is_err()
    );
}

#[test]
fn direct_folder_entry_is_silent_but_typed_cd_echoes_the_command() {
    let root = Fixture::new();
    fs::create_dir(root.0.join("folder with spaces")).unwrap();
    fs::write(root.0.join("folder with spaces/inside.txt"), "hello").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    let before = {
        let store = browser.store.lock().unwrap();
        store.base_id() + store.len() as u64
    };
    browser.enter(Path::new("folder with spaces")).unwrap();
    drain(&mut browser);
    assert_eq!(browser.directory(), root.0.join("folder with spaces"));
    {
        let mut store = browser.store.lock().unwrap();
        let end = store.base_id() + store.len() as u64;
        for id in before..end {
            assert!(
                !matches!(store.read_id(id).unwrap().data, RecordData::Header(_)),
                "A folder click must not invent a typed cd command"
            );
        }
    }
    browser.execute("cd ..").unwrap();
    drain(&mut browser);
    let mut store = browser.store.lock().unwrap();
    let mut found = false;
    for index in 0..store.len() {
        found |= matches!(store.read(index).unwrap().data, RecordData::Header(text) if text == "$ cd ..");
    }
    assert!(found, "Typed cd must retain its command echo");
}

#[test]
fn successful_native_listing_persists_the_legacy_final_blank_line() {
    let root = Fixture::new();
    fs::write(root.0.join("file.txt"), "text").unwrap();
    let mut browser = browser(&root);
    drain(&mut browser);
    for command in ["ls", "ls -l", "ls -1"] {
        browser.execute(command).unwrap();
        drain(&mut browser);
        let mut store = browser.store.lock().unwrap();
        let end = store.base_id() + store.len() as u64;
        assert!(matches!(
            store.read_id(end - 1).unwrap().data,
            RecordData::Result { status: 0, .. }
        ));
        let RecordData::Native(bytes) = store.read_id(end - 2).unwrap().data else {
            panic!("A completed listing must retain its final blank text row");
        };
        let blank = decode_native(&bytes).unwrap();
        assert_eq!(blank.text, "", "{command}");
        assert_eq!(blank.path, None);
    }
}

fn copied_browser_output(browser: &Browser) -> String {
    use super::super::output_selection::{self, Bounds, RecordText, TextSource};
    struct Saved(Vec<Record>);
    impl TextSource for Saved {
        fn bounds(&self) -> Bounds {
            let first = self.0.first().unwrap();
            Bounds {
                epoch: first.epoch,
                first: first.id,
                end: first.id + self.0.len() as u64,
            }
        }
        fn text(&mut self, id: u64) -> io::Result<RecordText> {
            let index = (id - self.0[0].id) as usize;
            let record = &self.0[index];
            let next = self.0.get(index + 1);
            let mut text = output_selection::record_text_at_end(record, next.is_none())?;
            text.join_next &= next.is_some_and(|next| output_selection::joins_next(record, next));
            text.terminated |= next.is_some_and(|next| {
                next.source == record.source && matches!(next.data, RecordData::Result { .. })
            });
            Ok(text)
        }
    }
    let mut store = browser.store.lock().unwrap();
    let records = (0..store.len())
        .map(|index| store.read(index).unwrap())
        .collect();
    drop(store);
    output_selection::Selection::default()
        .copy(&mut Saved(records), true, false)
        .unwrap()
}

#[test]
fn listing_argument_headers_and_blank_separators_match_legacy_copy() {
    let root = Fixture::new();
    for (directory, filename) in [("alpha", "inside-a"), ("beta", "inside-b")] {
        fs::create_dir(root.0.join(directory)).unwrap();
        fs::write(root.0.join(directory).join(filename), "text").unwrap();
    }
    let mut browser = browser(&root);
    drain(&mut browser);
    browser.clear().unwrap();
    browser.execute("ls -1 alpha beta").unwrap();
    drain(&mut browser);
    assert_eq!(
        copied_browser_output(&browser),
        "$ ls -1 alpha beta\nalpha:\n../\ninside-a\n\nbeta:\n../\ninside-b\n\n"
    );
    browser.clear().unwrap();
    browser.execute("cd alpha").unwrap();
    drain(&mut browser);
    assert_eq!(
        copied_browser_output(&browser),
        format!(
            "$ cd alpha\n{}:\n../        inside-a   \n\n",
            display_path(&root.0.join("alpha"))
        )
    );
}

#[test]
fn recursive_listing_visits_children_at_each_directory_entry_in_filesystem_order() {
    let root = Fixture::new();
    for directory in ["alpha", "beta"] {
        fs::create_dir(root.0.join(directory)).unwrap();
        fs::write(root.0.join(directory).join("child.txt"), "text").unwrap();
    }
    fs::write(root.0.join("loose.txt"), "text").unwrap();
    let mut expected = "$ ls -R1\n.:\n../\n".to_owned();
    // The longest fixture name is nine scalars; legacy measurement reserves
    // one slash and two separating spaces, giving twelve-column cells.
    let mut packed = "$ ls -R\n.:\n../         ".to_owned();
    let mut partial_parent_line = true;
    for entry in fs::read_dir(&root.0).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().unwrap().is_dir() {
            expected.push_str(&format!(
                "{name}/\n\n{}:\n../\nchild.txt\n",
                display_path(&entry.path())
            ));
            packed.push_str(&format!(
                "{name}/{}\n\n{}:\n../         child.txt   \n",
                " ".repeat(12 - name.len() - 1),
                display_path(&entry.path())
            ));
            partial_parent_line = false;
        } else {
            expected.push_str(&format!("{name}\n"));
            packed.push_str(&format!("{name}{}", " ".repeat(12 - name.len())));
            partial_parent_line = true;
        }
    }
    expected.push('\n');
    if partial_parent_line {
        packed.push('\n');
    }
    packed.push('\n');
    let mut browser = browser(&root);
    drain(&mut browser);
    browser.clear().unwrap();
    browser.execute("ls -R1").unwrap();
    drain(&mut browser);
    assert_eq!(copied_browser_output(&browser), expected);
    browser.clear().unwrap();
    browser.execute("ls -R").unwrap();
    drain(&mut browser);
    assert_eq!(
        copied_browser_output(&browser),
        packed,
        "A partial child row must finish before the next parent entry"
    );
}

#[test]
fn wide_listing_retains_long_filename_padding_in_disk_metadata() {
    let root = Fixture::new();
    fs::write(root.0.join("x".repeat(255)), "long").unwrap();
    for name in ["a", "b", "c"] {
        fs::write(root.0.join(name), "short").unwrap();
    }
    let mut browser = browser(&root);
    drain(&mut browser);
    browser.set_columns(1000);
    browser.execute("ls").unwrap();
    drain(&mut browser);
    assert_eq!(browser.columns, 1000);
    let mut store = browser.store.lock().unwrap();
    let mut wide = false;
    for index in 0..store.len() {
        let record = store.read(index).unwrap();
        if record.source != browser.source {
            continue;
        }
        if let RecordData::Native(bytes) = &record.data {
            let row = decode_native(bytes).unwrap();
            if matches!(row.text.as_str(), "a" | "b" | "c") && bytes[14] & 8 != 0 {
                assert_eq!(native_padding(bytes), 257);
                assert_ne!(bytes[14] & 32, 0);
                let text = super::super::output_selection::record_text(&record).unwrap();
                assert_eq!(text.text, format!("{}{}", row.text, " ".repeat(257)));
                wide = true;
            }
        }
    }
    assert!(
        wide,
        "At least two short names must have ninth-bit padding regardless of filesystem order"
    );
}
