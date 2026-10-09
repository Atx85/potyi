// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Portable PTY session, live grid, and file-backed scrollback.
use super::{
    history::History,
    lifecycle::{Completion, Lifecycle, Shell},
    transcript::SharedTranscript,
};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
#[path = "windows_command.rs"]
mod windows_command;

pub(super) const MAX_ROWS: u16 = 160;
pub(super) const MAX_COLS: u16 = 256;
pub(super) const HISTORY_ROWS: usize = 0;
pub(super) const MAX_PASTE_BYTES: usize = 64 * 1024;
const CHUNK: usize = 16 * 1024;
const OUTPUT_CHUNKS: usize = 32;
const PARSE_SLICE: usize = 1024;
const INPUT_CHUNKS: usize = 8;
pub(crate) type Wake = Arc<dyn Fn() + Send + Sync>;

#[cfg(unix)]
#[derive(Clone)]
struct ManagedRestart {
    shell: std::ffi::OsString,
    directory: PathBuf,
    environment: Vec<(String, String)>,
    wake: Wake,
}

enum Message {
    Output(Vec<u8>),
    Closed,
    Exited(String),
    Error(String),
    #[cfg(windows)]
    PipeClosed(u8),
    #[cfg(windows)]
    PipeExited(i32),
}

#[cfg(windows)]
struct WindowsState {
    shell: std::ffi::OsString,
    environment: Vec<(String, String)>,
    wake: Wake,
    directory: Option<PathBuf>,
    streams: u8,
    exit_status: Option<i32>,
    stopped: bool,
}

