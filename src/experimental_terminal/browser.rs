// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Portable browsing beside the PTY. Rows and authoritative paths live in the
//! existing bounded disk ring; only worker batches and completion are cached.
use super::{
    Wake,
    git_detail::{self, Request},
    transcript::{Record, RecordData, SharedTranscript, Transcript},
};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

const MAX_COMMAND_BYTES: usize = 64 * 1024;
const MAX_ROW_BYTES: usize = 64 * 1024;
const MAX_COMMAND_PATHS: usize = 256;
const BATCH_BYTES: usize = 16 * 1024;
const BATCH_ROWS: usize = 64;
const QUEUED_BATCHES: usize = 8;
const MAX_READ_ROWS: usize = 256;
const MAX_COMPLETIONS: usize = 4096;
const MAX_COMPLETION_BYTES: usize = 256 * 1024;
const MAX_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Text,
    Binary,
    Directory,
    Unreadable,
    Header,
}

impl Kind {
    pub(crate) fn color(self) -> (u8, u8, u8) {
        // Legacy listings distinguish clickable names by underline; clean
        // file kinds and listing headers share the normal output foreground.
        (215, 220, 215)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) text: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) kind: Kind,
    pub(crate) color: Option<(u8, u8, u8)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Completion {
    pub(crate) path: PathBuf,
    pub(crate) directory: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BrowseAction {
    NotHandled,
    Changed,
    Open {
        path: PathBuf,
        read_only: bool,
        line: Option<usize>,
        column: Option<usize>,
    },
    Editor,
}

enum Message {
    Started(Option<PathBuf>),
    Rows(Vec<Vec<u8>>),
    NamesReady,
    Git(git::Snapshot),
    Finished(Result<i32, String>),
}

struct Job {
    receiver: Receiver<Message>,
    cancelled: Arc<AtomicBool>,
    wake_pending: Arc<AtomicBool>,
    running: bool,
}

impl Drop for Job {
    fn drop(&mut self) {
        // Dropping the receiver also releases a producer blocked on a full queue.
        self.cancelled.store(true, Ordering::Release);
    }
}

pub(crate) struct Browser {
    directory: PathBuf,
    store: SharedTranscript,
    source: u64,
    source_start: u64,
    git_source: Option<u64>,
    freeze: Option<(u64, u64)>,
    job: Option<Job>,
    wake: Wake,
    pending: bool,
    status: Option<String>,
    completions: Vec<Completion>,
    completion_bytes: usize,
    columns: usize,
    git: Option<git::Snapshot>,
    storage_failed: bool,
    failure: bool,
}

impl Browser {
    /// Opening the browser immediately lists this directory. Reopening the pane
    /// should reuse the Browser rather than creating another worker or file.
    pub(crate) fn new(directory: &Path, wake: Wake) -> io::Result<Self> {
        Self::open_with_transcript(directory, wake, Transcript::shared()?)
    }

    pub(crate) fn open_with_transcript(
        directory: &Path,
        wake: Wake,
        store: SharedTranscript,
    ) -> io::Result<Self> {
        let mut browser = Self::open_empty_with_transcript(directory, wake, store)?;
        browser.refresh()?;
        Ok(browser)
    }

    /// A saved Git detail owns a temporary store, without an initial listing.
    pub(crate) fn open_empty_with_transcript(
        directory: &Path,
        wake: Wake,
        store: SharedTranscript,
    ) -> io::Result<Self> {
        let directory = directory.canonicalize()?;
        if !directory.is_dir() {
            return Err(io::Error::other("Not a directory"));
        }
        Ok(Self {
            directory,
            store,
            source: 0,
            source_start: 0,
            git_source: None,
            freeze: None,
            job: None,
            wake,
            pending: false,
            status: None,
            completions: Vec::new(),
            completion_bytes: 0,
            columns: 80,
            git: None,
            storage_failed: false,
            failure: false,
        })
    }

    pub(crate) fn echo_command(&mut self, command: &str) -> io::Result<()> {
        self.begin(command)?;
        self.finish(0, "")
    }
    pub(crate) fn seed_banner(&mut self) -> io::Result<()> {
        for text in [
            "--- Pötyi terminal --------------------------------------------------",
            "Commands  cd, pwd, ls, touch, edit, view, clear, help, exit",
            "Ctrl+`  return to editor    Tab / Shift+Tab  complete paths",
            "Links   green: edit file    blue: folder    amber: binary",
            "----------------------------------------------------------------------",
        ] {
            self.append(Row {
                text: text.into(),
                path: None,
                kind: Kind::Header,
                color: Some(Kind::Header.color()),
            })?;
        }
        self.finish(0, "")
    }
    pub(crate) fn set_columns(&mut self, columns: usize) {
        self.columns = columns.clamp(1, 1000);
    }
    pub(crate) fn clear_status(&mut self) {
        self.status = None;
    }
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn len(&self) -> usize {
        self.store.lock().unwrap().len()
    }

    pub(crate) fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub(crate) fn set_status(&mut self, status: String) {
        self.status = Some(status);
    }

    /// The pane must discard a shell command waiting on a failed asynchronous
    /// directory change, rather than launching it in the previous directory.
    pub(crate) fn take_failure(&mut self) -> bool {
        std::mem::take(&mut self.failure)
    }

    pub(crate) fn pending(&self) -> bool {
        self.pending
    }

    pub(crate) fn working(&self) -> bool {
        self.job.is_some() || self.freeze.is_some()
    }

    pub(crate) fn running(&self) -> bool {
        self.job.as_ref().is_some_and(|job| job.running)
    }

    pub(crate) fn completion(&self) -> &[Completion] {
        &self.completions
    }

    /// Read a viewport only. Larger requests are capped; transcript-wide copy
    /// can iterate through successive bounded requests.
    pub(crate) fn rows(&mut self, start: usize, count: usize) -> io::Result<Vec<Row>> {
        let records = self
            .store
            .lock()
            .unwrap()
            .read_rows(start, count.min(MAX_READ_ROWS))?;
        records
            .iter()
            .map(|record| self.decorate_record(record))
            .collect()
    }

    pub(crate) fn decorate_record(&self, record: &Record) -> io::Result<Row> {
        let mut row = match &record.data {
            RecordData::Native(bytes) => decode_native(bytes)?,
            RecordData::Header(text) | RecordData::Result { text, .. } => Row {
                text: text.clone(),
                path: None,
                kind: Kind::Header,
                color: Some(Kind::Header.color()),
            },
            RecordData::Terminal { cols: 0, .. } => Row {
                text: super::output_selection::record_text(record)?.text,
                path: None,
                kind: Kind::Header,
                color: Some(Kind::Header.color()),
            },
            RecordData::Terminal {
                cols, formatted, ..
            } => {
                let mut parser = vt100::Parser::new(1, (*cols).clamp(1, 256), 0);
                parser.process(formatted);
                Row {
                    text: parser.screen().contents(),
                    path: None,
                    kind: Kind::Header,
                    color: Some(Kind::Header.color()),
                }
            }
        };
        if self.git_source == Some(record.source) {
            if let Some(status) = row
                .path
                .as_deref()
                .and_then(|path| self.git.as_ref()?.status(path))
            {
                row.color = Some(status.color());
            }
        }
        Ok(row)
    }

    pub(crate) fn clear(&mut self) -> io::Result<()> {
        self.cancel_for_clear();
        self.store.lock().unwrap().clear()
    }

    /// Pane Clear resets the shared store exactly once, then cancels each owner.
    pub(crate) fn cancel_for_clear(&mut self) {
        self.cancel();
        self.git = None;
        self.git_source = None;
        self.status = None;
        self.failure = false;
    }

    pub(crate) fn stop(&mut self) {
        if self.job.is_some() {
            self.cancel();
            let _ = self.finish(130, "[listing stopped]");
            self.status = None;
        } else {
            self.status = Some("No browser command is running".into());
        }
    }

    fn cancel(&mut self) {
        self.job.take();
        self.freeze = None;
        self.pending = false;
    }

    pub(crate) fn refresh(&mut self) -> io::Result<()> {
        self.start(Work::List {
            arguments: String::new(),
            enter: None,
        })
    }

    pub(crate) fn enter(&mut self, path: &Path) -> io::Result<()> {
        let path = self.directory.join(path);
        self.start_with_echo(
            Work::List {
                arguments: String::new(),
                enter: Some(path),
            },
            None,
        )
    }

    /// Handle only Pötyi's standalone commands. Shell expressions and every
    /// other command return NotHandled, without sending anything to the PTY.
    pub(crate) fn execute(&mut self, command: &str) -> io::Result<BrowseAction> {
        if command.len() > MAX_COMMAND_BYTES {
            return Err(io::Error::other("Command exceeds 64 KiB"));
        }
        let command = command.trim();
        if command.is_empty() || shell_operators(command) {
            return Ok(BrowseAction::NotHandled);
        }
        if let Some(request) = Request::from_command(command, &self.directory) {
            self.execute_git_request(request)?;
            return Ok(BrowseAction::Changed);
        }
        #[cfg(windows)]
        if let Some(root) = drive_root(command) {
            self.start_named(
                Work::List {
                    arguments: String::new(),
                    enter: Some(root),
                },
                command,
            )?;
            return Ok(BrowseAction::Changed);
        }
        let (name, arguments) = split_command(command);
        match name {
            "ls" => {
                // Report syntax errors before changing a useful existing listing.
                list_options(arguments)?;
                self.start_named(
                    Work::List {
                        arguments: arguments.into(),
                        enter: None,
                    },
                    command,
                )?;
            }
            "cd" => {
                #[cfg(windows)]
                let arguments = arguments
                    .strip_prefix("/d ")
                    .or_else(|| arguments.strip_prefix("/D "))
                    .unwrap_or(arguments);
                let value = single_path(arguments)?;
                let path = if value.is_empty() {
                    home().ok_or_else(|| io::Error::other("Home directory is unavailable"))?
                } else {
                    #[cfg(windows)]
                    if let Some(root) = drive_root(&value) {
                        self.start_named(
                            Work::List {
                                arguments: String::new(),
                                enter: Some(root),
                            },
                            command,
                        )?;
                        return Ok(BrowseAction::Changed);
                    }
                    expand_home(&value)
                };
                self.start_named(
                    Work::List {
                        arguments: String::new(),
                        enter: Some(self.directory.join(path)),
                    },
                    command,
                )?;
            }
            "pwd" if arguments.is_empty() => {
                self.begin(command)?;
                self.append(Row {
                    text: display_path(&self.directory),
                    path: None,
                    kind: Kind::Header,
                    color: Some(Kind::Header.color()),
                })?;
                self.finish(0, "Done")?;
                self.status = None;
            }
            "touch" => {
                let (no_create, paths) = touch_options(arguments, &self.directory)?;
                self.start_named(Work::Touch { no_create, paths }, command)?;
            }
            "clear" if arguments.is_empty() => self.clear()?,
            "exit" | "editor" if arguments.is_empty() => return Ok(BrowseAction::Editor),
            "help" if arguments.is_empty() => {
                self.begin(command)?;
                for text in "cd PATH        change directory\n\
pwd            show current directory\n\
ls [OPTIONS] [PATH]  list directory contents, including dotfiles\n\
ls --hide-hidden  hide dotfiles and dotfolders\n\
touch [-c] [--] PATH...  create files or update their timestamps\n\
Tab / Shift+Tab  cycle matching filenames from the latest ls or cd\n\
Ctrl/Cmd+V     paste into command input\n\
Shift+Up      select output from command input; Esc returns to input\n\
Ctrl/Cmd+C     copy selection; Ctrl+Shift+C copies all; Ctrl+C stops if unselected\n\
Vim output    h/j/k/l, w/b, 0/$, gg/G, v/V selection, y copy\n\
Underlined names are clickable; folders end with / (e.g. src/)\n\
Git colors: amber modified, green added, cyan untracked, muted ignored, pink conflict\n\
edit PATH[:LINE[:COLUMN]]  edit a file\n\
PATH           open a text file (e.g. Cargo.lock or ./Cargo.lock)\n\
view PATH[:LINE[:COLUMN]]  open read-only\n\
clear          clear terminal output\n\
exit           return to the editor\n\
:exit          close this split pane, keeping the other pane open\n\
grep [OPTIONS] PATTERN [FILE...]  run installed grep with its supported flags\n\
grep -nH PATTERN FILE  clickable results jump to the reported line\n\
Pipelines and redirects run through the system shell (e.g. ls | grep .rs).\n"
                    .lines()
                {
                    self.append(Row {
                        text: text.into(),
                        path: None,
                        kind: Kind::Header,
                        color: Some(Kind::Header.color()),
                    })?;
                }
                #[cfg(windows)]
                self.append(Row {
                    text:
                        "C: or cd C:    switch to the drive root (C:\\); cd C:\\PATH opens a folder"
                            .into(),
                    path: None,
                    kind: Kind::Header,
                    color: Some(Kind::Header.color()),
                })?;
                self.finish(0, "Done")?;
                self.status = None;
            }
            "edit" | "view" => {
                let value = single_path(arguments)?;
                if value.is_empty() {
                    return Err(io::Error::other("Usage: edit/view PATH[:LINE[:COLUMN]]"));
                }
                let (path, line, column) = located_path(&value, &self.directory);
                return Ok(BrowseAction::Open {
                    path,
                    read_only: name == "view",
                    line,
                    column,
                });
            }
            _ => {
                let Ok(words) = words(command) else {
                    return Ok(BrowseAction::NotHandled);
                };
                let [value] = words.as_slice() else {
                    return Ok(BrowseAction::NotHandled);
                };
                let path = self.directory.join(expand_home(value));
                let Ok(metadata) = fs::metadata(&path) else {
                    return Ok(BrowseAction::NotHandled);
                };
                if metadata.is_file()
                    && !executable(&path, &metadata)
                    && (value.contains(['/', '\\']) || !command_on_path(value, &self.directory))
                    && classify(&path, &metadata) == Kind::Text
                {
                    return Ok(BrowseAction::Open {
                        path,
                        read_only: false,
                        line: None,
                        column: None,
                    });
                }
                return Ok(BrowseAction::NotHandled);
            }
        }
        Ok(BrowseAction::Changed)
    }

    fn append(&mut self, row: Row) -> io::Result<()> {
        if self.storage_failed {
            return Err(io::Error::other(
                "Browser history is unavailable; reopen the terminal",
            ));
        }
        self.append_encoded(&encode(&row)?)
    }

    fn append_encoded(&mut self, bytes: &[u8]) -> io::Result<()> {
        let result = self
            .store
            .lock()
            .unwrap()
            .append_native(&self.directory, self.source, bytes);
        if let Err(error) = result {
            self.storage_failed = true;
            self.failure = true;
            return Err(error);
        }
        Ok(())
    }

    fn begin(&mut self, command: &str) -> io::Result<()> {
        self.begin_with_echo(Some(command))
    }

    fn begin_with_echo(&mut self, command: Option<&str>) -> io::Result<()> {
        if self.job.is_some() {
            self.cancel();
            self.finish(130, "[listing stopped]")?;
        }
        self.git = None;
        self.git_source = None;
        self.freeze = None;
        let mut store = self.store.lock().unwrap();
        self.source = store.next_source();
        self.source_start = if let Some(command) = command {
            let title = if command.starts_with("Commit ")
                && command.contains(" · Back / Alt+Left returns to the log")
            {
                command.to_owned()
            } else {
                format!("$ {command}")
            };
            store
                .append_header(&self.directory, self.source, &title)?
                .record_id
        } else {
            store.base_id() + store.len() as u64
        };
        Ok(())
    }

    fn finish(&mut self, status: i32, text: &str) -> io::Result<()> {
        self.store
            .lock()
            .unwrap()
            .append_result(&self.directory, self.source, status, text)?;
        Ok(())
    }

    fn start(&mut self, work: Work) -> io::Result<()> {
        let command = match &work {
            Work::List {
                arguments,
                enter: Some(path),
            } => format!("cd {}{}", display_path(path), arguments),
            Work::List { arguments, .. } => format!("ls {arguments}").trim_end().to_owned(),
            Work::Touch { no_create, paths } => {
                let mut command = if *no_create { "touch -c" } else { "touch" }.to_owned();
                for path in paths {
                    command.push(' ');
                    command.push_str(&display_path(path));
                }
                command
            }
            Work::Git(request) => request.label().to_owned(),
        };
        self.start_named(work, &command)
    }

    fn start_named(&mut self, work: Work, command: &str) -> io::Result<()> {
        self.start_with_echo(work, Some(command))
    }

    fn start_with_echo(&mut self, work: Work, command: Option<&str>) -> io::Result<()> {
        let status = if matches!(&work, Work::Git(_)) {
            "Running Git… Stop cancels"
        } else {
            "Listing… Stop cancels"
        };
        self.begin_with_echo(command)?;
        let job = match start_worker(
            work,
            self.directory.clone(),
            self.wake.clone(),
            self.columns,
        ) {
            Ok(job) => job,
            Err(error) => {
                self.finish(1, &error.to_string())?;
                return Err(error);
            }
        };
        self.job = Some(job);
        self.failure = false;
        self.status = Some(status.into());
        Ok(())
    }

    pub(crate) fn execute_git_request(&mut self, request: Request) -> io::Result<()> {
        let label = request.label().to_owned();
        self.start_named(Work::Git(request), &label)
    }

    /// Persist a bounded portion of the current source's Git colors. The scan
    /// carries stable IDs and yields to input rather than walking a full listing.
    fn freeze_step(&mut self) -> io::Result<()> {
        let Some((mut id, end)) = self.freeze else {
            return Ok(());
        };
        let deadline = Instant::now() + Duration::from_millis(2);
        for _ in 0..8 {
            let base = self.store.lock().unwrap().base_id();
            id = id.max(base);
            if id >= end {
                break;
            }
            let record = self.store.lock().unwrap().read_id(id)?;
            if record.source == self.source && matches!(record.data, RecordData::Native(_)) {
                let row = self.decorate_record(&record)?;
                let mut bytes = encode(&row)?;
                if let RecordData::Native(previous) = &record.data {
                    bytes[14..16].copy_from_slice(&previous[14..16]);
                    if *previous != bytes {
                        self.store.lock().unwrap().replace_native(id, &bytes)?;
                    }
                }
            }
            id += 1;
            if Instant::now() >= deadline {
                break;
            }
        }
        self.freeze = (id < end).then_some((id, end));
        // Finished is queued behind Git. Continue polling once after the last
        // freeze batch even if its wake was coalesced with the Git message.
        self.pending = true;
        Ok(())
    }

    /// Consume one bounded message and yield to input. Old workers cannot apply
    /// stale results because each job owns a receiver discarded on cancellation.
    pub(crate) fn poll(&mut self) -> bool {
        self.pending = false;
        if self.freeze.is_some() {
            if let Err(error) = self.freeze_step() {
                self.cancel();
                self.failure = true;
                self.status = Some(format!("Browser storage failed: {error}"));
            }
            return true;
        }
        let Some(job) = &self.job else {
            return false;
        };
        job.wake_pending.store(false, Ordering::Release);
        let message = match job.receiver.try_recv() {
            Ok(message) => message,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.job = None;
                self.failure = true;
                self.status = Some("Browser worker ended unexpectedly".into());
                return true;
            }
        };
        self.pending = true;
        let result = match message {
            Message::Started(directory) => {
                if let Some(directory) = directory {
                    self.directory = directory;
                }
                self.completions.clear();
                self.completion_bytes = 0;
                Ok(())
            }
            Message::Rows(rows) => {
                let mut result = Ok(());
                for bytes in rows {
                    let row = match decode_native(&bytes) {
                        Ok(row) => row,
                        Err(error) => {
                            result = Err(error);
                            break;
                        }
                    };
                    if let Some(path) = &row.path {
                        let bytes = path.as_os_str().len();
                        if self.completions.len() < MAX_COMPLETIONS
                            && self.completion_bytes + bytes <= MAX_COMPLETION_BYTES
                        {
                            self.completions.push(Completion {
                                path: path.clone(),
                                directory: row.kind == Kind::Directory,
                            });
                            self.completion_bytes += bytes;
                        }
                    }
                    if let Err(error) = self.append_encoded(&bytes) {
                        result = Err(error);
                        break;
                    }
                }
                result.and_then(|()| self.store.lock().unwrap().flush())
            }
            Message::NamesReady => {
                self.job.as_mut().unwrap().running = false;
                self.status = Some("Checking Git… Stop cancels".into());
                Ok(())
            }
            Message::Git(snapshot) => {
                self.git = Some(snapshot);
                self.git_source = Some(self.source);
                let store = self.store.lock().unwrap();
                self.freeze = Some((
                    self.source_start.max(store.base_id()),
                    store.base_id() + store.len() as u64,
                ));
                Ok(())
            }
            Message::Finished(result) => {
                self.job = None;
                self.pending = false;
                let (status, error) = match result {
                    Ok(0) => (0, None),
                    Ok(status) => (status, Some(format!("Git exited with status {status}"))),
                    Err(error) => (1, Some(error)),
                };
                self.failure = status != 0;
                self.status = error;
                let text = self.status.clone().unwrap_or_else(|| "Done".into());
                self.finish(status, &text)
            }
        };
        if let Err(error) = result {
            self.cancel();
            self.failure = true;
            self.status = Some(format!("Browser storage failed: {error}"));
        }
        true
    }
}

