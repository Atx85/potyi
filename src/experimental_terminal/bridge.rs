// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded, session-authenticated requests from shell helpers to the editor.
use super::Wake;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};

const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_TRACE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Request {
    token: String,
    pub(crate) operation: String,
    pub(crate) directory: PathBuf,
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) command_id: Option<u64>,
    #[serde(default)]
    pub(crate) exit_status: Option<i32>,
}
pub(crate) struct Bridge {
    pub(crate) address: String,
    pub(crate) token: String,
    pub(crate) requests: Receiver<Request>,
    stopped: Arc<AtomicBool>,
}
impl Bridge {
    pub(crate) fn new(wake: Wake) -> io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?.to_string();
        let mut random = [0u8; 32];
        getrandom::getrandom(&mut random).map_err(|e| io::Error::other(e.to_string()))?;
        let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let secret = token.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let (send, requests) = mpsc::sync_channel(32);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let Ok(mut stream) = stream else {
                    break;
                };
                let _ = stream.set_nodelay(true);
                let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));
                let result = read_request(&mut stream).and_then(|request| {
                    if request.token != secret
                        || !matches!(
                            request.operation.as_str(),
                            "edit" | "view" | "cwd" | "ready" | "complete"
                        )
                    {
                        return Err(io::Error::other("Invalid terminal request"));
                    }
                    if request.operation == "complete"
                        && (request.command_id.is_none_or(|id| id == 0)
                            || request.exit_status.is_none())
                    {
                        return Err(io::Error::other("Invalid command completion"));
                    }
                    if request.operation != "ready" {
                        send.try_send(request)
                            .map_err(|_| io::Error::other("Editor request queue is busy"))?;
                    }
                    wake();
                    Ok(())
                });
                let reply = match result {
                    Ok(()) => "OK".into(),
                    Err(e) => e.to_string(),
                };
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        Ok(Self {
            address,
            token,
            requests,
            stopped,
        })
    }
    pub(crate) fn environment(&self) -> io::Result<Vec<(&'static str, String)>> {
        Ok(vec![
            (
                "POTYI_TERM_CLIENT",
                client_executable(&std::env::current_exe()?)
                    .to_string_lossy()
                    .into_owned(),
            ),
            ("POTYI_TERM_ADDRESS", self.address.clone()),
            ("POTYI_TERM_TOKEN", self.token.clone()),
        ])
    }
}
fn client_executable(application: &Path) -> PathBuf {
    // Release packages and Cargo builds place a helper from the same target
    // beside the app. A standalone/copied app retains its original helper path.
    let helper =
        application.with_file_name(format!("potyi-term-helper{}", std::env::consts::EXE_SUFFIX));
    let usable = std::fs::metadata(&helper).is_ok_and(|metadata| {
        if !metadata.is_file() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            true
        }
    });
    if usable {
        helper
    } else {
        application.to_owned()
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        // Wake blocking accept. No recurring timer is needed while idle.
        let _ = TcpStream::connect(&self.address);
    }
}
fn read_request(stream: &mut TcpStream) -> io::Result<Request> {
    let mut size = [0u8; 4];
    stream.read_exact(&mut size)?;
    let size = u32::from_be_bytes(size) as usize;
    if size > MAX_REQUEST_BYTES {
        return Err(io::Error::other("Terminal request is too large"));
    }
    let mut data = vec![0; size];
    stream.read_exact(&mut data)?;
    serde_json::from_slice(&data).map_err(io::Error::other)
}

