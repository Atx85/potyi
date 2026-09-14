// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

//! Bounded stdio transport. Blocking pipe I/O never runs on the UI thread.
use super::{ServerConfig, file_uri};
use crate::formatting::process::Running;
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::thread;
use std::time::{Duration, Instant};

const MAX_MESSAGE: usize = 8 * 1024 * 1024;
const MAX_HEADERS: usize = 8192;

pub(super) fn read_message(reader: &mut impl BufRead) -> io::Result<Value> {
    let mut length = None;
    let mut header_bytes = 0;
    loop {
        let mut line = String::new();
        let count = reader
            .take((MAX_HEADERS - header_bytes + 1) as u64)
            .read_line(&mut line)?;
        header_bytes += count;
        if count == 0 || header_bytes > MAX_HEADERS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP closed its output or sent an oversized header",
            ));
        }
        if line == "\r\n" {
            break;
        }
        let (key, value) = line
            .trim_end()
            .split_once(':')
            .ok_or_else(|| io::Error::other("Invalid LSP header"))?;
        if key.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(io::Error::other("Duplicate LSP Content-Length"));
            }
            length = Some(value.trim().parse::<usize>().map_err(io::Error::other)?);
        }
    }
    let length = length
        .filter(|n| *n > 0 && *n <= MAX_MESSAGE)
        .ok_or_else(|| io::Error::other("Missing or oversized LSP Content-Length"))?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