#[cfg(any(windows, test))]
#[derive(Default)]
struct PipeText {
    carry: Vec<u8>,
    previous_cr: bool,
}
#[cfg(any(windows, test))]
impl PipeText {
    fn feed(&mut self, input: &[u8], eof: bool) -> Vec<u8> {
        let mut bytes = std::mem::take(&mut self.carry);
        bytes.extend_from_slice(input);
        let mut complete = 0;
        if eof {
            complete = bytes.len();
        } else {
            while complete < bytes.len() {
                match std::str::from_utf8(&bytes[complete..]) {
                    Ok(_) => {
                        complete = bytes.len();
                        break;
                    }
                    Err(error) => {
                        complete += error.valid_up_to();
                        if let Some(invalid) = error.error_len() {
                            complete += invalid;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        // A stream's unfinished UTF-8 character must not be interleaved with
        // another producer. At most three bytes wait for the next read.
        self.carry.extend_from_slice(&bytes[complete..]);
        debug_assert!(self.carry.len() <= 3);
        let text = String::from_utf8_lossy(&bytes[..complete]);
        let mut output = Vec::with_capacity(text.len().saturating_mul(2));
        for byte in text.as_bytes() {
            // Ordinary pipes do not get the PTY's output newline translation.
            if *byte == b'\n' && !self.previous_cr {
                output.push(b'\r');
            }
            output.push(*byte);
            self.previous_cr = *byte == b'\r';
        }
        output
    }
}

#[cfg(windows)]
fn publish_pipe_text(
    sender: &SyncSender<Message>,
    bytes: Vec<u8>,
    pending: &AtomicBool,
    wake: &Wake,
) -> bool {
    let text = std::str::from_utf8(&bytes).expect("PipeText emits UTF-8");
    let mut start = 0;
    while start < bytes.len() {
        let mut end = (start + CHUNK).min(bytes.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if !publish(
            sender,
            Message::Output(bytes[start..end].to_vec()),
            pending,
            wake,
        ) {
            return false;
        }
        start = end;
    }
    true
}

#[derive(Clone, Debug)]
struct Origin {
    relative_safe: bool,
    source: u64,
    directory: PathBuf,
}

struct Capture {
    transcript: SharedTranscript,
    active: Option<Origin>,
    error: Option<String>,
    failed: bool,
}

impl std::fmt::Debug for Capture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Capture")
            .field("active", &self.active)
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}

impl vt100::ScrollbackSink for Capture {
    fn push(&mut self, cols: u16, wrapped: bool, formatted: &[u8]) {
        let Some(origin) = &self.active else {
            return;
        };
        if self.failed {
            return;
        }
        if let Err(error) = self.transcript.lock().unwrap().append_terminal(
            &origin.directory,
            origin.source,
            cols,
            wrapped,
            formatted,
        ) {
            self.failed = true;
            self.error = Some(format!("Transcript write failed: {error}"));
        }
    }

    // ED3 and RIS belong to the active terminal surface. They must never
    // erase native rows or another command's committed transcript records.
    fn clear(&mut self) {}
}

struct Seal {
    screen: vt100::Screen,
    origin: Origin,
    next: u16,
    rows: u16,
}

#[derive(Default)]
struct Replies {
    pending: Vec<Vec<u8>>,
    ready: bool,
    capture: Option<Arc<Mutex<Capture>>>,
    expected_end: Option<(String, u64)>,
    end_seen: bool,
    end_frame: Option<vt100::Screen>,
}

impl vt100::Callbacks for Replies {
    fn unhandled_osc(&mut self, screen: &mut vt100::Screen, params: &[&[u8]]) {
        if params == [b"777".as_slice(), b"potyi-ready".as_slice()] {
            self.ready = true;
            return;
        }
        let [b"777", b"potyi-end", nonce, id] = params else {
            return;
        };
        let Some((expected_nonce, expected_id)) = &self.expected_end else {
            return;
        };
        if self.end_seen || *nonce != expected_nonce.as_bytes() || id.len() > 20 {
            return;
        }
        let Ok(id) = std::str::from_utf8(id).unwrap_or("").parse::<u64>() else {
            return;
        };
        if id != *expected_id {
            return;
        }
        self.end_seen = true;
        if let Some(capture) = &self.capture {
            // Snapshot at the exact marker, before any trailing bytes in the
            // same parser slice can mutate the completed command's tail.
            self.end_frame = Some(screen.normal_screen_clone());
            capture.lock().unwrap().active = None;
        }
    }

    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        prefix: Option<u8>,
        intermediate: Option<u8>,
        params: &[&[u16]],
        command: char,
    ) {
        if intermediate.is_some() || self.pending.len() >= 64 {
            return;
        }
        let first = params.first().and_then(|p| p.first()).copied().unwrap_or(0);
        let reply = match (prefix, command, first) {
            (None, 'n', 5) => Some(b"\x1b[0n".to_vec()),
            (None | Some(b'?'), 'n', 6) => {
                let (row, col) = screen.cursor_position();
                Some(
                    format!(
                        "\x1b[{}{};{}R",
                        if prefix.is_some() { "?" } else { "" },
                        row + 1,
                        col + 1
                    )
                    .into_bytes(),
                )
            }
            (None, 'c', 0) => Some(b"\x1b[?1;2c".to_vec()),
            (Some(b'>'), 'c', 0) => Some(b"\x1b[>0;0;0c".to_vec()),
            (None, 't', 18) => {
                let (rows, cols) = screen.size();
                Some(format!("\x1b[8;{rows};{cols}t").into_bytes())
            }
            _ => None,
        };
        if let Some(reply) = reply {
            self.pending.push(reply);
        }
    }
}

pub(super) struct Session {
    #[cfg(unix)]
    managed_restart: Option<ManagedRestart>,
    #[cfg(unix)]
    shell_pid: Option<libc::pid_t>,
    #[cfg(unix)]
    stop_requested: bool,
    #[cfg(unix)]
    completion_after_stop: Option<Completion>,
    #[cfg(unix)]
    restart_receiver: Option<Receiver<io::Result<Session>>>,
    #[cfg(unix)]
    restarting: Option<Box<Session>>,
    lifecycle: Option<Lifecycle>,
    parser: vt100::Parser<Replies>,
    history: Option<Arc<Mutex<History>>>,
    transcript: Option<SharedTranscript>,
    capture: Option<Arc<Mutex<Capture>>>,
    origin: Option<Origin>,
    plain: Option<super::literal_output::Capture>,
    plain_failed: bool,
    plain_marker: Vec<u8>,
    plain_marker_position: usize,
    last_result: Option<super::transcript::Anchor>,
    seal: Option<Seal>,
    tail_sealed: bool,
    eof_seal: bool,
    history_view: Option<vt100::Parser>,
    history_offset: usize,
    history_serial: u64,
    master: Option<Box<dyn MasterPty + Send>>,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
    #[cfg(windows)]
    windows: Option<WindowsState>,
    input: Option<SyncSender<Vec<u8>>>,
    output: Option<Receiver<Message>>,
    pending_output: Option<(Vec<u8>, usize)>,
    wake_pending: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
    pub(super) status: Option<String>,
    pub(super) closed: bool,
    pub(super) work_pending: bool,
}

impl Session {
    #[cfg(test)]
    pub(super) fn open(directory: &Path, rows: u16, cols: u16, wake: Wake) -> io::Result<Self> {
        let shell = CommandBuilder::new_default_prog().get_shell();
        #[cfg(unix)]
        let shell_name = Path::new(&shell)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("sh");
        let mut command = CommandBuilder::new(&shell);
        #[cfg(unix)]
        match shell_name {
            "zsh" => {
                command.args(["-f", "-i"]);
            }
            "bash" => {
                command.args(["--noprofile", "--norc", "-i"]);
            }
            "fish" => {
                command.args(["--no-config", "-i"]);
            }
            _ => {
                command.arg("-i");
            }
        }
        command.env_remove("ENV");
        command.env_remove("BASH_ENV");
        #[cfg(windows)]
        command.args(["/D", "/Q"]);
        Self::spawn(directory, command, rows, cols, wake)
    }
    pub(super) fn open_with_environment(
        directory: &Path,
        rows: u16,
        cols: u16,
        wake: Wake,
        environment: &[(&str, String)],
    ) -> io::Result<Self> {
        Self::open_managed(directory, rows, cols, wake, environment, None)
    }

    pub(super) fn open_with_transcript(
        directory: &Path,
        rows: u16,
        cols: u16,
        wake: Wake,
        environment: &[(&str, String)],
        transcript: SharedTranscript,
    ) -> io::Result<Self> {
        Self::open_managed(directory, rows, cols, wake, environment, Some(transcript))
    }

    fn open_managed(
        directory: &Path,
        rows: u16,
        cols: u16,
        wake: Wake,
        environment: &[(&str, String)],
        transcript: Option<SharedTranscript>,
    ) -> io::Result<Self> {
        let shell = CommandBuilder::new_default_prog().get_shell();
        Self::open_managed_shell(
            shell.into(),
            directory,
            rows,
            cols,
            wake,
            environment,
            transcript,
        )
    }

    fn open_managed_shell(
        shell: std::ffi::OsString,
        directory: &Path,
        rows: u16,
        cols: u16,
        wake: Wake,
        environment: &[(&str, String)],
        transcript: Option<SharedTranscript>,
    ) -> io::Result<Self> {
        let shell_name = Path::new(&shell)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        #[cfg(unix)]
        let known_shell = matches!(
            shell_name,
            "sh" | "bash" | "zsh" | "dash" | "ksh" | "mksh" | "ash" | "fish"
        );
        #[cfg(unix)]
        if !known_shell && transcript.is_none() {
            return Err(io::Error::other(format!(
                "Managed commands do not support the {shell_name} shell; use sh, bash, zsh, ksh, dash, ash or fish"
            )));
        }
        #[cfg(unix)]
        let kind = if shell_name == "fish" {
            Shell::Fish
        } else {
            Shell::Posix
        };
        #[cfg(windows)]
        if !shell_name.eq_ignore_ascii_case("cmd") {
            return Err(io::Error::other("Managed Windows commands require cmd.exe"));
        }
        #[cfg(windows)]
        let kind = Shell::Windows;
        #[cfg(unix)]
        let helpers = if known_shell {
            super::bridge::setup_helpers(shell_name)
        } else {
            String::new()
        };
        #[cfg(windows)]
        let helpers = super::bridge::setup_helpers("cmd");
        let lifecycle = Lifecycle::new(kind, &helpers, transcript.is_some())?;
        #[cfg(windows)]
        return Self::windows_idle(
            shell.into(),
            directory,
            rows,
            cols,
            wake,
            environment,
            transcript,
            lifecycle,
        );
        #[cfg(unix)]
        {
            // Any shell that accepts the original -c contract can execute
            // production commands. Unknown grammars use a stable sh driver
            // and receive their command text without POSIX helper syntax.
            let driver_shell = if known_shell {
                shell.clone()
            } else {
                "/bin/sh".into()
            };
            let mut command = CommandBuilder::new(&driver_shell);
            if transcript.is_some() {
                // The internal dispatcher has no user startup side effects.
                // Each production child uses normal noninteractive startup.
                match shell_name {
                    "zsh" => {
                        command.args(["-f", "-c"]);
                    }
                    "bash" => {
                        command.args(["--noprofile", "--norc", "-c"]);
                    }
                    "fish" => {
                        command.args(["--no-config", "-c"]);
                    }
                    _ => {
                        command.arg("-c");
                    }
                }
            } else {
                command.arg("-c");
            }
            // Production retains :term's noninteractive stdin contract. Raw
            // backend comparison sessions can still exercise the actual PTY.
            command.env(
                "POTYI_COMMAND_STDIN_NULL",
                if transcript.is_some() { "1" } else { "0" },
            );
            #[cfg(windows)]
            command.args(["/D", "/Q", "/V:OFF", "/C"]);
            command.arg(lifecycle.launch());
            for (name, value) in environment {
                command.env(name, value);
            }
            if transcript.is_some() {
                for (name, saved, present) in [
                    (
                        "BASH_ENV",
                        "POTYI_CHILD_BASH_ENV",
                        "POTYI_CHILD_BASH_ENV_SET",
                    ),
                    ("ENV", "POTYI_CHILD_ENV", "POTYI_CHILD_ENV_SET"),
                ] {
                    let value = environment
                        .iter()
                        .rev()
                        .find(|(key, _)| *key == name)
                        .map(|(_, value)| std::ffi::OsString::from(value))
                        .or_else(|| std::env::var_os(name));
                    if let Some(value) = value {
                        command.env(saved, value);
                        command.env(present, "1");
                    } else {
                        command.env_remove(saved);
                        command.env(present, "0");
                    }
                    command.env_remove(name);
                }
            }
            command.env("POTYI_DRIVER_DIRECTORY", lifecycle.directory());
            command.env("POTYI_DRIVER_SHELL", &driver_shell);
            if !known_shell {
                command.env("POTYI_COMMAND_SHELL", &shell);
            } else {
                command.env_remove("POTYI_COMMAND_SHELL");
            }
            command.env("POTYI_DRIVER_NONCE", lifecycle.nonce());
            #[cfg(windows)]
            {
                let paths = std::iter::once(lifecycle.directory().to_path_buf())
                    .chain(std::env::split_paths(
                        &std::env::var_os("PATH").unwrap_or_default(),
                    ))
                    .collect::<Vec<_>>();
                command.env(
                    "PATH",
                    std::env::join_paths(paths).map_err(io::Error::other)?,
                );
            }
            let config = ManagedRestart {
                shell: shell.clone(),
                directory: directory.canonicalize()?,
                environment: environment
                    .iter()
                    .map(|(name, value)| ((*name).into(), value.clone()))
                    .collect(),
                wake: wake.clone(),
            };
            let mut session =
                Self::spawn_with_transcript(directory, command, rows, cols, wake, transcript)?;
            session.lifecycle = Some(lifecycle);
            session.managed_restart = Some(config);
            Ok(session)
        }
    }

    #[cfg(windows)]
    fn windows_idle(
        shell: std::ffi::OsString,
        directory: &Path,
        rows: u16,
        cols: u16,
        wake: Wake,
        environment: &[(&str, String)],
        transcript: Option<SharedTranscript>,
        lifecycle: Lifecycle,
    ) -> io::Result<Self> {
        if !directory.is_dir() {
            return Err(io::Error::other("The command directory is unavailable"));
        }
        let history = if transcript.is_none() {
            Some(Arc::new(Mutex::new(History::new()?)))
        } else {
            None
        };
        let plain = transcript
            .as_ref()
            .map(|_| super::literal_output::Capture::default());
        let capture: Option<Arc<Mutex<Capture>>> = None;
        let mut parser = vt100::Parser::new_with_callbacks(
            rows.clamp(1, MAX_ROWS),
            cols.clamp(1, MAX_COLS),
            HISTORY_ROWS,
            Replies {
                ready: true,
                capture: capture.clone(),
                ..Replies::default()
            },
        );
        if let Some(capture) = &capture {
            parser.screen_mut().set_scrollback_sink(capture.clone());
        } else if let Some(history) = &history {
            parser.screen_mut().set_scrollback_sink(history.clone());
        }
        let mut variables: Vec<(String, String)> = environment
            .iter()
            .map(|(name, value)| ((*name).into(), value.clone()))
            .collect();
        variables.extend([
            (
                "POTYI_DRIVER_DIRECTORY".into(),
                lifecycle.directory().to_string_lossy().into_owned(),
            ),
            (
                "POTYI_DRIVER_SHELL".into(),
                shell.to_string_lossy().into_owned(),
            ),
            ("TERM".into(), "dumb".into()),
            ("NO_COLOR".into(), "1".into()),
            ("PAGER".into(), "cat".into()),
            ("GIT_PAGER".into(), "cat".into()),
        ]);
        let paths = std::iter::once(lifecycle.directory().to_path_buf())
            .chain(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ))
            .collect::<Vec<_>>();
        variables.push((
            "PATH".into(),
            std::env::join_paths(paths)
                .map_err(io::Error::other)?
                .to_string_lossy()
                .into_owned(),
        ));
        Ok(Self {
            lifecycle: Some(lifecycle),
            parser,
            history,
            transcript,
            capture,
            origin: None,
            plain,
            plain_failed: false,
            plain_marker: Vec::new(),
            plain_marker_position: 0,
            last_result: None,
            seal: None,
            tail_sealed: false,
            eof_seal: false,
            history_view: None,
            history_offset: 0,
            history_serial: 0,
            master: None,
            killer: None,
            windows: Some(WindowsState {
                shell,
                environment: variables,
                wake,
                directory: None,
                streams: 0,
                exit_status: None,
                stopped: false,
            }),
            input: None,
            output: None,
            pending_output: None,
            wake_pending: Arc::new(AtomicBool::new(false)),
            exited: Arc::new(AtomicBool::new(false)),
            status: None,
            closed: false,
            work_pending: false,
        })
    }

    fn spawn(
        directory: &Path,
        command: CommandBuilder,
        rows: u16,
        cols: u16,
        wake: Wake,
    ) -> io::Result<Self> {
        Self::spawn_with_transcript(directory, command, rows, cols, wake, None)
    }

    fn spawn_with_transcript(
        directory: &Path,
        mut command: CommandBuilder,
        rows: u16,
        cols: u16,
        wake: Wake,
        transcript: Option<SharedTranscript>,
    ) -> io::Result<Self> {
        let rows = rows.clamp(1, MAX_ROWS);
        let cols = cols.clamp(1, MAX_COLS);
        #[cfg(windows)]
        let cmd_directory = if command.get_argv().first().is_some_and(|program| {
            Path::new(program)
                .file_stem()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("cmd"))
        }) {
            Some(windows_command::cmd_directory(&directory.canonicalize()?)?)
        } else {
            None
        };
        #[cfg(windows)]
        command.cwd(cmd_directory.as_deref().unwrap_or(directory));
        #[cfg(not(windows))]
        command.cwd(directory);
        if transcript.is_some() {
            command.env("TERM", "dumb");
            command.env("NO_COLOR", "1");
            command.env("PAGER", "cat");
            command.env("GIT_PAGER", "cat");
            command.env_remove("COLORTERM");
        } else {
            command.env("TERM", "xterm-256color");
            command.env("COLORTERM", "truecolor");
            for name in ["NO_COLOR", "PAGER", "GIT_PAGER"] {
                command.env_remove(name);
            }
        }
        for name in ["COLUMNS", "LINES"] {
            command.env_remove(name);
        }
        // Create storage before spawning a shell so a file error cannot leave
        // an unowned process behind.
        let history = if transcript.is_none() {
            Some(Arc::new(Mutex::new(History::new()?)))
        } else {
            None
        };
        let plain = transcript
            .as_ref()
            .map(|_| super::literal_output::Capture::default());
        let capture: Option<Arc<Mutex<Capture>>> = None;
        let mut parser = vt100::Parser::new_with_callbacks(
            rows,
            cols,
            HISTORY_ROWS,
            Replies {
                capture: capture.clone(),
                ..Replies::default()
            },
        );
        if let Some(capture) = &capture {
            parser.screen_mut().set_scrollback_sink(capture.clone());
        } else if let Some(history) = &history {
            parser.screen_mut().set_scrollback_sink(history.clone());
        }
        let pair = native_pty_system()
            .openpty(size(rows, cols))
            .map_err(io::Error::other)?;
        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let mut writer = pair.master.take_writer().map_err(io::Error::other)?;
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(io::Error::other)?;
        drop(pair.slave); // Retaining the slave would prevent EOF detection.
        let killer = child.clone_killer();
        #[cfg(unix)]
        let shell_pid = child.process_id().and_then(|pid| i32::try_from(pid).ok());
        let (output_sender, output) = mpsc::sync_channel(OUTPUT_CHUNKS);
        let (input, input_receiver) = mpsc::sync_channel::<Vec<u8>>(INPUT_CHUNKS);
        let wake_pending = Arc::new(AtomicBool::new(false));
        let exited = Arc::new(AtomicBool::new(false));

        let send = output_sender.clone();
        let notify = wake.clone();
        let pending = wake_pending.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; CHUNK];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if !publish(
                            &send,
                            Message::Output(buffer[..count].to_vec()),
                            &pending,
                            &notify,
                        ) {
                            return;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    // Unix PTYs may report EIO instead of EOF when the slave closes.
                    #[cfg(unix)]
                    Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
                    Err(error) => {
                        publish(
                            &send,
                            Message::Error(format!("Terminal read failed: {error}")),
                            &pending,
                            &notify,
                        );
                        break;
                    }
                }
            }
            publish(&send, Message::Closed, &pending, &notify);
        });

        let send = output_sender.clone();
        let notify = wake.clone();
        let pending = wake_pending.clone();
        thread::spawn(move || {
            while let Ok(bytes) = input_receiver.recv() {
                if let Err(error) = writer.write_all(&bytes).and_then(|_| writer.flush()) {
                    publish(
                        &send,
                        Message::Error(format!("Terminal write failed: {error}")),
                        &pending,
                        &notify,
                    );
                    break;
                }
            }
        });

        let pending = wake_pending.clone();
        let done = exited.clone();
        thread::spawn(move || {
            let message = match child.wait() {
                Ok(status) => format!("Shell exited: {status}"),
                Err(error) => format!("Shell wait failed: {error}"),
            };
            done.store(true, Ordering::Release);
            publish(&output_sender, Message::Exited(message), &pending, &wake);
        });

        Ok(Self {
            #[cfg(unix)]
            managed_restart: None,
            #[cfg(unix)]
            shell_pid,
            #[cfg(unix)]
            stop_requested: false,
            #[cfg(unix)]
            completion_after_stop: None,
            #[cfg(unix)]
            restart_receiver: None,
            #[cfg(unix)]
            restarting: None,
            lifecycle: None,
            parser,
            history,
            transcript,
            capture,
            origin: None,
            plain,
            plain_failed: false,
            plain_marker: Vec::new(),
            plain_marker_position: 0,
            last_result: None,
            seal: None,
            tail_sealed: false,
            eof_seal: false,
            history_view: None,
            history_offset: 0,
            history_serial: 0,
            master: Some(pair.master),
            killer: Some(killer),
            #[cfg(windows)]
            windows: None,
            input: Some(input),
            output: Some(output),
            pending_output: None,
            wake_pending,
            exited,
            status: None,
            closed: false,
            work_pending: false,
        })
    }

    pub(super) fn ready(&self) -> bool {
        #[cfg(unix)]
        if self.stop_requested {
            return false;
        }
        self.parser.callbacks().ready
    }
    pub(super) fn running(&self) -> bool {
        #[cfg(unix)]
        if self.stop_requested {
            return true;
        }
        self.lifecycle.as_ref().is_some_and(Lifecycle::running)
    }
    pub(super) fn start_command(&mut self, command: &str, directory: &Path) -> io::Result<()> {
        if !self.ready() {
            return Err(io::Error::other("The managed terminal is starting"));
        }
        if self.closed || self.exited.load(Ordering::Acquire) {
            return Err(io::Error::other("The shell has exited"));
        }
        let launch_directory = directory.canonicalize()?;
        #[cfg(windows)]
        windows_command::cmd_directory(&launch_directory)?;
        let bytes = self
            .lifecycle
            .as_mut()
            .ok_or_else(|| io::Error::other("This is a raw terminal session"))?
            .prepare(command, &launch_directory)?;
        #[cfg(unix)]
        if let Some(config) = &mut self.managed_restart {
            config.directory = launch_directory.clone();
        }
        let id = self.lifecycle.as_ref().unwrap().current_id().unwrap();
        if let Some(transcript) = &self.transcript {
            let result = (|| -> io::Result<Origin> {
                let mut transcript = transcript.lock().unwrap();
                let source = transcript.next_source();
                let directory = launch_directory.clone();
                transcript.append_header(&directory, source, command)?;
                Ok(Origin {
                    source,
                    directory,
                    relative_safe: true,
                })
            })();
            let origin = match result {
                Ok(origin) => origin,
                Err(error) => {
                    self.lifecycle.as_mut().unwrap().cancel_start();
                    return Err(error);
                }
            };
            self.reset_managed_grid();
            self.origin = Some(origin.clone());
            if let Some(capture) = &self.capture {
                capture.lock().unwrap().active = Some(origin);
            }
            if let Some(plain) = &mut self.plain {
                plain.reset();
            }
        }
        self.tail_sealed = false;
        self.eof_seal = false;
        self.seal = None;
        let callbacks = self.parser.callbacks_mut();
        callbacks.expected_end = Some((self.lifecycle.as_ref().unwrap().nonce().into(), id));
        callbacks.end_seen = false;
        callbacks.end_frame = None;
        self.plain_marker = format!(
            "\x18\x1b]777;potyi-end;{};{id}\x07",
            self.lifecycle.as_ref().unwrap().nonce()
        )
        .into_bytes();
        self.plain_marker_position = 0;
        #[cfg(windows)]
        let start_result = if self.windows.is_some() {
            self.start_windows_command(command, &launch_directory)
        } else {
            self.send(bytes)
        };
        #[cfg(not(windows))]
        let start_result = self.send(bytes);
        if let Err(error) = start_result {
            self.lifecycle.as_mut().unwrap().cancel_start();
            if let Some(capture) = &self.capture {
                capture.lock().unwrap().active = None;
            }
            if let (Some(transcript), Some(origin)) = (&self.transcript, self.origin.take()) {
                let _ = transcript.lock().unwrap().append_result(
                    &origin.directory,
                    origin.source,
                    -1,
                    &format!("Command could not start: {error}"),
                );
            }
            self.parser.callbacks_mut().expected_end = None;
            return Err(error);
        }
        Ok(())
    }

    #[cfg(windows)]
    fn start_windows_command(&mut self, command: &str, directory: &Path) -> io::Result<()> {
        let launcher = self.lifecycle.as_ref().unwrap().windows_launcher()?;
        let state = self.windows.as_mut().unwrap();
        let directory = directory.canonicalize()?;
        let mut environment = state.environment.clone();
        environment.push(("POTYI_COMMAND_TEXT".into(), command.to_owned()));
        environment.push((
            "POTYI_COMMAND_DIRECTORY".into(),
            super::lifecycle::windows_directory(
                directory
                    .to_str()
                    .ok_or_else(|| io::Error::other("Command directory is not valid Unicode"))?,
            ),
        ));
        let mut child = windows_command::spawn(&state.shell, &launcher, &directory, &environment)?;
        self.killer = Some(child.clone_killer());
        let (sender, output) = mpsc::sync_channel(OUTPUT_CHUNKS);
        self.output = Some(output);
        self.pending_output = None;
        self.exited.store(false, Ordering::Release);
        state.directory = Some(directory);
        state.streams = 0;
        state.exit_status = None;
        state.stopped = false;
        self.history_offset = 0;
        self.history_view = None;
        self.status = None;
        self.parser.callbacks_mut().expected_end = None;
        for (mut reader, stream) in [
            (child.stdout.take().unwrap(), 1u8),
            (child.stderr.take().unwrap(), 2u8),
        ] {
            let sender = sender.clone();
            let pending = self.wake_pending.clone();
            let wake = state.wake.clone();
            thread::spawn(move || {
                let mut buffer = [0; CHUNK];
                let mut text = PipeText::default();
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => {
                            if !publish_pipe_text(
                                &sender,
                                text.feed(&buffer[..count], false),
                                &pending,
                                &wake,
                            ) {
                                return;
                            }
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(error) => {
                            publish(
                                &sender,
                                Message::Error(format!("Command output read failed: {error}")),
                                &pending,
                                &wake,
                            );
                            break;
                        }
                    }
                }
                if !publish_pipe_text(&sender, text.feed(&[], true), &pending, &wake) {
                    return;
                }
                publish(&sender, Message::PipeClosed(stream), &pending, &wake);
            });
        }
        let pending = self.wake_pending.clone();
        let wake = state.wake.clone();
        thread::spawn(move || {
            let status = match child.wait() {
                Ok(status) => status,
                Err(error) => {
                    publish(
                        &sender,
                        Message::Error(format!("Command wait failed: {error}")),
                        &pending,
                        &wake,
                    );
                    -1
                }
            };
            publish(&sender, Message::PipeExited(status), &pending, &wake);
        });
        Ok(())
    }
    /// Call only after authenticating a Bridge completion request.
    pub(super) fn command_finished(
        &mut self,
        id: u64,
        status: i32,
        directory: std::path::PathBuf,
    ) -> bool {
        #[cfg(unix)]
        if self.stop_requested {
            return false;
        }
        let accepted = self
            .lifecycle
            .as_mut()
            .is_some_and(|state| state.finish(id, status, directory));
        self.work_pending |= accepted;
        accepted
    }
    pub(super) fn take_completion(&mut self) -> Option<Completion> {
        #[cfg(unix)]
        if !self.stop_requested && self.completion_after_stop.is_some() {
            return self.completion_after_stop.take();
        }
        self.lifecycle.as_mut().and_then(Lifecycle::take_completion)
    }
    pub(super) fn clear(&mut self) {
        if let Some(plain) = &mut self.plain {
            plain.reset();
            // Clear may arrive while the hidden completion OSC spans reader
            // slices. Keep its remaining nonce/status bytes invisible, and
            // leave the protocol parser's in-flight state undisturbed.
            let prefix: &[u8] = if self.plain_marker_position >= 3 {
                b"\x1b]"
            } else if self.plain_marker_position == 2 {
                b"\x1b"
            } else {
                b""
            };
            let _ = plain.process(prefix, |_, _| Ok(()));
            self.history_offset = 0;
            self.history_view = None;
            return;
        }
        if self.transcript.is_some() {
            // The caller resets the shared transcript once. This only clears
            // the active surface; an unfinished command keeps its source.
            self.parser.process(b"\x1b[3J\x1b[2J\x1b[H");
            if let Some(seal) = &mut self.seal {
                seal.next = seal.rows;
            }
            if let Some(frame) = &mut self.parser.callbacks_mut().end_frame {
                *frame = vt100::Parser::new(frame.size().0, frame.size().1, 0)
                    .screen()
                    .clone();
            }
        } else {
            self.parser.process(b"\x1b[3J\x1b[2J\x1b[H");
        }
        self.history_offset = 0;
        self.history_view = None;
    }

    /// The transient normal-screen tail is visible only while its command
    /// owns input. Committed rows live exclusively in the shared transcript.
    pub(super) fn live_tail_rows(&self) -> u16 {
        if self.plain.is_some() {
            return u16::from(self.plain_tail().is_some());
        }
        self.tail_screen().map_or(0, |(_, _, rows)| rows)
    }

    pub(super) fn plain_tail(&self) -> Option<(&str, u64)> {
        if self.tail_sealed || self.plain_failed {
            return None;
        }
        let text = self.plain.as_ref()?.tail();
        (!text.is_empty()).then_some((text, self.origin.as_ref()?.source))
    }

    /// During bounded sealing, the rows not yet written keep their future
    /// record IDs: shared next-ID + offset within this remaining slice.
    pub(super) fn tail_screen(&self) -> Option<(&vt100::Screen, u16, u16)> {
        if self.plain.is_some() {
            return None;
        }
        if self.transcript.is_none() {
            return Some((self.live_screen(), 0, self.live_screen().size().0));
        }
        if let Some(seal) = &self.seal {
            let count = seal.rows.saturating_sub(seal.next);
            return (count > 0).then_some((&seal.screen, seal.next, count));
        }
        if let Some(frame) = &self.parser.callbacks().end_frame {
            let count = occupied_rows(frame);
            return (count > 0).then_some((frame, 0, count));
        }
        if !self.running() || self.parser.callbacks().end_seen || self.eof_seal {
            return None;
        }
        let count = occupied_rows(self.live_screen());
        (count > 0).then_some((self.live_screen(), 0, count))
    }

    pub(super) fn tail_source(&self) -> Option<u64> {
        if self.plain.is_some() {
            return self.plain_tail().map(|(_, source)| source);
        }
        if self.tail_screen().is_none() {
            return None;
        }
        self.seal
            .as_ref()
            .map(|seal| seal.origin.source)
            .or_else(|| self.origin.as_ref().map(|origin| origin.source))
    }

    fn reset_managed_grid(&mut self) {
        if let Some(capture) = &self.capture {
            capture.lock().unwrap().active = None;
        }
        let (rows, cols) = self.parser.screen().size();
        let ready = self.ready();
        self.parser = vt100::Parser::new_with_callbacks(
            rows,
            cols,
            HISTORY_ROWS,
            Replies {
                ready,
                capture: self.capture.clone(),
                ..Replies::default()
            },
        );
        if let Some(capture) = &self.capture {
            self.parser
                .screen_mut()
                .set_scrollback_sink(capture.clone());
        }
    }
    pub(super) fn screen(&self) -> &vt100::Screen {
        self.history_view
            .as_ref()
            .map_or(self.parser.screen(), |view| view.screen())
    }
    pub(super) fn live_screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub(super) fn invalidate_relative_links(&mut self) {
        if let Some(origin) = &mut self.origin {
            origin.relative_safe = false;
        } else if let (Some(transcript), Some(anchor)) = (&self.transcript, self.last_result) {
            let mut store = transcript.lock().unwrap();
            if store.epoch() == anchor.epoch && store.read_anchor(anchor).is_ok() {
                if let Err(error) = store.invalidate_result(anchor) {
                    self.status = Some(error.to_string());
                }
            }
        }
    }
    pub(super) fn supports_input(&self) -> bool {
        if self.transcript.is_some() {
            return false;
        }
        #[cfg(windows)]
        if self.windows.is_some() {
            return false;
        }
        true
    }
    #[cfg(unix)]
    fn stop_managed(&mut self) -> io::Result<()> {
        if self.stop_requested {
            return Ok(());
        }
        let lifecycle = self.lifecycle.as_ref().unwrap();
        let Some(id) = lifecycle.current_id() else {
            return Ok(());
        };
        let shell = self
            .shell_pid
            .ok_or_else(|| io::Error::other("The owned terminal process is unavailable"))?;
        if shell <= 0 || shell == unsafe { libc::getpgrp() } {
            return Err(io::Error::other("The terminal process group is invalid"));
        }
        self.completion_after_stop = Some(Completion {
            id,
            status: 130,
            directory: self.managed_restart.as_ref().unwrap().directory.clone(),
        });
        self.stop_requested = true;
        if let Some(origin) = &mut self.origin {
            origin.relative_safe = false;
        }
        // Kill the owned dispatcher PID before either process group. Group
        // delivery visits members individually: killing its waiting child
        // first can otherwise wake the shell into the next command.
        let foreground = self
            .master
            .as_ref()
            .and_then(|master| master.process_group_leader());
        if unsafe { libc::kill(shell, libc::SIGKILL) } == -1 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
        // Clean up both owned groups, including a program with job control.
        // A fresh driver opens only after ordered EOF seals the old output;
        // no blocking wait occurs here. Do not signal the same group twice.
        for group in [Some(shell), foreground.filter(|group| *group != shell)]
            .into_iter()
            .flatten()
        {
            if group <= 0 || group == unsafe { libc::getpgrp() } {
                continue;
            }
            if unsafe { libc::kill(-group, libc::SIGKILL) } == -1 {
                let error = io::Error::last_os_error();
                // macOS excludes zombie members from group delivery and can
                // return EPERM for a dying group with no eligible live member.
                let empty_macos_group =
                    cfg!(target_os = "macos") && error.raw_os_error() == Some(libc::EPERM);
                if error.raw_os_error() != Some(libc::ESRCH) && !empty_macos_group {
                    return Err(error);
                }
            }
        }
        self.status = Some("Stopping command…".into());
        Ok(())
    }
    pub(super) fn send(&mut self, bytes: Vec<u8>) -> io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        if bytes.len() > MAX_PASTE_BYTES + 12 {
            return Err(io::Error::other("Input exceeds the 64 KiB paste limit"));
        }
        #[cfg(windows)]
        let running = self.running();
        #[cfg(windows)]
        if let Some(state) = &mut self.windows {
            if bytes == [3] && running {
                if let Some(killer) = &mut self.killer {
                    killer.kill()?;
                }
                state.stopped = true;
                return Ok(());
            }
            return Err(io::Error::other(
                "Windows commands do not accept interactive input",
            ));
        }
        #[cfg(unix)]
        if bytes == [3] && self.running() && self.managed_restart.is_some() {
            return self.stop_managed();
        }
        if self.closed || self.exited.load(Ordering::Acquire) {
            return Err(io::Error::other(
                "The shell has exited; open :term-new for a new session",
            ));
        }
        self.input
            .as_ref()
            .ok_or_else(|| io::Error::other("Terminal input is closed"))?
            .try_send(bytes)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    io::Error::other("Terminal input is busy; try again")
                }
                mpsc::TrySendError::Disconnected(_) => io::Error::other("Terminal input is closed"),
            })?;
        self.history_offset = 0;
        self.history_view = None;
        self.status = None;
        Ok(())
    }

    pub(super) fn paste(&mut self, text: &str) -> io::Result<()> {
        if text.len() > MAX_PASTE_BYTES {
            return Err(io::Error::other("Paste is limited to 64 KiB"));
        }
        // Normalize clipboard newlines to Enter. Strip control bytes so clipboard
        // text cannot escape bracketed paste or inject terminal key sequences.
        let clean: String = text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
            .collect();
        if clean.is_empty() {
            return Ok(());
        }
        let bytes = if self.live_screen().bracketed_paste() {
            format!("\x1b[200~{clean}\x1b[201~").into_bytes()
        } else {
            clean.replace('\n', "\r").into_bytes()
        };
        self.send(bytes)
    }

    pub(super) fn resize(&mut self, rows: u16, cols: u16) -> io::Result<bool> {
        let rows = rows.clamp(1, MAX_ROWS);
        let cols = cols.clamp(1, MAX_COLS);
        if self.live_screen().size() == (rows, cols) {
            return Ok(false);
        }
        if let Some(master) = &self.master {
            master.resize(size(rows, cols)).map_err(io::Error::other)?;
        }
        self.parser.screen_mut().set_size(rows, cols);
        self.refresh_history();
        Ok(true)
    }

    pub(super) fn scroll(&mut self, rows: isize) {
        if self.live_screen().alternate_screen() {
            return;
        }
        let Some(history) = &self.history else {
            return;
        };
        let count = history.lock().unwrap().len();
        self.history_offset = self.history_offset.saturating_add_signed(rows).min(count);
        self.refresh_history();
    }

    fn refresh_history(&mut self) {
        let Some(history) = &self.history else {
            self.history_view = None;
            return;
        };
        if self.history_offset == 0 || self.live_screen().alternate_screen() {
            self.history_view = None;
            return;
        }
        let (rows, cols) = self.live_screen().size();
        let result = (|| -> io::Result<vt100::Parser> {
            let mut history = history.lock().unwrap();
            self.history_offset = self.history_offset.min(history.len());
            let mut view = vt100::Parser::new(rows, cols, 0);
            view.process(b"\x1b[?25l");
            let count = self.history_offset.min(rows as usize);
            let first = history.len() - self.history_offset;
            for index in 0..count {
                let saved = history.row(first + index)?;
                // Decode at the saved width before clipping. A narrower pane
                // must never wrap a saved row over the following rows.
                let mut row = vt100::Parser::new(1, saved.cols.clamp(1, MAX_COLS), 0);
                row.process(&saved.bytes);
                row.screen_mut()
                    .set_row_wrapped(0, saved.wrapped && saved.cols == cols);
                view.screen_mut().restore_row(index as u16, row.screen(), 0);
            }
            for index in count..rows as usize {
                view.screen_mut().restore_row(
                    index as u16,
                    self.parser.screen(),
                    (index - count) as u16,
                );
            }
            Ok(view)
        })();
        match result {
            Ok(view) => self.history_view = Some(view),
            Err(error) => {
                self.status = Some(format!("History read failed: {error}"));
                self.history_offset = 0;
                self.history_view = None;
            }
        }
    }

    fn update_history(&mut self) {
        let Some(history) = &self.history else {
            if let Some(capture) = &self.capture {
                let mut capture = capture.lock().unwrap();
                if let Some(error) = capture.error.take() {
                    self.status = Some(error);
                }
            }
            if let Some(transcript) = &self.transcript
                && let Err(error) = transcript.lock().unwrap().flush()
            {
                self.status = Some(format!("Transcript flush failed: {error}"));
            }
            return;
        };
        let mut history = history.lock().unwrap();
        if let Err(error) = history.flush() {
            self.status = Some(format!("History flush failed: {error}"));
        }
        if let Some(error) = history.error.take() {
            self.status = Some(error);
        }
        let added = history.serial.saturating_sub(self.history_serial) as usize;
        self.history_serial = history.serial;
        if self.history_offset > 0 {
            self.history_offset = self.history_offset.saturating_add(added).min(history.len());
        }
        drop(history);
        self.refresh_history();
    }

    fn observe_boundary(&mut self) {
        if self.parser.callbacks().end_seen {
            if let Some(lifecycle) = &mut self.lifecycle
                && let Some(id) = lifecycle.current_id()
            {
                lifecycle.observe_end(id);
            }
            if self.plain.is_some() && !self.tail_sealed {
                self.seal_plain();
            }
            if let Some(screen) = self.parser.callbacks_mut().end_frame.take()
                && let Some(origin) = &self.origin
            {
                self.seal = Some(Seal {
                    rows: occupied_rows(&screen),
                    screen,
                    origin: origin.clone(),
                    next: 0,
                });
            }
            if self.transcript.is_none() {
                self.tail_sealed = true;
            }
        }
    }

    fn seal_plain(&mut self) {
        if let (Some(plain), Some(store), Some(origin)) =
            (&mut self.plain, &self.transcript, &self.origin)
        {
            if !self.plain_failed {
                // Reuse one lazy guard for this bounded call; a call with no
                // emitted rows needs no transcript lock.
                let mut locked = None;
                let result = plain.finish(|text, continued| {
                    locked
                        .get_or_insert_with(|| store.lock().unwrap())
                        .append_plain(&origin.directory, origin.source, text, continued)
                        .map(|_| ())
                });
                drop(locked);
                if let Err(error) = result {
                    self.plain_failed = true;
                    self.status = Some(format!("Transcript tail write failed: {error}"));
                }
            }
            self.tail_sealed = true;
        }
    }

    fn process_output(&mut self, bytes: &[u8]) {
        if self.plain.is_none() {
            self.parser.process(bytes);
            return;
        }
        // Find the bounded authenticated marker without splitting ordinary
        // output into individual parser calls. Its CAN prefix cannot occur in
        // the remaining marker, so a mismatching prefix restarts in one step.
        let mut capture_end = if self.parser.callbacks().end_seen {
            0
        } else {
            bytes.len()
        };
        let mut boundary = false;
        if capture_end > 0 && !self.plain_marker.is_empty() {
            for (position, byte) in bytes.iter().enumerate() {
                if *byte == self.plain_marker[self.plain_marker_position] {
                    self.plain_marker_position += 1;
                } else {
                    self.plain_marker_position = usize::from(*byte == self.plain_marker[0]);
                }
                if self.plain_marker_position == self.plain_marker.len() {
                    capture_end = position + 1;
                    boundary = true;
                    self.plain_marker_position = 0;
                    break;
                }
            }
        }
        if !self.plain_failed {
            if let (Some(plain), Some(store), Some(origin)) =
                (&mut self.plain, &self.transcript, &self.origin)
            {
                // Reuse one lazy guard for this bounded call; a call with no
                // emitted rows needs no transcript lock.
                let mut locked = None;
                let result = plain.process(&bytes[..capture_end], |text, continued| {
                    locked
                        .get_or_insert_with(|| store.lock().unwrap())
                        .append_plain(&origin.directory, origin.source, text, continued)
                        .map(|_| ())
                });
                drop(locked);
                if let Err(error) = result {
                    self.plain_failed = true;
                    self.status = Some(format!("Transcript write failed: {error}"));
                }
            }
        }
        // Production renders literal output, so it has no VT surface to update.
        // Parse the private driver's tiny startup once, then only an exactly
        // matched authenticated completion marker. Building/scrolling a second
        // grid for every printable byte would duplicate the text capture work.
        if !self.parser.callbacks().ready {
            self.parser.process(bytes);
        } else if boundary {
            self.parser.process(&self.plain_marker);
        }
        self.observe_boundary();
    }

    fn seal_tail(&mut self, start: Instant) -> bool {
        let mut changed = false;
        if let Some(seal) = &mut self.seal {
            // At most one bounded grid row is encoded at a time. Do not let a
            // large final screen monopolize the UI turn or form a heap log.
            for _ in 0..8 {
                if seal.next >= seal.rows {
                    break;
                }
                let row = seal.next;
                let formatted = seal.screen.row_scrollback_formatted(row);
                let result = self
                    .transcript
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .append_terminal(
                        &seal.origin.directory,
                        seal.origin.source,
                        seal.screen.size().1,
                        seal.screen.row_wrapped(row),
                        &formatted,
                    );
                seal.next += 1;
                changed = true;
                if let Err(error) = result {
                    self.status = Some(format!("Transcript tail write failed: {error}"));
                    seal.next = seal.rows;
                    break;
                }
                if start.elapsed() >= Duration::from_millis(2) {
                    break;
                }
            }
            if seal.next == seal.rows {
                self.seal = None;
                self.tail_sealed = true;
            } else {
                self.work_pending = true;
            }
        }
        #[cfg(windows)]
        if self
            .windows
            .as_ref()
            .is_some_and(|state| state.streams == 3 && state.exit_status.is_some())
            && self.tail_sealed
        {
            let state = self.windows.as_mut().unwrap();
            let status = state.exit_status.take().unwrap();
            let directory = state.directory.take().unwrap();
            if let (Some(transcript), Some(origin)) = (&self.transcript, self.origin.take()) {
                match transcript.lock().unwrap().append_result_with_provenance(
                    &directory,
                    origin.source,
                    status,
                    &format!("Exit {status}"),
                    origin.relative_safe,
                ) {
                    Ok(anchor) => self.last_result = Some(anchor),
                    Err(error) => {
                        self.status = Some(format!("Transcript result write failed: {error}"))
                    }
                }
            }
            self.lifecycle
                .as_mut()
                .unwrap()
                .complete_pipes(status, directory);
            self.output.take();
            self.killer.take();
            state.streams = 0;
            changed = true;
            return changed;
        }
        if self.eof_seal && self.tail_sealed {
            if let (Some(transcript), Some(origin)) = (&self.transcript, self.origin.take()) {
                #[cfg(unix)]
                let stopped = self.stop_requested;
                #[cfg(not(unix))]
                let stopped = false;
                let result = transcript.lock().unwrap().append_result_with_provenance(
                    &origin.directory,
                    origin.source,
                    if stopped { 130 } else { -1 },
                    if stopped {
                        "Command stopped"
                    } else {
                        "Shell closed before a command completion boundary"
                    },
                    false,
                );
                match result {
                    Ok(anchor) => self.last_result = Some(anchor),
                    Err(error) => {
                        self.status = Some(format!("Transcript result write failed: {error}"))
                    }
                }
            }
            self.eof_seal = false;
            if let Some(lifecycle) = &mut self.lifecycle {
                lifecycle.abort_after_closed();
            }
            self.parser.callbacks_mut().expected_end = None;
            changed = true;
        } else if self.tail_sealed
            && self.lifecycle.as_ref().is_some_and(Lifecycle::paired)
            && !self.stopping()
        {
            if let (Some(transcript), Some(origin), Some(metadata)) = (
                &self.transcript,
                self.origin.take(),
                self.lifecycle.as_ref().and_then(Lifecycle::metadata),
            ) {
                match transcript.lock().unwrap().append_result_with_provenance(
                    &metadata.directory,
                    origin.source,
                    metadata.status,
                    &format!("Exit {}", metadata.status),
                    origin.relative_safe,
                ) {
                    Ok(anchor) => self.last_result = Some(anchor),
                    Err(error) => {
                        self.status = Some(format!("Transcript result write failed: {error}"))
                    }
                }
            }
            self.lifecycle.as_mut().unwrap().release_after_seal();
            self.parser.callbacks_mut().expected_end = None;
            changed = true;
        }
        changed
    }

    fn ordered_closed(&mut self) {
        self.closed = true;
        if self.transcript.is_none() {
            #[cfg(unix)]
            if self.stop_requested {
                self.tail_sealed = true;
                self.eof_seal = true;
                return;
            }
            if let Some(lifecycle) = &mut self.lifecycle {
                lifecycle.abort_after_closed();
            }
            return;
        }
        if self.origin.is_none() {
            return;
        }
        if self.plain.is_some() {
            if !self.tail_sealed {
                self.seal_plain();
            }
            self.eof_seal = true;
            return;
        }
        if !self.parser.callbacks().end_seen {
            let screen = self.parser.screen().normal_screen_clone();
            self.capture.as_ref().unwrap().lock().unwrap().active = None;
            self.seal = Some(Seal {
                rows: occupied_rows(&screen),
                screen,
                origin: self.origin.clone().unwrap(),
                next: 0,
            });
        }
        self.eof_seal = true;
    }

    fn stopping(&self) -> bool {
        #[cfg(unix)]
        return self.stop_requested;
        #[cfg(not(unix))]
        false
    }

    #[cfg(unix)]
    fn poll_restart(&mut self) -> bool {
        if !self.stop_requested {
            return false;
        }
        if self.closed
            && self.tail_sealed
            && !self.eof_seal
            && self.exited.load(Ordering::Acquire)
            && self.restart_receiver.is_none()
            && self.restarting.is_none()
        {
            let config = self.managed_restart.as_ref().unwrap().clone();
            let transcript = self.transcript.clone();
            let (rows, cols) = self.parser.screen().size();
            let (sender, receiver) = mpsc::sync_channel(1);
            self.restart_receiver = Some(receiver);
            thread::spawn(move || {
                let environment: Vec<_> = config
                    .environment
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.clone()))
                    .collect();
                let result = Self::open_managed_shell(
                    config.shell,
                    &config.directory,
                    rows,
                    cols,
                    config.wake.clone(),
                    &environment,
                    transcript,
                );
                let _ = sender.send(result);
                (config.wake)();
            });
        }
        if let Some(receiver) = &self.restart_receiver {
            match receiver.try_recv() {
                Ok(Ok(session)) => {
                    self.restart_receiver = None;
                    self.restarting = Some(Box::new(session));
                }
                Ok(Err(error)) => {
                    self.restart_receiver = None;
                    self.stop_requested = false;
                    self.status =
                        Some(format!("Command stopped; terminal restart failed: {error}"));
                    return true;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.restart_receiver = None;
                    self.stop_requested = false;
                    self.status = Some("Command stopped; terminal restart worker closed".into());
                    return true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(session) = &mut self.restarting {
            session.poll();
            if session.closed || session.exited.load(Ordering::Acquire) {
                self.restarting = None;
                self.stop_requested = false;
                self.status = Some("Command stopped; replacement terminal could not start".into());
                return true;
            }
            if session.ready() {
                let mut replacement = self.restarting.take().unwrap();
                let (rows, cols) = self.parser.screen().size();
                if let Err(error) = replacement.resize(rows, cols) {
                    self.stop_requested = false;
                    self.status = Some(format!(
                        "Command stopped; replacement resize failed: {error}"
                    ));
                    return true;
                }
                replacement
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .resume_after(self.completion_after_stop.as_ref().unwrap().id);
                replacement.completion_after_stop = self.completion_after_stop.take();
                replacement.last_result = self.last_result;
                if let Some(history) = &self.history {
                    replacement.history = Some(history.clone());
                    replacement
                        .parser
                        .screen_mut()
                        .set_scrollback_sink(history.clone());
                    // Raw managed comparison tests retain their completed
                    // screen until the next command produces output.
                    let screen = self.parser.screen().normal_screen_clone();
                    for row in 0..screen.size().0 {
                        replacement
                            .parser
                            .screen_mut()
                            .restore_row(row, &screen, row);
                    }
                    replacement.history_serial = self.history_serial;
                }
                let replacement = *replacement;
                *self = replacement;
                return true;
            }
        }
        false
    }

    #[cfg(windows)]
    fn observe_windows_end(&mut self) {
        if !self
            .windows
            .as_ref()
            .is_some_and(|state| state.streams == 3 && state.exit_status.is_some())
            || self.tail_sealed
            || self.seal.is_some()
        {
            return;
        }
        // Both independent pipe producers enqueue EOF after their last bytes.
        // The parser has consumed those bytes before this control message.
        if self.plain.is_some() {
            self.seal_plain();
            return;
        }
        if let (Some(capture), Some(origin)) = (&self.capture, &self.origin) {
            capture.lock().unwrap().active = None;
            let screen = self.parser.screen().normal_screen_clone();
            self.seal = Some(Seal {
                rows: occupied_rows(&screen),
                screen,
                origin: origin.clone(),
                next: 0,
            });
        } else {
            self.tail_sealed = true;
        }
    }

    pub(super) fn poll(&mut self) -> bool {
        self.wake_pending.store(false, Ordering::Release);
        self.work_pending = false;
        let start = Instant::now();
        let mut changed = false;
        // Check the time budget inside reader chunks. Newline-heavy output or
        // repeated controls must yield without consuming a full 16 KiB chunk.
        for _ in 0..(8 * CHUNK / PARSE_SLICE) {
            if self.pending_output.is_none() {
                let Some(output) = &self.output else {
                    break;
                };
                let Ok(message) = output.try_recv() else {
                    break;
                };
                match message {
                    Message::Output(bytes) => self.pending_output = Some((bytes, 0)),
                    Message::Closed => self.ordered_closed(),
                    Message::Exited(status) | Message::Error(status) => self.status = Some(status),
                    #[cfg(windows)]
                    Message::PipeClosed(stream) => {
                        if let Some(state) = &mut self.windows {
                            state.streams |= stream;
                        }
                        self.observe_windows_end();
                    }
                    #[cfg(windows)]
                    Message::PipeExited(status) => {
                        if let Some(state) = &mut self.windows {
                            state.exit_status = Some(status);
                        }
                        self.observe_windows_end();
                    }
                }
            }
            if let Some((bytes, position)) = self.pending_output.take() {
                let end = (position + PARSE_SLICE).min(bytes.len());
                self.process_output(&bytes[position..end]);
                self.observe_boundary();
                if end < bytes.len() {
                    self.pending_output = Some((bytes, end));
                }
            }
            changed = true;
            self.work_pending = true;
            if start.elapsed() >= Duration::from_millis(2) {
                break;
            }
        }
        changed |= self.seal_tail(start);
        #[cfg(unix)]
        {
            changed |= self.poll_restart();
        }
        if changed {
            self.update_history();
        }
        // Responses share the bounded input queue with keyboard input. Keep an
        // unsent response for the next UI turn rather than blocking the UI.
        let replies = &mut self.parser.callbacks_mut().pending;
        if let Some(input) = &self.input {
            while !replies.is_empty() {
                match input.try_send(replies.remove(0)) {
                    Ok(()) => {}
                    Err(mpsc::TrySendError::Full(reply)) => {
                        replies.insert(0, reply);
                        break;
                    }
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        replies.clear();
                        break;
                    }
                }
            }
        }
        changed
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Release blocked output producers before ClosePseudoConsole runs.
        // Otherwise a full queue can deadlock Windows console destruction.
        self.output.take();
        self.input.take();
        if !self.exited.load(Ordering::Acquire) {
            #[cfg(unix)]
            if let Some(group) = self
                .master
                .as_ref()
                .and_then(|master| master.process_group_leader())
            {
                // The foreground job can have a different process group from
                // the shell. Hang up both when this terminal window closes.
                unsafe {
                    if group > 0 && group != libc::getpgrp() {
                        libc::kill(-group, libc::SIGHUP);
                        libc::kill(-group, libc::SIGCONT);
                    }
                }
            }
            if let Some(killer) = &mut self.killer {
                let _ = killer.kill();
            }
        }
        // Waiter owns/reaps the child; it never blocks the UI's destructor.
    }
}

fn size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn occupied_rows(screen: &vt100::Screen) -> u16 {
    let (rows, cols) = screen.size();
    if screen.alternate_screen() {
        return rows;
    }
    let (cursor_row, cursor_col) = screen.cursor_position();
    // A cursor at column zero after LF denotes the next empty line, not a
    // padding row to persist. Earlier empty rows still represent real LFs.
    let mut count = cursor_row
        .saturating_add(u16::from(cursor_col > 0))
        .min(rows);
    for row in (0..rows).rev() {
        if (0..cols).any(|col| {
            screen
                .cell(row, col)
                .is_some_and(|cell| cell.has_contents() || cell.is_wide_continuation())
        }) {
            count = count.max(row + 1);
            break;
        }
    }
    count
}

fn publish(
    sender: &SyncSender<Message>,
    message: Message,
    pending: &AtomicBool,
    wake: &Wake,
) -> bool {
    if sender.send(message).is_err() {
        return false;
    }
    if !pending.swap(true, Ordering::AcqRel) {
        wake();
    }
    true
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