enum Work {
    List {
        arguments: String,
        enter: Option<PathBuf>,
    },
    Touch {
        no_create: bool,
        paths: Vec<PathBuf>,
    },
    Git(Request),
}

fn start_worker(work: Work, directory: PathBuf, wake: Wake, width: usize) -> io::Result<Job> {
    let (sender, receiver) = mpsc::sync_channel(QUEUED_BATCHES);
    let cancelled = Arc::new(AtomicBool::new(false));
    let wake_pending = Arc::new(AtomicBool::new(false));
    let worker_cancelled = cancelled.clone();
    let worker_pending = wake_pending.clone();
    thread::Builder::new()
        .name("potyi-browser".into())
        .spawn(move || {
            let mut writer = Writer {
                sender,
                cancelled: worker_cancelled,
                wake_pending: worker_pending,
                wake,
                batch: Vec::new(),
                bytes: 0,
                width,
                column_position: 0,
            };
            let result = run_work(work, directory, &mut writer);
            if !writer.batch.is_empty() {
                let _ = writer.flush();
            }
            let _ = writer.send(Message::Finished(result.map_err(|e| e.to_string())));
        })?;
    Ok(Job {
        receiver,
        cancelled,
        wake_pending,
        running: true,
    })
}

struct Writer {
    sender: SyncSender<Message>,
    cancelled: Arc<AtomicBool>,
    wake_pending: Arc<AtomicBool>,
    wake: Wake,
    batch: Vec<Vec<u8>>,
    bytes: usize,
    width: usize,
    column_position: usize,
}