struct RequestPacket(Vec<u8>);
impl Write for RequestPacket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_REQUEST_BYTES + 4 - self.0.len() {
            return Err(io::Error::other("Terminal request is too large"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn request_packet(request: &Request) -> io::Result<Vec<u8>> {
    // Reserve the framing bytes first and serialize directly into one capped
    // buffer. Escaping must count toward the existing 64 KiB JSON limit too.
    let mut packet = RequestPacket(Vec::with_capacity(MAX_REQUEST_BYTES + 4));
    packet.0.extend_from_slice(&[0; 4]);
    serde_json::to_writer(&mut packet, request).map_err(io::Error::other)?;
    let size = (packet.0.len() - 4) as u32;
    packet.0[..4].copy_from_slice(&size.to_be_bytes());
    Ok(packet.0)
}
struct ClientTrace {
    file: File,
    start: Instant,
}
impl ClientTrace {
    fn new() -> Option<Self> {
        let path = std::env::var_os("POTYI_TERM_TRACE_FILE")?;
        Self::open(Path::new(&path))
    }
    fn open(path: &Path) -> Option<Self> {
        if !path.is_absolute() {
            return None;
        }
        let mut options = OpenOptions::new();
        options.read(true).append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path).ok()?;
        if !file.metadata().ok()?.is_file() {
            return None;
        }
        Some(Self {
            file,
            start: Instant::now(),
        })
    }
    fn record(&self, operation: &str, stage: &'static str) {
        // Trace labels never contain arguments, command text, paths or tokens.
        let operation = match operation {
            "edit" => "edit",
            "view" => "view",
            "cwd" => "cwd",
            "ready" => "ready",
            "complete" => "complete",
            "read-command" => "read-command",
            _ => "invalid",
        };
        // Helpers share one bounded file. Skip a contended trace rather than
        // delay a command, and hold the lock across the size check and write.
        if self.file.try_lock().is_err() {
            return;
        }
        let _ = (|| -> io::Result<()> {
            let remaining = MAX_TRACE_BYTES.saturating_sub(self.file.metadata()?.len());
            if remaining < 192 {
                return Ok(());
            }
            let record = format!(
                "POTYI_TERM_TRACE pid={} operation={operation} stage={stage} elapsed_us={}\n",
                std::process::id(),
                self.start.elapsed().as_micros()
            );
            if record.len() as u64 <= remaining {
                (&self.file).write_all(record.as_bytes())?;
            }
            Ok(())
        })();
        let _ = self.file.unlock();
    }
}
fn trace_client(trace: Option<&ClientTrace>, operation: &str, stage: &'static str) {
    if let Some(trace) = trace {
        trace.record(operation, stage);
    }
}
pub(crate) fn client(arguments: Vec<String>) -> io::Result<()> {
    let trace = ClientTrace::new();
    #[cfg(windows)]
    unsafe {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn AttachConsole(process: u32) -> i32;
        }
        // Release builds use the GUI subsystem; helpers need their parent's console.
        let _ = AttachConsole(u32::MAX);
    }
    let [address, token, operation, rest @ ..] = arguments.as_slice() else {
        return Err(io::Error::other("Invalid terminal helper arguments"));
    };
    trace_client(trace.as_ref(), operation, "client_enter");
    // Restrict the helper to the loopback endpoint supplied by this session.
    let endpoint: std::net::SocketAddr = address.parse().map_err(io::Error::other)?;
    if !endpoint.ip().is_loopback() {
        return Err(io::Error::other("Invalid terminal endpoint"));
    }
    #[cfg(windows)]
    if operation == "read-command" {
        return read_command_console();
    }
    let (command_id, exit_status, end_nonce) = if operation == "complete" {
        let (id, status, nonce) = match rest {
            [id, status] => (id, status, None),
            [id, status, nonce]
                if nonce.len() == 32 && nonce.bytes().all(|b| b.is_ascii_hexdigit()) =>
            {
                (id, status, Some(nonce.as_str()))
            }
            _ => return Err(io::Error::other("Invalid completion arguments")),
        };
        (
            Some(id.parse().map_err(io::Error::other)?),
            Some(status.parse().map_err(io::Error::other)?),
            nonce,
        )
    } else {
        (None, None, None)
    };
    let request = Request {
        token: token.clone(),
        operation: operation.clone(),
        directory: std::env::current_dir()?,
        path: if operation == "complete" {
            String::new()
        } else {
            rest.join(" ")
        },
        command_id,
        exit_status,
    };
    let packet = request_packet(&request)?;
    trace_client(trace.as_ref(), operation, "encoded");
    let mut stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?;
    stream.set_nodelay(true)?;
    trace_client(trace.as_ref(), operation, "connected");
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    // A separate tiny header write can leave its body waiting for a delayed
    // acknowledgment while the server is blocked reading that same body.
    stream.write_all(&packet)?;
    trace_client(trace.as_ref(), operation, "sent");
    let mut reply = String::new();
    stream.take(1024).read_to_string(&mut reply)?;
    trace_client(trace.as_ref(), operation, "replied");
    if reply == "OK" {
        if operation == "ready" {
            print!("\x1b]777;potyi-ready\x07");
            std::io::stdout().flush()?;
        } else if let Some(nonce) = end_nonce {
            // This byte-stream boundary is ordered after the program's PTY
            // output. The IPC acknowledgment alone cannot establish that.
            print!("\x18\x1b]777;potyi-end;{nonce};{}\x07", command_id.unwrap());
            std::io::stdout().flush()?;
        }
        trace_client(trace.as_ref(), operation, "completed");
        Ok(())
    } else {
        Err(io::Error::other(reply))
    }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod packet_tests;

/// Definitions used inside the persistent managed shell driver.
pub(super) fn setup_helpers(shell_name: &str) -> String {
    if shell_name == "fish" {
        return concat!("function edit; \"$POTYI_TERM_CLIENT\" --term-request $POTYI_TERM_ADDRESS $POTYI_TERM_TOKEN edit $argv; end; ",
            "function view; \"$POTYI_TERM_CLIENT\" --term-request $POTYI_TERM_ADDRESS $POTYI_TERM_TOKEN view $argv; end; ",
            "function cd; builtin cd $argv; and \"$POTYI_TERM_CLIENT\" --term-request $POTYI_TERM_ADDRESS $POTYI_TERM_TOKEN cwd; end; ").into();
    }
    if shell_name == "cmd" {
        return String::new();
    }
    concat!(
        "edit(){ \"$POTYI_TERM_CLIENT\" --term-request \"$POTYI_TERM_ADDRESS\" \"$POTYI_TERM_TOKEN\" edit \"$@\"; }; ",
        "view(){ \"$POTYI_TERM_CLIENT\" --term-request \"$POTYI_TERM_ADDRESS\" \"$POTYI_TERM_TOKEN\" view \"$@\"; }; ",
        "cd(){ command cd \"$@\" && \"$POTYI_TERM_CLIENT\" --term-request \"$POTYI_TERM_ADDRESS\" \"$POTYI_TERM_TOKEN\" cwd; }; "
    ).replace("command cd",if matches!(shell_name,"bash"|"zsh") { "builtin cd" } else { "command cd" })
}

#[cfg(windows)]
fn read_command_console() -> io::Result<()> {
    use std::ffi::c_void;
    type Handle = *mut c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            name: *const u16,
            access: u32,
            sharing: u32,
            security: *mut c_void,
            creation: u32,
            flags: u32,
            template: Handle,
        ) -> Handle;
        fn GetConsoleMode(handle: Handle, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: Handle, mode: u32) -> i32;
        fn ReadConsoleW(
            handle: Handle,
            buffer: *mut c_void,
            count: u32,
            read: *mut u32,
            reserved: *mut c_void,
        ) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }
    let name: Vec<u16> = "CONIN$\0".encode_utf16().collect();
    unsafe {
        let handle = CreateFileW(
            name.as_ptr(),
            0x80000000 | 0x40000000,
            3,
            std::ptr::null_mut(),
            3,
            0,
            std::ptr::null_mut(),
        );
        if handle as isize == -1 {
            return Err(io::Error::last_os_error());
        }
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            CloseHandle(handle);
            return Err(io::Error::last_os_error());
        }
        // Keep canonical lines and Ctrl+C processing, suppress opaque command IDs.
        if SetConsoleMode(handle, (mode | 1 | 2) & !4) == 0 {
            CloseHandle(handle);
            return Err(io::Error::last_os_error());
        }
        let expected_nonce = std::env::var("POTYI_DRIVER_NONCE").unwrap_or_default();
        let mut buffer = [0u16; 64];
        let mut line = Vec::with_capacity(64);
        let mut overflow = false;
        let result = (|| -> io::Result<()> {
            loop {
                let mut count = 0;
                if ReadConsoleW(
                    handle,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut count,
                    std::ptr::null_mut(),
                ) == 0
                    || count == 0
                {
                    return Err(io::Error::last_os_error());
                }
                for c in &buffer[..count as usize] {
                    if matches!(*c, 10 | 13) {
                        if !overflow {
                            let text = String::from_utf16(&line).map_err(io::Error::other)?;
                            let valid = text.split_once(':').is_some_and(|(nonce, id)| {
                                nonce == expected_nonce
                                    && nonce.len() == 32
                                    && !id.is_empty()
                                    && id.len() <= 20
                                    && id.bytes().all(|b| b.is_ascii_digit())
                            });
                            if valid {
                                println!("{text}");
                                return Ok(());
                            }
                        }
                        line.clear();
                        overflow = false;
                    } else if line.len() < 64 {
                        line.push(*c);
                    } else {
                        overflow = true;
                    }
                }
            }
        })();
        let _ = SetConsoleMode(handle, mode);
        CloseHandle(handle);
        result?;
    }
    Ok(())
}