pub(super) fn write_message(writer: &mut impl Write, message: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(message).map_err(io::Error::other)?;
    if bytes.len() > MAX_MESSAGE {
        return Err(io::Error::other("LSP request exceeds size limit"));
    }
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

pub(super) struct Transport {
    process: Running,
    outgoing: SyncSender<Value>,
    incoming: Receiver<Result<Value, String>>,
    cancel: Arc<AtomicBool>,
    next_id: u64,
    root: String,
    options: Value,
    initialized: bool,
    failed: Cell<bool>,
    quiescent: Cell<Option<bool>>,
    diagnostics: RefCell<HashMap<String, (Option<i64>, Vec<Value>)>>,
}

impl Transport {
    pub fn start(
        config: &ServerConfig,
        root: &Path,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        #[cfg(windows)]
        if config.command.to_ascii_lowercase().ends_with(".cmd")
            || config.command.to_ascii_lowercase().ends_with(".bat")
        {
            return Err(
                "Use a native language-server executable or invoke its interpreter explicitly"
                    .into(),
            );
        }
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let mut process = Running::spawn(&mut command).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                format!(
                    "Language server '{}' was not found. Use :lsp install <server> for supported servers, or install it on PATH / set its full path in config/lsp.toml, then use :lsp restart. You can keep editing without LSP.",
                    config.command,
                )
            } else {
                format!("Could not start {}: {error}", config.command)
            }
        })?;
        let mut input = process.child.stdin.take().unwrap();
        let output = process.child.stdout.take().unwrap();
        let (outgoing, writes) = mpsc::sync_channel::<Value>(8);
        let (messages, incoming) = mpsc::sync_channel(4);
        let errors = messages.clone();
        // Separate readers/writers prevent a server that stops reading stdin
        // from blocking the coordinator's request timeout or cancellation.
        let reader = thread::Builder::new()
            .name("lsp-read".into())
            .spawn(move || {
                let mut reader = BufReader::new(output);
                loop {
                    let message = read_message(&mut reader).map_err(|e| e.to_string());
                    let failed = message.is_err();
                    if messages.send(message).is_err() || failed {
                        break;
                    }
                }
            });
        if let Err(error) = reader {
            return Err(error.to_string());
        }
        let writer = thread::Builder::new()
            .name("lsp-write".into())
            .spawn(move || {
                while let Ok(message) = writes.recv() {
                    if let Err(error) = write_message(&mut input, &message) {
                        let _ = errors.send(Err(error.to_string()));
                        break;
                    }
                }
            });
        if let Err(error) = writer {
            return Err(error.to_string());
        }
        Ok(Self {
            process,
            outgoing,
            incoming,
            cancel,
            next_id: 1,
            root: file_uri(root)?,
            options: config.initialization_options.clone(),
            initialized: false,
            failed: Cell::new(false),
            quiescent: Cell::new(None),
            diagnostics: RefCell::new(HashMap::new()),
        })
    }

    fn send(&self, value: Value) -> Result<(), String> {
        self.outgoing.try_send(value).map_err(|_| {
            self.failed.set(true);
            "Language server is busy or disconnected; try :lsp restart".into()
        })
    }

    pub fn has_failed(&self) -> bool { self.failed.get() }

    /// Rust-analyzer initializes before project discovery finishes. Wait on its
    /// pushed status on the worker, without timer work on the editor thread.
    pub fn wait_until_ready(&self, timeout: Duration) -> Result<(), String> {
        self.wait_until_ready_cancellable(timeout, None)
    }

    pub fn wait_until_ready_cancellable(&self, timeout: Duration, cancelled: Option<&AtomicBool>) -> Result<(), String> {
        self.drain().inspect_err(|_| self.failed.set(true))?;
        let deadline = Instant::now() + timeout;
        while self.quiescent.get() != Some(true) {
            if cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) { return Err("Completion cancelled".into()); }
            if self.cancel.load(Ordering::Relaxed) { return Err("LSP stopped".into()); }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Err("Language server is still loading the project; try the command again shortly".into());
            };
            match self.incoming.recv_timeout(remaining.min(Duration::from_millis(50))) {
                Ok(message) => self.handle_server_message(&message.inspect_err(|_| self.failed.set(true))?)?,
                Err(mpsc::RecvTimeoutError::Timeout) => {},
                Err(_) => { self.failed.set(true); return Err("Language server disconnected".into()); }
            }
        }
        Ok(())
    }

    pub fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.send(json!({"jsonrpc":"2.0", "method":method, "params":params}))
    }

    pub fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        self.request_cancellable(method, params, timeout, None)
    }

    pub fn request_cancellable(&mut self, method: &str, params: Value, timeout: Duration,
        cancelled: Option<&AtomicBool>) -> Result<Value, String> {
        if cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) { return Err("Completion cancelled".into()); }
        self.failed.set(true);
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}))?;
        let deadline = Instant::now() + timeout;
        loop {
            if cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) {
                let _ = self.notify("$/cancelRequest", json!({"id":id}));
                self.failed.set(false);
                return Err("Completion cancelled".into());
            }
            if self.cancel.load(Ordering::Relaxed) {
                return Err("LSP stopped".into());
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                let _ = self.notify("$/cancelRequest", json!({"id":id}));
                return Err(format!("Language server timed out during {method}"));
            };
            let message = match self
                .incoming
                .recv_timeout(remaining.min(Duration::from_millis(50)))
            {
                Ok(message) => message?,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => return Err("Language server disconnected".into()),
            };
            if message.get("method").is_some() {
                self.handle_server_message(&message)?;
            } else if message.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = message.get("error") {
                    // A valid feature error does not imply a broken connection.
                    self.failed.set(false);
                    return Err(error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("LSP request failed")
                        .chars()
                        .take(500)
                        .collect());
                }
                let result = message.get("result").cloned().ok_or("Invalid LSP response")?;
                self.failed.set(false);
                return Ok(result);
            }
        }
    }

    fn handle_server_message(&self, message: &Value) -> Result<(), String> {
        // Late responses to cancelled requests need no response of their own.
        if message.get("method").is_none() { return Ok(()); }
        if message["method"] == "textDocument/publishDiagnostics" {
            if let (Some(uri), Some(items)) = (message["params"]["uri"].as_str(), message["params"]["diagnostics"].as_array()) {
                let mut diagnostics = self.diagnostics.borrow_mut();
                if !diagnostics.contains_key(uri) && diagnostics.len() >= 4 { diagnostics.clear(); }
                let mut size = 0;
                let items = items.iter().take(256).take_while(|item| {
                    size += item.to_string().len(); size <= 256 * 1024
                }).cloned().collect();
                diagnostics.insert(uri.to_owned(), (message["params"]["version"].as_i64(), items));
            }
        }
        if message["method"] == "experimental/serverStatus" {
            if let Some(quiescent) = message["params"]["quiescent"].as_bool() {
                self.quiescent.set(Some(quiescent));
            }
        }
        let Some(id) = message.get("id") else {
            return Ok(());
        };
        let result = match message["method"].as_str().unwrap_or("") {
            "workspace/configuration" => {
                let items = message["params"]["items"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                Value::Array(
                    items
                        .iter()
                        .map(|item| {
                            let mut value = &self.options;
                            if let Some(section) = item["section"].as_str() {
                                for key in section.split('.') {
                                    value = &value[key];
                                }
                            }
                            value.clone()
                        })
                        .collect(),
                )
            }
            "workspace/workspaceFolders" => json!([{"uri":self.root,"name":"project"}]),
            "window/workDoneProgress/create" | "window/showMessageRequest" => Value::Null,
            "workspace/applyEdit" => {
                json!({"applied":false,"failureReason":"Server-initiated edits are disabled; use the rename preview"})
            }
            _ => {
                return self.send(json!({"jsonrpc":"2.0", "id":id,
                "error":{"code":-32601,"message":"Client method not supported"}}));
            }
        };
        self.send(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }

    pub fn clear_diagnostics(&self, uri: &str) { self.diagnostics.borrow_mut().remove(uri); }

    pub fn action_diagnostics(&self, uri: &str, version: i64) -> Result<Vec<Value>, String> {
        self.drain()?;
        let deadline = Instant::now() + Duration::from_millis(750);
        loop {
            if let Some((v, items)) = self.diagnostics.borrow().get(uri) {
                if v.is_none_or(|v| v == version) { return Ok(items.clone()); }
            }
            if self.cancel.load(Ordering::Relaxed) { return Err("LSP stopped".into()); }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else { return Ok(Vec::new()); };
            match self.incoming.recv_timeout(remaining.min(Duration::from_millis(50))) {
                Ok(message) => self.handle_server_message(&message?)?,
                Err(mpsc::RecvTimeoutError::Timeout) => {},
                Err(_) => return Err("Language server disconnected".into()),
            }
        }
    }

    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }

    /// Drain unsolicited messages even while idle, without polling the UI.
    pub fn drain(&self) -> Result<(), String> {
        for message in self.incoming.try_iter().take(32) {
            self.handle_server_message(&message?)?;
        }
        Ok(())
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        // All cleanup runs on the coordinator thread, never during UI input.
        if self.initialized {
            self.cancel = Arc::new(AtomicBool::new(false));
            let _ = self.request("shutdown", Value::Null, Duration::from_millis(250));
            let _ = self.notify("exit", Value::Null);
            let deadline = Instant::now() + Duration::from_millis(100);
            while Instant::now() < deadline {
                if matches!(self.process.child.try_wait(), Ok(Some(_))) {
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
        // The shared process guard terminates the server process group/job
        // and reaps the direct child, including on timeout and startup errors.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framed_unicode_messages_and_multiple_frames() {
        let value = json!({"text":"héllo 🦀"});
        let mut bytes = Vec::new();
        write_message(&mut bytes, &value).unwrap();
        write_message(&mut bytes, &Value::Null).unwrap();
        let mut input = io::Cursor::new(bytes);
        assert_eq!(read_message(&mut input).unwrap(), value);
        assert_eq!(read_message(&mut input).unwrap(), Value::Null);
    }
    #[test]
    fn rejects_bad_or_unbounded_messages() {
        for input in [
            "\r\n",
            "Content-Length: 999999999\r\n\r\n",
            "Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
            "Content-Length: 3\r\n\r\n{}",
        ] {
            assert!(read_message(&mut io::Cursor::new(input)).is_err());
        }
        assert!(read_message(&mut io::Cursor::new(vec![b'x'; MAX_HEADERS + 1])).is_err());
    }
}
