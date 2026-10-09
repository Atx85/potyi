// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Saved Git output and reversible commit details. Repository context travels
//! with each disk row; no transcript-sized link map or second permanent store.
use super::{
    Wake,
    browser::{self, Browser},
    transcript::{Record, RecordData, SharedTranscript, Transcript},
};
use std::{
    io::{self, Read},
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread,
    time::Duration,
};

const MAX_COMMAND_BYTES: usize = 64 * 1024;
const MAX_CONTEXT_BYTES: usize = 48 * 1024;
const MAX_ROW_BYTES: usize = 64 * 1024;
const CHUNK_BYTES: usize = 16 * 1024;
const QUEUED_CHUNKS: usize = 8;
const CONTEXT_FLAG: u8 = 0x80;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Repository {
    cwd: PathBuf,
    program: String,
    arguments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Commit {
    pub(crate) hash: String,
    repo: Arc<Repository>,
}

impl Commit {
    pub(crate) fn directory(&self) -> &Path {
        &self.repo.cwd
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Request {
    repository: Arc<Repository>,
    subcommand: String,
    arguments: Vec<String>,
    label: String,
    links: bool,
}

impl Request {
    /// Unsupported commands/expressions retain the normal interactive path.
    /// These are the same global repository options understood by legacy :term.
    pub(crate) fn from_command(command: &str, cwd: &Path) -> Option<Self> {
        if command.len() > MAX_COMMAND_BYTES
            || !cwd.is_absolute()
            || browser::shell_operators(command)
        {
            return None;
        }
        let words = browser::words(command).ok()?;
        let program = words.first()?;
        if !matches!(Path::new(program).file_name()?.to_str()?, "git" | "git.exe") {
            return None;
        }
        let mut index = 1;
        while let Some(word) = words.get(index) {
            if matches!(word.as_str(), "log" | "diff" | "status" | "show") {
                let arguments = words[index + 1..].to_vec();
                // These options run external producers or return a non-text
                // stream; retain their normal PTY handling rather than changing
                // their meaning in this structured view.
                if arguments.iter().any(|argument| {
                    matches!(argument.as_str(), "--ext-diff" | "--textconv" | "-z")
                        || argument.starts_with("--output=")
                        || argument == "--output"
                        || argument.starts_with("--porcelain=v2")
                }) {
                    return None;
                }
                let repository = Arc::new(Repository {
                    cwd: cwd.into(),
                    program: program.clone(),
                    arguments: words[1..index].to_vec(),
                });
                encode_context(&repository).ok()?;
                return Some(Self {
                    repository,
                    subcommand: word.clone(),
                    arguments,
                    label: command.trim().into(),
                    links: word == "log",
                });
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

    pub(crate) fn for_commit(commit: &Commit) -> io::Result<Self> {
        if !(4..=64).contains(&commit.hash.len())
            || !commit.hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid commit hash",
            ));
        }
        encode_context(&commit.repo)?;
        Ok(Self {
            repository: commit.repo.clone(),
            subcommand: "show".into(),
            arguments: vec![
                "--format=fuller".into(),
                "--stat".into(),
                "--patch".into(),
                "--root".into(),
                "--first-parent".into(),
                "--submodule=short".into(),
                format!("{}^{{commit}}", commit.hash),
                "--".into(),
            ],
            label: format!(
                "Commit {} · Back / Alt+Left returns to the log",
                commit.hash
            ),
            links: false,
        })
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.repository.program);
        command
            .current_dir(&self.repository.cwd)
            .args(&self.repository.arguments)
            .args(["--no-pager", &self.subcommand]);
        let delimiter = self
            .arguments
            .iter()
            .position(|argument| argument == "--")
            .unwrap_or(self.arguments.len());
        command.args(&self.arguments[..delimiter]);
        // Last options before the path delimiter win even over explicit color
        // choices; Pötyi persists its own bounded row colors instead of ANSI.
        if matches!(self.subcommand.as_str(), "log" | "diff" | "show") {
            command.args(["--no-ext-diff", "--no-textconv", "--color=never"]);
        }
        command.args(&self.arguments[delimiter..]);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.env("GIT_TERMINAL_PROMPT", "0");
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command
    }
}

pub(crate) struct GitDetail {
    browser: Browser,
    store: SharedTranscript,
}

impl GitDetail {
    pub(crate) fn open(commit: Commit, wake: Wake) -> io::Result<Self> {
        let request = Request::for_commit(&commit)?;
        let store = Transcript::shared()?;
        let mut browser =
            Browser::open_empty_with_transcript(commit.directory(), wake, store.clone())?;
        browser.execute_git_request(request)?;
        Ok(Self { browser, store })
    }

    /// The pane move-saves its base state, then installs these temporary owners.
    /// Dropping/replacing the Browser cancels its receiver before dropping disk.
    pub(crate) fn into_parts(self) -> (Browser, SharedTranscript) {
        (self.browser, self.store)
    }

    pub(crate) fn commit_at(record: &Record, byte: usize) -> io::Result<Option<Commit>> {
        let RecordData::Native(bytes) = &record.data else {
            return Ok(None);
        };
        if bytes.len() < 16 || bytes[14] & CONTEXT_FLAG == 0 || bytes[9] != 0 {
            return Ok(None);
        }
        let row = browser::decode_native(bytes)?;
        let Some(range) = hash_range(&row.text) else {
            return Ok(None);
        };
        if !range.contains(&byte) {
            return Ok(None);
        }
        let repository = decode_context(&bytes[16 + row.text.len()..], &record.cwd)?;
        Ok(Some(Commit {
            hash: row.text[range].into(),
            repo: Arc::new(repository),
        }))
    }
}

pub(crate) fn hash_range(line: &str) -> Option<Range<usize>> {
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
    let start = prefix
        + if line[prefix..].starts_with("commit ") {
            7
        } else {
            0
        };
    let length = line.as_bytes()[start..]
        .iter()
        .take_while(|byte| byte.is_ascii_hexdigit())
        .count();
    if !(4..=64).contains(&length)
        || line
            .as_bytes()
            .get(start + length)
            .is_some_and(|byte| !byte.is_ascii_whitespace())
    {
        return None;
    }
    Some(start..start + length)
}

fn encode_context(repository: &Repository) -> io::Result<Vec<u8>> {
    if repository.arguments.len() > 256 {
        return Err(io::Error::other("Too many Git repository options"));
    }
    let size = 4
        + 4
        + repository.program.len()
        + repository
            .arguments
            .iter()
            .map(|argument| 4 + argument.len())
            .sum::<usize>();
    if size > MAX_CONTEXT_BYTES {
        return Err(io::Error::other("Git repository metadata exceeds 48 KiB"));
    }
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(&[1, 0]);
    bytes.extend_from_slice(&(repository.arguments.len() as u16).to_le_bytes());
    for value in std::iter::once(&repository.program).chain(repository.arguments.iter()) {
        bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    Ok(bytes)
}

fn decode_context(mut bytes: &[u8], cwd: &Path) -> io::Result<Repository> {
    if bytes.len() < 4 || bytes.len() > MAX_CONTEXT_BYTES || bytes[0] != 1 || !cwd.is_absolute() {
        return Err(io::Error::other("Invalid saved Git repository context"));
    }
    let count = usize::from(u16::from_le_bytes([bytes[2], bytes[3]]));
    if count > 256 {
        return Err(io::Error::other("Invalid saved Git options"));
    }
    bytes = &bytes[4..];
    let mut strings = Vec::with_capacity(count + 1);
    for _ in 0..=count {
        if bytes.len() < 4 {
            return Err(io::Error::other("Truncated saved Git context"));
        }
        let len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        bytes = &bytes[4..];
        let value = bytes
            .get(..len)
            .ok_or_else(|| io::Error::other("Truncated saved Git option"))?;
        strings.push(
            std::str::from_utf8(value)
                .map_err(io::Error::other)?
                .to_owned(),
        );
        bytes = &bytes[len..];
    }
    if !bytes.is_empty() {
        return Err(io::Error::other("Invalid saved Git context length"));
    }
    let program = strings.remove(0);
    if !matches!(
        Path::new(&program)
            .file_name()
            .and_then(|name| name.to_str()),
        Some("git" | "git.exe")
    ) {
        return Err(io::Error::other("Invalid saved Git program"));
    }
    Ok(Repository {
        cwd: cwd.into(),
        program,
        arguments: strings,
    })
}

fn encode_row(text: &str, color: Option<(u8, u8, u8)>, context: &[u8]) -> io::Result<Vec<u8>> {
    let size = 16 + text.len() + context.len();
    if size > MAX_ROW_BYTES {
        return Err(io::Error::other(
            "Git output row exceeds 64 KiB including metadata",
        ));
    }
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(context.len() as u32).to_le_bytes());
    let (r, g, b) = color.unwrap_or_default();
    bytes.extend_from_slice(&[
        4,
        0,
        u8::from(color.is_some()),
        r,
        g,
        b,
        if context.is_empty() { 0 } else { CONTEXT_FLAG },
        0,
    ]);
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(context);
    Ok(bytes)
}

#[cfg(unix)]
struct Process(std::process::Child);
#[cfg(windows)]
struct Process(super::windows_command::Child);
impl Process {
    fn spawn(command: &mut Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            command.spawn().map(Self)
        }
        #[cfg(windows)]
        {
            super::windows_command::spawn_process(command).map(Self)
        }
    }
    fn try_wait(&mut self) -> io::Result<Option<i32>> {
        #[cfg(unix)]
        {
            self.0
                .try_wait()
                .map(|status| status.map(|status| status.code().unwrap_or(1)))
        }
        #[cfg(windows)]
        {
            self.0.try_wait()
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            // Git hooks/producers inherit this dedicated group. Reader threads
            // are never joined on the UI thread, including a blocked pipe.
            libc::kill(-(self.0.id() as i32), libc::SIGKILL);
        }
        let _ = self.0.kill();
        #[cfg(unix)]
        let _ = self.0.wait();
    }
}

enum ReadEvent {
    Bytes(usize, Vec<u8>),
    End(usize),
    Error(io::Error),
}
fn reader(
    mut pipe: impl Read + Send + 'static,
    stream: usize,
    sender: SyncSender<ReadEvent>,
) -> io::Result<()> {
    thread::Builder::new()
        .name("potyi-git-reader".into())
        .spawn(move || {
            let mut buffer = [0; CHUNK_BYTES];
            loop {
                match pipe.read(&mut buffer) {
                    Ok(0) => {
                        let _ = sender.send(ReadEvent::End(stream));
                        break;
                    }
                    Ok(count) => {
                        if sender
                            .send(ReadEvent::Bytes(stream, buffer[..count].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        let _ = sender.send(ReadEvent::Error(error));
                        break;
                    }
                }
            }
        })?;
    Ok(())
}

/// The Browser's bounded receiver remains the sole writer of the shared disk
/// store. A cancelled worker therefore cannot append to a restored base view.
pub(super) fn run(
    request: Request,
    cancelled: &AtomicBool,
    mut emit: impl FnMut(Vec<u8>) -> io::Result<()>,
) -> io::Result<i32> {
    if cancelled.load(Ordering::Acquire) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let context = if request.links {
        encode_context(&request.repository)?
    } else {
        Vec::new()
    };
    let mut process = Process::spawn(&mut request.command())?;
    let stdout = process
        .0
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Git stdout unavailable"))?;
    let stderr = process
        .0
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("Git stderr unavailable"))?;
    let (sender, receiver) = mpsc::sync_channel(QUEUED_CHUNKS);
    reader(stdout, 0, sender.clone())?;
    reader(stderr, 1, sender)?;
    let mut ended = [false; 2];
    let mut lines = [
        Lines::new(MAX_ROW_BYTES - 16 - context.len()),
        Lines::new(MAX_ROW_BYTES - 16 - context.len()),
    ];
    let mut colors = [Colors::default(), Colors::default()];
    let mut status = None;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(ReadEvent::Bytes(stream, bytes)) => lines[stream].push(&bytes, |text| {
                let color = colors[stream].line(text);
                emit(encode_row(text, color, &context)?)
            })?,
            Ok(ReadEvent::End(stream)) => {
                lines[stream]
                    .finish(|text| emit(encode_row(text, colors[stream].line(text), &context)?))?;
                ended[stream] = true;
            }
            Ok(ReadEvent::Error(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(mpsc::RecvTimeoutError::Disconnected) if !ended.iter().all(|end| *end) => {
                return Err(io::Error::other("Git reader ended unexpectedly"));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Once both readers close there are no more events to wait on.
                if status.is_none() {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
        if status.is_none() {
            status = process.try_wait()?;
        }
        if ended.iter().all(|end| *end) {
            if let Some(status) = status {
                return Ok(status);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Escape {
    None,
    Start,
    Csi,
    Osc,
    OscEnd,
}
struct Lines {
    bytes: Vec<u8>,
    escape: Escape,
    limit: usize,
}
impl Lines {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            escape: Escape::None,
            limit,
        }
    }
    fn push(
        &mut self,
        bytes: &[u8],
        mut emit: impl FnMut(&str) -> io::Result<()>,
    ) -> io::Result<()> {
        for &byte in bytes {
            match self.escape {
                Escape::Start => {
                    self.escape = match byte {
                        b'[' => Escape::Csi,
                        b']' => Escape::Osc,
                        _ => Escape::None,
                    };
                    continue;
                }
                Escape::Csi => {
                    if (0x40..=0x7e).contains(&byte) {
                        self.escape = Escape::None;
                    }
                    continue;
                }
                Escape::Osc => {
                    if byte == 7 {
                        self.escape = Escape::None;
                    } else if byte == 0x1b {
                        self.escape = Escape::OscEnd;
                    }
                    continue;
                }
                Escape::OscEnd => {
                    self.escape = if byte == b'\\' {
                        Escape::None
                    } else {
                        Escape::Osc
                    };
                    continue;
                }
                Escape::None => (),
            }
            match byte {
                0x1b => self.escape = Escape::Start,
                b'\n' => {
                    emit(&String::from_utf8_lossy(&self.bytes))?;
                    self.bytes.clear();
                }
                b'\t' | 0x20..=0x7e | 0x80..=0xff => {
                    if self.bytes.len() == self.limit {
                        return Err(io::Error::other(
                            "Git output row exceeds its 64 KiB storage limit",
                        ));
                    }
                    self.bytes.push(byte);
                }
                _ => (),
            }
        }
        Ok(())
    }
    fn finish(&mut self, mut emit: impl FnMut(&str) -> io::Result<()>) -> io::Result<()> {
        if !self.bytes.is_empty() {
            emit(&String::from_utf8_lossy(&self.bytes))?;
            self.bytes.clear();
        }
        Ok(())
    }
}

#[derive(Default)]
struct Colors {
    section: Option<super::output_colors::Tone>,
}
impl Colors {
    fn line(&mut self, line: &str) -> Option<(u8, u8, u8)> {
        super::output_colors::git_line(line, &mut self.section)
    }
}

#[cfg(test)]
#[path = "git_detail_tests.rs"]
mod tests;