impl Writer {
    fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Browsing cancelled",
            ))
        } else {
            Ok(())
        }
    }
    fn send(&self, message: Message) -> io::Result<()> {
        self.check()?;
        self.sender
            .send(message)
            .map_err(|_| io::Error::new(io::ErrorKind::Interrupted, "Browser closed"))?;
        if !self.wake_pending.swap(true, Ordering::AcqRel) {
            (self.wake)();
        }
        Ok(())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.batch.is_empty() {
            return Ok(());
        }
        self.bytes = 0;
        let batch = std::mem::take(&mut self.batch);
        self.send(Message::Rows(batch))
    }
    fn row(&mut self, row: Row) -> io::Result<()> {
        self.encoded(encode(&row)?)
    }
    fn encoded(&mut self, bytes: Vec<u8>) -> io::Result<()> {
        self.check()?;
        let size = bytes.len();
        if self.bytes + size > BATCH_BYTES {
            self.flush()?;
        }
        self.bytes += size;
        self.batch.push(bytes);
        if self.bytes >= BATCH_BYTES || self.batch.len() >= BATCH_ROWS {
            self.flush()?;
        }
        Ok(())
    }
    fn header(&mut self, text: String) -> io::Result<()> {
        self.row(Row {
            text,
            path: None,
            kind: Kind::Header,
            color: Some(Kind::Header.color()),
        })
    }
    fn finish_columns(&mut self) -> io::Result<()> {
        if self.column_position != 0 {
            // A zero-text fixed row terminates an unfinished packed line. It
            // joins the prior entry, then contributes just that line's LF,
            // including when a recursive child is followed by a parent entry.
            let mut bytes = encode(&Row {
                text: String::new(),
                path: None,
                kind: Kind::Header,
                color: Some(Kind::Header.color()),
            })?;
            bytes[14] = 16;
            self.encoded(bytes)?;
            self.column_position = 0;
        }
        Ok(())
    }
    fn entry(
        &mut self,
        path: PathBuf,
        name: &str,
        options: &LsOptions,
        cell: u16,
    ) -> io::Result<()> {
        self.check()?;
        let metadata = fs::metadata(&path);
        let kind = metadata
            .as_ref()
            .map_or(Kind::Unreadable, |m| classify(&path, m));
        let mut text = String::new();
        if options.long {
            let marker = if kind == Kind::Directory { 'd' } else { '-' };
            let size = metadata.as_ref().map_or("?".into(), |m| {
                if options.human {
                    human_size(m.len())
                } else {
                    m.len().to_string()
                }
            });
            text.push_str(&format!("{marker} {size:>12} "));
        }
        for ch in name.chars() {
            if ch.is_control() {
                text.extend(ch.escape_default());
            } else {
                text.push(ch);
            }
        }
        if kind == Kind::Directory && !name.ends_with('/') {
            text.push('/');
        }
        let mut bytes = encode(&Row {
            text,
            path: Some(path),
            kind,
            color: Some(kind.color()),
        })?;
        bytes[14] = if options.long { 2 } else { 0 };
        if cell != 0 {
            let count = ((self.width + 2) / usize::from(cell)).max(1);
            self.column_position += 1;
            let joins = self.column_position < count;
            if !joins {
                self.column_position = 0;
            }
            bytes[14] |= 16 | if joins { 8 } else { 0 };
            let padding = if joins {
                usize::from(cell)
                    .saturating_sub(decode_native(&bytes)?.text.chars().count())
                    .max(2)
            } else {
                0
            };
            // At most two columns fit when padding exceeds 255: the legacy
            // 1000-column limit bounds it to 500, encoded without larger rows.
            debug_assert!(padding <= 511);
            bytes[14] |= if padding >= 256 { 32 } else { 0 };
            bytes[15] = padding as u8;
        }
        self.encoded(bytes)
    }
    fn list(
        &mut self,
        path: &Path,
        name: &str,
        options: &LsOptions,
        header: bool,
        depth: usize,
    ) -> io::Result<()> {
        self.check()?;
        if depth > MAX_DEPTH {
            return Err(io::Error::other("Directory recursion exceeds 64 levels"));
        }
        let metadata = fs::metadata(path)?;
        if !metadata.is_dir() {
            let parent = path.parent().unwrap_or(Path::new(".")).canonicalize()?;
            return self.entry(
                parent.join(path.file_name().unwrap_or_default()),
                name,
                options,
                0,
            );
        }
        let directory = path.canonicalize()?;
        self.column_position = 0;
        if header {
            self.header(format!("{}:", display_path(Path::new(name))))?;
        }
        let cell = if options.long || options.one {
            0
        } else {
            // As in :term, measure names on the worker without collecting them.
            // A second streaming pass emits entries in filesystem order.
            let mut longest = 3usize;
            for entry in fs::read_dir(&directory)? {
                self.check()?;
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if options.all || !name.starts_with('.') {
                    let length = name
                        .chars()
                        .map(|ch| {
                            if ch.is_control() {
                                ch.escape_default().count()
                            } else {
                                1
                            }
                        })
                        .sum::<usize>();
                    longest = longest.max(length.saturating_add(1));
                }
            }
            let cell = longest.saturating_add(2);
            if cell > 1000 { 0 } else { cell as u16 }
        };
        if let Some(parent) = directory.parent() {
            self.entry(parent.into(), "..", options, cell)?;
        }
        self.flush()?;
        // Streaming filesystem order avoids an output-sized vector or sort index.
        for entry in fs::read_dir(&directory)? {
            self.check()?;
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !options.all && name.starts_with('.') {
                continue;
            }
            self.entry(directory.join(entry.file_name()), &name, options, cell)?;
            if options.recursive && entry.file_type()?.is_dir() {
                // The blank row terminates any partial packed row, then
                // separates the child header. Preserve filesystem depth-first
                // order instead of postponing all children until names end.
                self.finish_columns()?;
                self.header(String::new())?;
                self.list(
                    &entry.path(),
                    &display_path(&entry.path()),
                    options,
                    true,
                    depth + 1,
                )?;
                self.column_position = 0;
            }
        }
        self.finish_columns()?;
        self.flush()
    }
}

