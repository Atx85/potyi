// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later
//! Commit links and a single reversible terminal detail view. No output copies.
use super::*;
use std::ops::Range;

pub(super) const MAX_LINKS: usize = 8192;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Commit {
    pub hash: String,
    pub(super) repo: Arc<Repository>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Repository {
    cwd: PathBuf,
    program: String,
    args: Vec<String>,
}
impl Repository {
    pub fn from_log(command: &str, cwd: &Path) -> Option<Arc<Self>> {
        if has_shell_operators(command) {
            return None;
        }
        let words = touch_arguments(command).ok()?;
        let program = words.first()?;
        if !matches!(Path::new(program).file_name()?.to_str()?, "git" | "git.exe") {
            return None;
        }
        let mut index = 1;
        while let Some(word) = words.get(index) {
            if word == "log" {
                return Some(Arc::new(Self {
                    cwd: cwd.into(),
                    program: program.clone(),
                    args: words[1..index].to_vec(),
                }));
            }
            if matches!(
                word.as_str(),
                "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace"
            ) {
                words.get(index + 1)?;
                index += 2;
            } else if matches!(
                word.as_str(),
                "--no-pager"
                    | "--paginate"
                    | "--bare"
                    | "--no-optional-locks"
                    | "--no-replace-objects"
            ) || word.starts_with("--git-dir=")
                || word.starts_with("--work-tree=")
                || word.starts_with("--namespace=")
            {
                index += 1;
            } else {
                return None;
            }
        }
        None
    }
}

pub(super) fn hash_range(line: &str) -> Option<Range<usize>> {
    let prefix = if line
        .trim_start_matches(' ')
        .starts_with(['|', '*', '/', '\\'])
    {
        line.len()
            - line
                .trim_start_matches([' ', '|', '*', '/', '\\', '_', '.'])
                .len()
    } else {
        0
    };
    let body = &line[prefix..];
    let start = prefix + if body.starts_with("commit ") { 7 } else { 0 };
    let bytes = line.as_bytes();
    let length = bytes[start..]
        .iter()
        .take_while(|b| b.is_ascii_hexdigit())
        .count();
    if !(4..=64).contains(&length)
        || bytes
            .get(start + length)
            .is_some_and(|b| !b.is_ascii_whitespace())
    {
        return None;
    }
    Some(start..start + length)
}
impl Commit {
    fn command(&self) -> io::Result<Command> {
        if self.hash.len() < 4
            || self.hash.len() > 64
            || !self.hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid commit hash",
            ));
        }
        let mut command = Command::new(&self.repo.program);
        command
            .args(&self.repo.args)
            .args([
                "--no-pager",
                "show",
                "--no-ext-diff",
                "--no-textconv",
                "--color=never",
                "--format=fuller",
                "--stat",
                "--patch",
                "--root",
                "--first-parent",
                "--submodule=short",
            ])
            .arg(format!("{}^{{commit}}", self.hash))
            .arg("--");
        configure_process(&mut command, &self.repo.cwd);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        Ok(command)
    }
}

pub(super) struct View {
    output: PieceTable,
    entries: Vec<OutputEntry>,
    scanner: locations::Scanner,
    colors: colors::Colors,
    selection: selection::Selection,
    input: String,
    cursor: usize,
    cwd: PathBuf,
    completions: completion::Cache,
    last_command: Option<String>,
    status: Option<String>,
    scroll: usize,
    history_position: Option<usize>,
    history_draft: String,
    pub commit: Commit,
}

impl Terminal {
    pub fn can_go_back(&self) -> bool {
        self.git_view.is_some()
    }