pub(crate) fn setup_script() -> String {
    #[cfg(windows)]
    return concat!(
        "doskey edit=\"%POTYI_TERM_CLIENT%\" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% edit $*\r",
        "doskey view=\"%POTYI_TERM_CLIENT%\" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% view $*\r",
        "doskey cd=cd $* $T \"%POTYI_TERM_CLIENT%\" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% cwd\r",
        "cls\r\"%POTYI_TERM_CLIENT%\" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% ready\r"
    ).into();
    #[cfg(unix)]
    {
        let shell = std::env::var("SHELL").unwrap_or_default();
        if shell.rsplit('/').next() == Some("fish") {
            return concat!("function edit; \"$POTYI_TERM_CLIENT\" --term-request $POTYI_TERM_ADDRESS $POTYI_TERM_TOKEN edit $argv; end; ",
                "function view; \"$POTYI_TERM_CLIENT\" --term-request $POTYI_TERM_ADDRESS $POTYI_TERM_TOKEN view $argv; end; ",
                "function cd; builtin cd $argv; and \"$POTYI_TERM_CLIENT\" --term-request $POTYI_TERM_ADDRESS $POTYI_TERM_TOKEN cwd; end; ",
                "printf '\\033[2J\\033[H\\033]777;potyi-ready\\007'\r").into();
        }
        // POSIX functions also work in the supported bash/zsh interactive shells.
        let script = concat!(
            "edit(){ \"$POTYI_TERM_CLIENT\" --term-request \"$POTYI_TERM_ADDRESS\" \"$POTYI_TERM_TOKEN\" edit \"$@\"; }; ",
            "view(){ \"$POTYI_TERM_CLIENT\" --term-request \"$POTYI_TERM_ADDRESS\" \"$POTYI_TERM_TOKEN\" view \"$@\"; }; ",
            "cd(){ command cd \"$@\" && \"$POTYI_TERM_CLIENT\" --term-request \"$POTYI_TERM_ADDRESS\" \"$POTYI_TERM_TOKEN\" cwd; }; ",
            "printf '\\033[2J\\033[H\\033]777;potyi-ready\\007'\r"
        );
        script.replace(
            "command cd",
            if matches!(shell.rsplit('/').next(), Some("zsh" | "bash" | "ksh")) {
                "builtin cd"
            } else {
                "command cd"
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_bridge_completions_have_explicit_ids_status_and_cwd() {
        let bridge = Bridge::new(Arc::new(|| {})).unwrap();
        let args = vec![
            bridge.address.clone(),
            bridge.token.clone(),
            "complete".into(),
            "17".into(),
            "130".into(),
        ];
        client(args).unwrap();
        let request = bridge
            .requests
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        assert_eq!(request.operation, "complete");
        assert_eq!(request.command_id, Some(17));
        assert_eq!(request.exit_status, Some(130));
        assert_eq!(request.directory, std::env::current_dir().unwrap());
        assert!(request.path.is_empty());
        assert!(
            client(vec![
                bridge.address.clone(),
                bridge.token.clone(),
                "complete".into(),
                "malformed".into(),
                "0".into()
            ])
            .is_err()
        );
    }
    #[test]
    fn authenticated_bridge_rejects_foreign_requests_and_keeps_unicode_paths() {
        let bridge = Bridge::new(Arc::new(|| {})).unwrap();
        let arguments = vec![
            bridge.address.clone(),
            "wrong".into(),
            "edit".into(),
            "é notes.rs:4:2".into(),
        ];
        assert!(client(arguments).is_err());
        assert!(bridge.requests.try_recv().is_err());
        client(vec![
            bridge.address.clone(),
            bridge.token.clone(),
            "view".into(),
            "é notes.rs:4:2".into(),
        ])
        .unwrap();
        let request = bridge.requests.try_recv().unwrap();
        assert_eq!(request.operation, "view");
        assert_eq!(request.path, "é notes.rs:4:2");
    }
}