fn run_work(work: Work, initial: PathBuf, writer: &mut Writer) -> io::Result<i32> {
    let (arguments, enter) = match work {
        Work::List { arguments, enter } => (arguments, enter),
        Work::Touch { no_create, paths } => {
            let now = std::time::SystemTime::now();
            let times = fs::FileTimes::new().set_accessed(now).set_modified(now);
            let mut errors = Vec::new();
            for path in paths {
                writer.check()?;
                let result = OpenOptions::new()
                    .write(true)
                    .create(!no_create)
                    .truncate(false)
                    .open(&path)
                    .and_then(|file| file.set_times(times));
                if let Err(error) = result {
                    if no_create && error.kind() == io::ErrorKind::NotFound {
                        continue;
                    }
                    if errors.len() < 8 {
                        errors.push(format!("{}: {error}", display_path(&path)));
                    }
                }
            }
            return if errors.is_empty() {
                Ok(0)
            } else {
                Err(io::Error::other(errors.join("\n")))
            };
        }
        Work::Git(request) => {
            let cancelled = writer.cancelled.clone();
            return git_detail::run(request, &cancelled, |bytes| writer.encoded(bytes));
        }
    };
    let directory = if let Some(path) = enter {
        let path = path.canonicalize()?;
        if !path.is_dir() {
            return Err(io::Error::other("Not a directory"));
        }
        fs::read_dir(&path)?;
        writer.send(Message::Started(Some(path.clone())))?;
        writer.header(format!("{}:", display_path(&path)))?;
        path
    } else {
        writer.send(Message::Started(None))?;
        initial
    };
    let (options, paths) = list_options(&arguments)?;
    let header = paths.len() > 1 || options.recursive;
    let mut scopes: Vec<PathBuf> = Vec::new();
    for (index, value) in paths.into_iter().enumerate() {
        writer.check()?;
        if index > 0 {
            writer.header(String::new())?;
        }
        let path = directory.join(expand_home(&value));
        let scope = if path.is_dir() {
            path.canonicalize()?
        } else {
            path.parent().unwrap_or(&directory).canonicalize()?
        };
        if scopes.len() < 8 && !scopes.iter().any(|s| scope.starts_with(s)) {
            scopes.retain(|s| !s.starts_with(&scope));
            scopes.push(scope);
        }
        writer.list(&path, &value, &options, header, 0)?;
    }
    // Preserve the legacy listing's final blank text line after later commands
    // are appended, including when copying or navigating retained output.
    writer.header(String::new())?;
    writer.flush()?;
    writer.send(Message::NamesReady)?;
    // Git is optional and collected only after names are already available.
    let deadline = Instant::now() + Duration::from_secs(2);
    for scope in scopes {
        writer.check()?;
        if let Some(snapshot) = git::load(&scope, &writer.cancelled, deadline) {
            writer.send(Message::Git(snapshot))?;
        }
    }
    Ok(0)
}