    pub fn open_commit(&mut self, commit: Commit) -> io::Result<()> {
        self.cancel_git_status();
        if self.is_running() {
            self.status =
                Some("Wait for the command to finish, or stop it before opening a commit.".into());
            return Ok(());
        }
        let events = self
            .events
            .clone()
            .ok_or_else(|| io::Error::other("Terminal events are unavailable"))?;
        // Validate and allocate before changing either view.
        let mut command = commit.command()?;
        let mut output = PieceTable::empty()?;
        output.insert(
            0,
            &format!(
                "Commit {} · Back / Alt+Left returns to the log\n",
                commit.hash
            ),
        )?;
        let child = command.spawn()?;
        if self.git_view.is_none() {
            self.git_view = Some(Box::new(View {
                output: std::mem::replace(&mut self.output, output),
                entries: std::mem::take(&mut self.entries),
                scanner: std::mem::take(&mut self.location_scanner),
                colors: std::mem::take(&mut self.colors),
                selection: std::mem::take(&mut self.selection),
                input: std::mem::take(&mut self.input),
                cursor: self.cursor,
                cwd: self.cwd.clone(),
                completions: std::mem::take(&mut self.completions),
                last_command: self.last_command.take(),
                status: self.status.take(),
                scroll: self.scroll_back,
                history_position: self.history_position.take(),
                history_draft: std::mem::take(&mut self.history_draft),
                commit: commit.clone(),
            }));
        } else {
            self.output = output;
            self.entries.clear();
            self.location_scanner = locations::Scanner::default();
            self.colors = colors::Colors::default();
            self.selection = selection::Selection::default();
            self.input.clear();
            self.git_view.as_mut().unwrap().commit = commit.clone();
        }
        self.cursor = 0;
        self.selection.focused = true;
        self.scroll_back = usize::MAX;
        self.cwd = commit.repo.cwd.clone();
        self.reset_detail_layout();
        self.colors.begin("git show");
        // The reader threads and bounded output queue are shared with ordinary commands.
        if let Err(error) = self.start_child(child, &events) {
            self.restore_git_view();
            return Err(error);
        }
        Ok(())
    }

    fn reset_detail_layout(&mut self) {
        self.output_generation = self.output_generation.wrapping_add(1);
        self.output_layout_reset = self.output_layout_reset.wrapping_add(1);
        self.output_trimmed = 0;
        self.ansi_state = AnsiState::Ground;
        self.pending_utf8.clear();
        self.pending_carriage_return = false;
    }

    pub fn go_back(&mut self) -> io::Result<()> {
        if !self.can_go_back() {
            return Ok(());
        }
        if self.is_running() {
            self.stop()?;
            if self.running.is_some() {
                // Restore only after all old pipe events/data have drained.
                self.git_back_pending = true;
                self.status = Some("Stopping diff and returning to log…".into());
                return Ok(());
            }
        }
        self.restore_git_view();
        Ok(())
    }