struct LsOptions {
    all: bool,
    long: bool,
    human: bool,
    recursive: bool,
    one: bool,
}

fn list_options(arguments: &str) -> io::Result<(LsOptions, Vec<String>)> {
    let mut options = LsOptions {
        all: true,
        long: false,
        human: false,
        recursive: false,
        one: false,
    };
    let mut paths = Vec::new();
    let mut flags = true;
    for argument in words(arguments)? {
        if flags && argument == "--" {
            flags = false;
            continue;
        }
        if flags && argument.starts_with("--") {
            match argument.as_str() {
                "--all" => options.all = true,
                "--hide-hidden" => options.all = false,
                "--long" => options.long = true,
                "--human-readable" => options.human = true,
                "--recursive" => options.recursive = true,
                "--one-per-line" => options.one = true,
                _ => {
                    return Err(io::Error::other(format!(
                        "Unsupported ls option: {argument}"
                    )));
                }
            }
        } else if flags && argument.starts_with('-') && argument != "-" {
            for ch in argument[1..].chars() {
                match ch {
                    'a' => options.all = true,
                    'l' => options.long = true,
                    'h' => options.human = true,
                    'R' => options.recursive = true,
                    '1' => options.one = true,
                    _ => return Err(io::Error::other(format!("Unsupported ls option: -{ch}"))),
                }
            }
        } else {
            paths.push(argument);
        }
    }
    if paths.is_empty() {
        paths.push(".".into());
    }
    Ok((options, paths))
}

fn touch_options(arguments: &str, cwd: &Path) -> io::Result<(bool, Vec<PathBuf>)> {
    let mut no_create = false;
    let mut flags = true;
    let mut paths = Vec::new();
    for argument in words(arguments)? {
        match argument.as_str() {
            "--" if flags => flags = false,
            "-c" | "--no-create" if flags => no_create = true,
            option if flags && option.starts_with('-') && option != "-" => {
                return Err(io::Error::other(format!(
                    "Unsupported touch option: {option}"
                )));
            }
            "" => return Err(io::Error::other("A file path is required")),
            _ => paths.push(cwd.join(expand_home(&argument))),
        }
    }
    if paths.is_empty() {
        return Err(io::Error::other("Usage: touch [-c] [--] PATH..."));
    }
    Ok((no_create, paths))
}

pub(super) fn words(arguments: &str) -> io::Result<Vec<String>> {
    if arguments.len() > MAX_COMMAND_BYTES {
        return Err(io::Error::other("Command exceeds 64 KiB"));
    }
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = arguments.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && !cfg!(windows) && quote != Some('\'') {
            let Some(next) = chars.peek().copied() else {
                return Err(io::Error::other("Incomplete path escape"));
            };
            if quote.is_none() || matches!(next, '"' | '\\' | '$' | '`') {
                word.push(chars.next().unwrap());
                started = true;
                continue;
            }
        }
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            } else {
                word.push(ch);
            }
        } else if matches!(ch, '"' | '\'') {
            quote = Some(ch);
            started = true;
        } else if ch.is_whitespace() {
            if started {
                if words.len() == MAX_COMMAND_PATHS {
                    return Err(io::Error::other("Too many path arguments"));
                }
                words.push(std::mem::take(&mut word));
                started = false;
            }
        } else {
            word.push(ch);
            started = true;
        }
    }
    if quote.is_some() {
        return Err(io::Error::other("Close the quoted path"));
    }
    if started {
        if words.len() == MAX_COMMAND_PATHS {
            return Err(io::Error::other("Too many path arguments"));
        }
        words.push(word);
    }
    Ok(words)
}

fn single_path(value: &str) -> io::Result<String> {
    let value = value.trim();
    let words = words(value)?;
    match words.as_slice() {
        [] => Ok(String::new()),
        [path] => Ok(path.clone()),
        _ if !value.starts_with(['\'', '"']) => Ok(value.into()),
        _ => Err(io::Error::other("A single file path is required")),
    }
}

fn split_command(command: &str) -> (&str, &str) {
    let end = command.find(char::is_whitespace).unwrap_or(command.len());
    (&command[..end], command[end..].trim_start())
}

pub(super) fn shell_operators(command: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    for ch in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if (cfg!(windows) && ch == '^' && quote.is_none())
            || (!cfg!(windows) && ch == '\\' && quote != Some('\''))
        {
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if delimiter == '"'
                && ((!cfg!(windows) && matches!(ch, '$' | '`')) || (cfg!(windows) && ch == '%'))
            {
                return true;
            }
            if ch == delimiter {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' => quote = Some(ch),
            '\'' if !cfg!(windows) => quote = Some(ch),
            '|' | '&' | '<' | '>' | '\n' | '(' | ')' => return true,
            ';' if !cfg!(windows) => return true,
            // Expansion belongs to the shell, not literal Pötyi path handling.
            '$' | '`' | '*' | '?' if !cfg!(windows) => return true,
            '%' if cfg!(windows) => return true,
            _ => {}
        }
    }
    false
}

fn home() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn expand_home(value: &str) -> PathBuf {
    if value == "~" {
        return home().unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(rest) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
        && let Some(home) = home()
    {
        return home.join(rest);
    }
    PathBuf::from(value)
}

#[cfg(windows)]
fn drive_root(value: &str) -> Option<PathBuf> {
    let bytes = value.as_bytes();
    (bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        .then(|| PathBuf::from(format!("{}:\\", bytes[0].to_ascii_uppercase() as char)))
}

fn located_path(value: &str, cwd: &Path) -> (PathBuf, Option<usize>, Option<usize>) {
    let full = cwd.join(expand_home(value));
    // Literal names such as notes:2 take precedence over location syntax.
    if full.exists() {
        return (full, None, None);
    }
    if let Some((before, last)) = value.rsplit_once(':')
        && let Ok(number) = last.parse::<usize>()
    {
        if let Some((path, line)) = before.rsplit_once(':')
            && let Ok(line) = line.parse::<usize>()
        {
            return (cwd.join(expand_home(path)), Some(line), Some(number));
        }
        return (cwd.join(expand_home(before)), Some(number), None);
    }
    (full, None, None)
}

fn executable(path: &Path, metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = path;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        let _ = metadata;
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(extension))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, metadata);
        false
    }
}

fn command_on_path(name: &str, cwd: &Path) -> bool {
    let Some(search) = env::var_os("PATH") else {
        return false;
    };
    let mut names = vec![name.to_string()];
    #[cfg(windows)]
    names.extend(
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .take(32)
            .filter(|e| !e.is_empty())
            .map(|e| format!("{name}{e}")),
    );
    #[cfg(not(windows))]
    let _ = &mut names;
    env::split_paths(&search).any(|directory| {
        names.iter().any(|name| {
            let path = cwd.join(&directory).join(name);
            fs::metadata(&path).is_ok_and(|m| m.is_file() && executable(&path, &m))
        })
    })
}

fn classify(path: &Path, metadata: &fs::Metadata) -> Kind {
    if metadata.is_dir() {
        return Kind::Directory;
    }
    if !metadata.is_file() {
        return Kind::Unreadable;
    }
    let mut sample = Vec::with_capacity(8192);
    if File::open(path)
        .and_then(|f| f.take(8192).read_to_end(&mut sample))
        .is_err()
    {
        return Kind::Unreadable;
    }
    let utf8 = match std::str::from_utf8(&sample) {
        Ok(_) => true,
        Err(e) => e.error_len().is_none() && sample.len() == 8192 && metadata.len() > 8192,
    };
    if utf8
        && !sample
            .iter()
            .any(|b| *b < 32 && !matches!(*b, b'\t' | b'\n' | b'\r' | 12))
    {
        Kind::Text
    } else {
        Kind::Binary
    }
}