    pub(super) fn restore_git_view(&mut self) {
        let Some(view) = self.git_view.take() else {
            return;
        };
        self.output = view.output;
        self.entries = view.entries;
        self.location_scanner = view.scanner;
        self.colors = view.colors;
        self.selection = view.selection;
        self.input = view.input;
        self.cursor = view.cursor;
        self.cwd = view.cwd;
        self.completions = view.completions;
        self.last_command = view.last_command;
        self.status = view.status;
        self.scroll_back = view.scroll;
        self.history_position = view.history_position;
        self.history_draft = view.history_draft;
        self.git_back_pending = false;
        self.reset_detail_layout();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_cover_only_hashes_and_keep_the_original_repository() {
        let cwd = std::env::temp_dir();
        let mut scanner =
            locations::Scanner::new("git -C 'project with spaces' log --oneline --graph", &cwd);
        let text = "* 290a677 (HEAD -> main) Unicode 🦀 message\n| * 4e3f6ad another\n|/\n";
        let mut entries = Vec::new();
        for (offset, ch) in text.char_indices() {
            scanner.append(&ch.to_string(), offset, &mut entries);
        }
        assert_eq!(entries.len(), 2);
        assert_eq!(&text[entries[0].range.clone()], "290a677");
        let TerminalAction::Commit(commit) = entries[0].action().unwrap() else {
            panic!()
        };
        assert_eq!(commit.repo.cwd, cwd);
        assert_eq!(commit.repo.args, ["-C", "project with spaces"]);
        assert_eq!(hash_range("commit abcdef0123456789\n"), Some(7..23));
        for line in [
            "    abcdef ordinary message",
            "abc123;bad command",
            "abc too short",
            "not-a-hash",
        ] {
            assert!(hash_range(line).is_none(), "{line}");
        }
        for command in [
            "echo git log",
            "git status",
            "git log | cat",
            "cd project && git log",
            "git -C",
        ] {
            assert!(Repository::from_log(command, &cwd).is_none(), "{command}");
        }
        let mut invalid = commit.clone();
        invalid.hash = "abcd;echo bad".into();
        assert!(invalid.command().is_err());
    }

    #[test]
    fn commit_link_storage_is_bounded_and_trim_rebases_clicks() {
        let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        terminal.location_scanner = locations::Scanner::new("git log --oneline", &terminal.cwd);
        terminal
            .append_text(&"abcdef1 message\n".repeat(MAX_LINKS + 10))
            .unwrap();
        assert_eq!(terminal.entries.len(), MAX_LINKS);
        terminal
            .append_text(&"padding\n".repeat(MAX_OUTPUT_BYTES / 8))
            .unwrap();
        assert!(terminal.entries.is_empty());
        assert!(terminal.output.len() <= MAX_OUTPUT_BYTES);
    }

    fn git(root: &Path, args: &[&str]) -> std::process::Output {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Potyi Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Potyi Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "potyi-git-navigation-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture() -> (Fixture, Commit, String) {
        let root = Fixture::new();
        git(&root.0, &["init", "-q"]);
        fs::write(root.0.join("example.txt"), "old line\n").unwrap();
        git(&root.0, &["add", "example.txt"]);
        git(
            &root.0,
            &["-c", "commit.gpgsign=false", "commit", "-qm", "First"],
        );
        fs::write(root.0.join("example.txt"), "new line\n").unwrap();
        git(&root.0, &["add", "example.txt"]);
        git(
            &root.0,
            &["-c", "commit.gpgsign=false", "commit", "-qm", "Second"],
        );
        let log = String::from_utf8(git(&root.0, &["log", "--oneline"]).stdout).unwrap();
        let hash = log.split_whitespace().next().unwrap().to_owned();
        let commit = Commit {
            hash,
            repo: Repository::from_log("git log --oneline", &root.0).unwrap(),
        };
        (root, commit, log)
    }
    fn drain(terminal: &mut Terminal, pump: &mut sdl3::EventPump) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while terminal.is_running() {
            terminal.poll_background().unwrap();
            assert!(Instant::now() < deadline, "Git process did not finish");
            if let Some(event) = pump.wait_event_timeout(Duration::from_millis(20))
                && let Some(event) = event.as_user_event_type::<TerminalEvent>()
            {
                terminal.handle_event(event).unwrap();
            }
        }
    }
    #[test]
    #[ignore = "requires SDL and git; run with SDL_VIDEODRIVER=dummy and --test-threads=1"]
    fn commit_diff_streams_and_back_restores_log_without_rerunning_it() {
        let sdl = sdl3::init().unwrap();
        let events = sdl.event().unwrap();
        register_test_events(&events);
        let mut pump = sdl.event_pump().unwrap();
        let (root, commit, log) = fixture();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.set_events(events.clone());
        terminal.clear().unwrap();
        terminal.location_scanner = locations::Scanner::new("git log --oneline", &root.0);
        terminal.colors.begin("git log --oneline");
        terminal.append_text(&log).unwrap();
        terminal.colors.end();
        terminal.scroll_back = 7;
        terminal.insert_text("unfinished command");
        terminal.last_command = Some("git log --oneline".into());
        let saved_input = terminal.input.clone();
        let saved_cursor = terminal.cursor;
        let action = terminal.action_at_output_offset(2).unwrap().unwrap();
        let TerminalAction::Commit(clicked) = action else {
            panic!()
        };
        assert_eq!(clicked, commit);
        terminal.open_commit(clicked).unwrap();
        assert!(terminal.can_go_back());
        drain(&mut terminal, &mut pump);
        let diff = terminal.output_text().unwrap();
        assert!(diff.contains("-old line"), "{diff}");
        assert!(diff.contains("+new line"));
        assert_eq!(
            terminal.output_color(diff.find("+new line").unwrap()),
            Some(colors::Style::Added.rgb())
        );
        // Changing the repository's log must not change the saved view on Back.
        git(
            &root.0,
            &[
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-qm",
                "Third",
            ],
        );
        terminal.go_back().unwrap();
        assert!(!terminal.can_go_back());
        assert_eq!(terminal.output_text().unwrap(), log);
        assert_eq!(terminal.scroll_back, 7);
        assert_eq!(terminal.input, saved_input);
        assert_eq!(terminal.cursor, saved_cursor);
        assert_eq!(terminal.last_command.as_deref(), Some("git log --oneline"));
        assert!(matches!(
            terminal.action_at_output_offset(2).unwrap(),
            Some(TerminalAction::Commit(_))
        ));
        terminal.open_commit(commit.clone()).unwrap();
        terminal.go_back().unwrap();
        drain(&mut terminal, &mut pump);
        assert!(!terminal.can_go_back());
        assert_eq!(terminal.output_text().unwrap(), log);
        terminal.poll_background().unwrap();
        assert_eq!(terminal.output_text().unwrap(), log);
        // Failed resolution stays in the detail view with an available Back action.
        let mut missing = commit;
        missing.hash = "0000000000000000000000000000000000000000".into();
        terminal.open_commit(missing).unwrap();
        drain(&mut terminal, &mut pump);
        assert!(terminal.output_text().unwrap().contains("fatal:"));
        terminal.go_back().unwrap();
        assert_eq!(terminal.output_text().unwrap(), log);
    }
}