fn human_size(size: u64) -> String {
    let mut value = size as f64;
    let mut unit = 0;
    let units = ['B', 'K', 'M', 'G', 'T'];
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{size}B")
    } else {
        format!("{value:.1}{}", units[unit])
    }
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
            return format!("\\\\{rest}");
        }
        if let Some(rest) = value.strip_prefix("\\\\?\\") {
            return rest.into();
        }
    }
    value.into_owned()
}

fn native_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect()
    }
    #[cfg(not(any(unix, windows)))]
    {
        path.to_string_lossy().as_bytes().to_vec()
    }
}

fn native_path(bytes: &[u8]) -> io::Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(bytes.to_vec()).into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        if bytes.len() % 2 != 0 {
            return Err(io::Error::other("Invalid browser path record"));
        }
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        Ok(std::ffi::OsString::from_wide(&words).into())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(PathBuf::from(
            std::str::from_utf8(bytes).map_err(io::Error::other)?,
        ))
    }
}

fn encode(row: &Row) -> io::Result<Vec<u8>> {
    let path = row.path.as_deref().map(native_bytes).unwrap_or_default();
    let size = 16 + row.text.len() + path.len();
    if size > MAX_ROW_BYTES {
        return Err(io::Error::other("Browser row exceeds 64 KiB"));
    }
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(&(row.text.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(path.len() as u32).to_le_bytes());
    bytes.push(match row.kind {
        Kind::Text => 0,
        Kind::Binary => 1,
        Kind::Directory => 2,
        Kind::Unreadable => 3,
        Kind::Header => 4,
    });
    bytes.push(u8::from(row.path.is_some()));
    bytes.push(u8::from(row.color.is_some()));
    let (r, g, b) = row.color.unwrap_or_default();
    bytes.extend_from_slice(&[r, g, b, 0, 0]);
    bytes.extend_from_slice(row.text.as_bytes());
    bytes.extend_from_slice(&path);
    Ok(bytes)
}

/// The durable visible filename region, independent of column packing.
/// Long metadata uses a persisted bit so short names such as "d 42 hello"
/// never get mistaken for a size prefix. No filesystem access is performed.
pub(crate) fn native_name_range(bytes: &[u8]) -> io::Result<Option<std::ops::Range<usize>>> {
    let row = decode_native(bytes)?;
    if !matches!(row.kind, Kind::Text | Kind::Directory) {
        return Ok(None);
    }
    native_entry_range_for(bytes, &row)
}

/// Color belongs to the filename only, including non-clickable binary names.
/// Size/type prefixes and inter-column padding keep the normal foreground.
pub(super) fn native_entry_range(bytes: &[u8]) -> io::Result<Option<std::ops::Range<usize>>> {
    let row = decode_native(bytes)?;
    native_entry_range_for(bytes, &row)
}
fn native_entry_range_for(bytes: &[u8], row: &Row) -> io::Result<Option<std::ops::Range<usize>>> {
    if row.path.is_none() {
        return Ok(None);
    }
    let start = if bytes[14] & 2 != 0 {
        let prefix = row
            .text
            .get(2..)
            .ok_or_else(|| io::Error::other("Invalid long listing prefix"))?;
        let size_start = row.text.len() - prefix.trim_start().len();
        let size_end = row.text[size_start..]
            .find(' ')
            .map(|offset| size_start + offset)
            .ok_or_else(|| io::Error::other("Invalid long listing size"))?;
        size_end + 1
    } else {
        0
    };
    Ok((start < row.text.len()).then_some(start..row.text.len()))
}

/// Persisted native padding includes a ninth bit for the original wide-pane
/// limit. Dynamic preview packing retains its separate one-byte cell width.
pub(crate) fn native_padding(bytes: &[u8]) -> usize {
    if bytes.len() < 16 || bytes[14] & 16 == 0 {
        return 0;
    }
    usize::from(bytes[15]) + if bytes[14] & 32 != 0 { 256 } else { 0 }
}

/// Fixed per-directory packing metadata survives command-header eviction.
pub(crate) fn native_packing(bytes: &[u8]) -> Option<u16> {
    (bytes.len() >= 16 && bytes[14] & 16 == 0 && bytes[14] & 1 != 0 && bytes[15] != 0)
        .then(|| u16::from(bytes[15]))
}

pub(crate) fn decode_native(bytes: &[u8]) -> io::Result<Row> {
    if bytes.len() < 16 {
        return Err(io::Error::other("Invalid browser row record"));
    }
    let text_len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    let path_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if 16usize
        .checked_add(text_len)
        .and_then(|n| n.checked_add(path_len))
        != Some(bytes.len())
    {
        return Err(io::Error::other("Invalid browser row lengths"));
    }
    let kind = match bytes[8] {
        0 => Kind::Text,
        1 => Kind::Binary,
        2 => Kind::Directory,
        3 => Kind::Unreadable,
        4 => Kind::Header,
        _ => return Err(io::Error::other("Invalid browser row kind")),
    };
    Ok(Row {
        text: std::str::from_utf8(&bytes[16..16 + text_len])
            .map_err(io::Error::other)?
            .into(),
        path: if bytes[9] != 0 {
            Some(native_path(&bytes[16 + text_len..])?)
        } else {
            None
        },
        kind,
        color: (bytes[10] != 0).then_some((bytes[11], bytes[12], bytes[13])),
    })
}

// The legacy collector has a test-only restricted visibility tied to the
// terminal module. This isolated copy preserves its production bounds and
// cancellation, without changing the original :term's source or public API.
mod git {
    // Pötyi - Lightweight text editor
    // Copyright (C) 2026 Attila Banko
    // SPDX-License-Identifier: GPL-3.0-or-later

    use std::collections::HashMap;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::time::{Duration, Instant};

    const MAX_OUTPUT: usize = 1024 * 1024;
    const MAX_PATHS: usize = 8192;
    const MAX_PATH_BYTES: usize = 1024 * 1024;

    // Ordered by precedence when several children decorate the same directory.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub(crate) enum Status {
        Ignored,
        Untracked,
        Added,
        Modified,
        Deleted,
        Conflict,
    }

    impl Status {
        pub(crate) fn color(self) -> (u8, u8, u8) {
            match self {
                Self::Ignored => (140, 145, 140),
                Self::Untracked => (120, 205, 220),
                Self::Added => (150, 220, 165),
                Self::Modified => (225, 185, 115),
                Self::Deleted => (230, 140, 140),
                Self::Conflict => (225, 140, 215),
            }
        }

        fn parse(xy: &[u8]) -> Option<Self> {
            match xy {
                b"!!" => Some(Self::Ignored),
                b"??" => Some(Self::Untracked),
                b"DD" | b"AU" | b"UD" | b"UA" | b"DU" | b"AA" | b"UU" => Some(Self::Conflict),
                _ if xy.contains(&b'D') => Some(Self::Deleted),
                _ if xy.contains(&b'M')
                    || xy.contains(&b'R')
                    || xy.contains(&b'C')
                    || xy.contains(&b'T') =>
                {
                    Some(Self::Modified)
                }
                _ if xy.contains(&b'A') => Some(Self::Added),
                _ => None,
            }
        }
    }

    pub(super) struct Snapshot {
        pub scope: PathBuf,
        paths: HashMap<PathBuf, Decoration>,
    }

    #[derive(Clone, Copy)]
    struct Decoration {
        status: Status,
        descendants: Option<Status>,
    }

    impl Snapshot {
        pub fn status(&self, path: &Path) -> Option<Status> {
            if !path.starts_with(&self.scope) {
                return None;
            }
            if let Some(status) = self.paths.get(path) {
                return Some(status.status);
            }
            // Git collapses untracked/ignored directories. Inherit those states
            // for recursive listings without asking Git to enumerate their contents.
            for parent in path.ancestors().skip(1) {
                if !parent.starts_with(&self.scope) {
                    break;
                }
                if let Some(status) = self.paths.get(parent).and_then(|value| value.descendants) {
                    return Some(status);
                }
            }
            None
        }
    }

    pub(super) fn load(
        scope: &Path,
        cancelled: &Arc<AtomicBool>,
        deadline: Instant,
    ) -> Option<Snapshot> {
        let mut command = git(scope);
        command.args(["rev-parse", "--show-toplevel"]);
        let root = output(command, cancelled, deadline, 32 * 1024)?;
        let root = root.strip_suffix(b"\n").unwrap_or(&root);
        #[cfg(windows)]
        let root = root.strip_suffix(b"\r").unwrap_or(root);
        let root = path_from_bytes(root)?.canonicalize().ok()?;
        let mut command = git(&root);
        command.args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=normal",
            "--ignored=matching",
            "--ignore-submodules=all",
            "--no-renames",
            "--",
        ]);
        let relative = scope.strip_prefix(&root).ok()?;
        command.arg(if relative.as_os_str().is_empty() {
            Path::new(".")
        } else {
            relative
        });
        let bytes = output(command, cancelled, deadline, MAX_OUTPUT)?;
        parse(&bytes, &root, scope)
    }

    fn git(cwd: &Path) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(cwd)
            .args(["--no-optional-locks", "-c", "core.fsmonitor=false"])
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_LITERAL_PATHSPECS", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command
    }

    // A capped pipe reader prevents an enormous status output from filling memory.
    // Only this listing worker waits. Cancellation/timeout kills and reaps Git;
    // neither shutdown nor the UI waits for it.
    fn output(
        mut command: Command,
        cancelled: &Arc<AtomicBool>,
        deadline: Instant,
        limit: usize,
    ) -> Option<Vec<u8>> {
        if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
            return None;
        }
        let mut child = command.spawn().ok()?;
        let stdout = child.stdout.take()?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let reader = std::thread::Builder::new()
            .name("potyi-git-status".into())
            .spawn(move || {
                let mut bytes = Vec::new();
                let result = stdout
                    .take(limit as u64 + 1)
                    .read_to_end(&mut bytes)
                    .ok()
                    .filter(|_| bytes.len() <= limit)
                    .map(|_| bytes);
                let _ = sender.send(result);
            });
        if reader.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        let mut bytes = None;
        let result = loop {
            if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
                break None;
            }
            if bytes.is_none() {
                match receiver.try_recv() {
                    Ok(Some(value)) => bytes = Some(value),
                    Ok(None) | Err(mpsc::TryRecvError::Disconnected) => break None,
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => break None,
                Ok(Some(_)) if bytes.is_some() => break bytes,
                Err(_) => break None,
                _ => {}
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if result.is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
        result
    }

    fn parse(bytes: &[u8], root: &Path, scope: &Path) -> Option<Snapshot> {
        let mut snapshot = Snapshot {
            scope: scope.into(),
            paths: HashMap::new(),
        };
        let mut path_bytes = 0;
        let mut records = bytes.split(|&b| b == 0);
        while let Some(record) = records.next() {
            if record.is_empty() {
                continue;
            }
            if record.len() < 4 || record[2] != b' ' {
                return None;
            }
            let status = Status::parse(&record[..2])?;
            let relative = path_from_bytes(&record[3..])?;
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return None;
            }
            let path = root.join(relative);
            let descendants = matches!(status, Status::Ignored | Status::Untracked)
                .then_some(status)
                .filter(|_| record.ends_with(b"/"));
            // Git can report a collapsed directory above a requested subdirectory.
            let path = if descendants.is_some() && scope.starts_with(&path) {
                scope.to_path_buf()
            } else {
                path
            };
            let mut current = path.as_path();
            while current.starts_with(scope) {
                if let Some(old) = snapshot.paths.get_mut(current) {
                    old.status = old.status.max(status);
                    if current == path {
                        old.descendants = descendants;
                    }
                } else {
                    path_bytes += current.as_os_str().len();
                    if snapshot.paths.len() >= MAX_PATHS || path_bytes > MAX_PATH_BYTES {
                        return None;
                    }
                    snapshot.paths.insert(
                        current.into(),
                        Decoration {
                            status,
                            descendants: if current == path { descendants } else { None },
                        },
                    );
                }
                // Ignored entries do not make their tracked parent look ignored.
                if status == Status::Ignored || current == scope {
                    break;
                }
                let Some(parent) = current.parent() else {
                    break;
                };
                current = parent;
            }
            // Accept rename records too, although collection disables rename detection.
            if record[..2].contains(&b'R') || record[..2].contains(&b'C') {
                records.next()?;
            }
        }
        Some(snapshot)
    }

    #[cfg(unix)]
    fn path_from_bytes(bytes: &[u8]) -> Option<PathBuf> {
        use std::os::unix::ffi::OsStrExt;
        Some(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
    }

    #[cfg(not(unix))]
    fn path_from_bytes(bytes: &[u8]) -> Option<PathBuf> {
        std::str::from_utf8(bytes).ok().map(PathBuf::from)
    }
}

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;

#[cfg(test)]
fn decode(bytes: &[u8]) -> io::Result<Row> {
    decode_native(bytes)
}
