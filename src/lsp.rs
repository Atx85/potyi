// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

//! Optional LSP. Each worker owns one server/project session.
//! The UI supplies bounded snapshots on commands/member triggers, never per frame.
mod transport;
mod actions;
pub(crate) mod completion;
pub(crate) mod unity;
pub(crate) use actions::CodeActions;
#[cfg(test)]
pub(crate) use actions::CodeActionItem;

use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, SyncSender},
};
use std::thread;
use std::time::Duration;
use transport::Transport;
use url::Url;

pub(crate) const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);
const HOVER_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub extensions: Vec<String>,
    pub language_id: String,
    #[serde(default)]
    pub root_markers: Vec<String>,
    #[serde(default)]
    pub initialization_options: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        use std::io::Read;
        let mut text = String::new();
        match std::fs::File::open("config/lsp.toml") {
            Ok(file) => {
                file.take(65537)
                    .read_to_string(&mut text)
                    .map_err(|e| e.to_string())?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                text.push_str(crate::embedded_config::LSP)
            }
            Err(e) => return Err(e.to_string()),
        }
        let mut config = Self::parse(&text)?;
        crate::lsp_setup::augment(&mut config);
        Ok(config)
    }

    fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 65536 {
            return Err("lsp.toml exceeds 64 KiB".into());
        }
        let config: Self = toml::from_str(text).map_err(|e| format!("Invalid lsp.toml: {e}"))?;
        if config.servers.len() > 32 {
            return Err("At most 32 language servers can be configured".into());
        }
        for (i, server) in config.servers.iter().enumerate() {
            if server.name.is_empty()
                || server.command.trim().is_empty()
                || server.command.contains('\0')
                || server.language_id.is_empty()
                || server.extensions.is_empty()
                || server.args.len() > 64
                || server.args.iter().any(|arg| arg.contains('\0'))
                || server
                    .root_markers
                    .iter()
                    .any(|m| m.is_empty() || m.contains(['/', '\\']) || m == "..")
                || config.servers[..i].iter().any(|s| s.name == server.name)
            {
                return Err("Invalid or duplicate language server configuration".into());
            }
        }
        Ok(config)
    }

    pub fn server_for(&self, path: &Path) -> Option<&ServerConfig> {
        let extension = path.extension()?.to_str()?;
        self.servers.iter().find(|s| {
            s.extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(extension))
        })
    }
}

pub(crate) fn project_root(path: &Path, config: &ServerConfig) -> PathBuf {
    if config.language_id == "csharp" {
        if let Some(root) = unity::root(path) { return root; }
    }
    let parent = path.parent().unwrap_or(path);
    // Prefer the repository root when present so Cargo workspace members share
    // a server. Otherwise use the nearest configured project marker.
    if config.root_markers.iter().any(|m| m == ".git") {
        if let Some(root) = parent.ancestors().find(|p| p.join(".git").exists()) {
            return root.to_path_buf();
        }
    }
    parent
        .ancestors()
        .find(|p| config.root_markers.iter().any(|m| p.join(m).exists()))
        .unwrap_or(parent)
        .to_path_buf()
}

pub(crate) fn file_uri(path: &Path) -> Result<String, String> {
    Url::from_file_path(path)
        .map(|url| url.into())
        .map_err(|_| "LSP needs an absolute file path".into())
}

pub(crate) fn uri_path(uri: &str) -> Result<PathBuf, String> {
    let url = Url::parse(uri).map_err(|e| e.to_string())?;
    if url.scheme() != "file" || url.query().is_some() || url.fragment().is_some() {
        return Err("This definition is not a local file".into());
    }
    if url
        .host_str()
        .is_some_and(|host| !host.is_empty() && host != "localhost")
    {
        return Err("Remote definition locations are not supported".into());
    }
    url.to_file_path()
        .map_err(|_| "Invalid definition file URI".into())
}

#[derive(Clone, Debug)]
pub(crate) enum Action {
    Start,
    Hover,
    Definition,
    Complete { trigger: String, cancel: Arc<AtomicBool> },
    ResolveCompletion { item: Value, cancel: Arc<AtomicBool> },
    Rename(String),
    CodeActions { anchor: usize, refactor_only: bool },
    ResolveCodeAction { action: Value, started: std::time::SystemTime },
}

#[derive(Clone, Debug)]
pub(crate) struct Document {
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug)]
pub(crate) struct Request {
    pub id: u64,
    pub action: Action,
    pub documents: Vec<Document>, // focused document first
    pub cursor: usize,            // UTF-8 byte offset
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub character: usize, // UTF-16 units
}

#[derive(Debug)]
pub(crate) struct HoverContent {
    pub text: String,
    pub documentation: Vec<std::ops::Range<usize>>,
}

#[derive(Debug)]
pub(crate) enum Reply {
    Started(String),
    Hover(HoverContent),
    Definition(Vec<Location>),
    Rename(crate::workspace_edit::PreparedRename),
    CodeActions(CodeActions),
    Completions(Vec<completion::Item>),
    CompletionEdit(completion::Edit),
}

#[derive(Debug)]
pub(crate) struct Event {
    pub id: u64,
    pub result: Result<Reply, String>,
}

enum Job {
    Request(Request),
    Keep(Vec<PathBuf>),
    Changed(Vec<(PathBuf, u8)>),
}

pub(crate) struct Client {
    jobs: SyncSender<Job>,
    cancel: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
}

impl Client {
    pub fn start(
        config: ServerConfig,
        root: PathBuf,
        notify: impl Fn(Event) + Send + 'static,
    ) -> Result<Self, String> {
        let (jobs, incoming) = mpsc::sync_channel(2);
        let cancel = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let cancelled = cancel.clone();
        let running = alive.clone();
        thread::Builder::new()
            .name("lsp-client".into())
            .spawn(move || {
                let mut server: Option<Session> = None;
                while !cancelled.load(Ordering::Relaxed) {
                    match incoming.recv_timeout(Duration::from_millis(100)) {
                        Ok(Job::Request(request)) => {
                            let result = (|| {
                                if server.is_none() {
                                    server =
                                        Some(Session::start(&config, &root, cancelled.clone())?);
                                }
                                server.as_mut().unwrap().execute(&config, &request)
                            })();
                            let failed = server.as_ref().is_some_and(|session| session.transport.has_failed());
                            notify(Event {
                                id: request.id,
                                result,
                            });
                            // Reconnect only after a broken transport, not an ordinary feature error.
                            if failed {
                                server = None;
                            }
                        }
                        Ok(Job::Changed(paths)) => {
                            if let Some(session) = server.as_mut() {
                                let changes: Result<Vec<_>,_> = paths.iter().map(|(path, kind)|
                                    file_uri(path).map(|uri| json!({"uri":uri,"type":kind}))).collect();
                                if changes.and_then(|changes| session.transport.notify(
                                    "workspace/didChangeWatchedFiles", json!({"changes":changes}))).is_err() {
                                    server = None;
                                }
                            }
                        }
                        Ok(Job::Keep(paths)) => {
                            if let Some(session) = server.as_mut() {
                                if session.keep(&paths).is_err() {
                                    server = None;
                                }
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if let Some(session) = server.as_ref() {
                                if session.transport.drain().is_err() {
                                    server = None;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                drop(server);
                running.store(false, Ordering::Release);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            jobs,
            cancel,
            alive,
        })
    }

    pub fn request(&self, request: Request) -> Result<(), String> {
        self.jobs
            .try_send(Job::Request(request))
            .map_err(|_| "LSP is busy; wait for the current request or use :lsp stop".into())
    }

    pub fn files_changed(&self, paths: Vec<(PathBuf, u8)>) -> bool {
        self.jobs.try_send(Job::Changed(paths)).is_ok()
    }

    pub fn keep(&self, paths: Vec<PathBuf>) -> bool {
        self.jobs.try_send(Job::Keep(paths)).is_ok()
    }
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

struct Session {
    transport: Transport,
    sync: u64,
    hover: bool,
    definition: bool,
    rename: bool,
    prepare_rename: bool,
    code_actions: bool,
    resolve_actions: bool,
    pull_diagnostics: bool,
    reports_readiness: bool,
    completion_triggers: Vec<String>,
    resolve_completion: bool,
    root: PathBuf,
    documents: HashMap<PathBuf, (String, i64)>,
}

impl Session {
    fn start(config: &ServerConfig, root: &Path, cancel: Arc<AtomicBool>) -> Result<Self, String> {
        let mut config = unity::server_config(config, root)?;
        crate::lsp_setup::configure_project(&mut config, root)?;
        let mut transport = Transport::start(&config, root, cancel)?;
        let root_uri = file_uri(root)?;
        let result = transport.request("initialize", json!({
            "processId":std::process::id(), "clientInfo":{"name":"Pötyi","version":env!("CARGO_PKG_VERSION")},
            "rootUri":root_uri, "workspaceFolders":[{"uri":root_uri,"name":"project"}],
            "capabilities":{
                "general":{"positionEncodings":["utf-16"]},
                "experimental":{"serverStatusNotification":true},
                "workspace":{"workspaceFolders":true,"configuration":true,"applyEdit":false,"workspaceEdit":{"documentChanges":true,"resourceOperations":["create","rename","delete"],"failureHandling":"transactional","changeAnnotationSupport":{"groupsOnLabel":true}}},
                "textDocument":{"completion":{"contextSupport":true,"completionItem":{"snippetSupport":false,"insertReplaceSupport":true,"resolveSupport":{"properties":["additionalTextEdits"]}}},"codeAction":{"dynamicRegistration":false,"codeActionLiteralSupport":{"codeActionKind":{"valueSet":["quickfix","refactor","refactor.extract","refactor.rewrite","source.organizeImports"]}},"isPreferredSupport":true,"honorsChangeAnnotations":true,"disabledSupport":true,"dataSupport":true,"resolveSupport":{"properties":["edit"]}},"publishDiagnostics":{"versionSupport":true,"dataSupport":true},"diagnostic":{"dynamicRegistration":false},"rename":{"prepareSupport":true},"hover":{"contentFormat":["plaintext"]},"definition":{"linkSupport":true},
                    "synchronization":{"dynamicRegistration":false,"didSave":false,"willSave":false}}
            },
            "initializationOptions":config.initialization_options
        }), Duration::from_secs(30))?;
        let capabilities = &result["capabilities"];
        if capabilities["positionEncoding"]
            .as_str()
            .is_some_and(|s| s != "utf-16")
        {
            return Err(
                "Server selected an unsupported position encoding (expected UTF-16)".into(),
            );
        }
        let sync = &capabilities["textDocumentSync"];
        if sync.is_object() && sync["openClose"] != Value::Bool(true) {
            return Err("Server does not support opening and closing document buffers".into());
        }
        let sync = sync
            .as_u64()
            .or_else(|| sync["change"].as_u64())
            .unwrap_or(0);
        if !matches!(sync, 1 | 2) {
            return Err("Server does not support document synchronization".into());
        }
        let supported = |value: &Value| value == &Value::Bool(true) || value.is_object();
        let hover = supported(&capabilities["hoverProvider"]);
        let definition = supported(&capabilities["definitionProvider"]);
        transport.mark_initialized();
        transport.notify("initialized", json!({}))?;
        Ok(Self {
            transport,
            sync,
            hover,
            definition,
            rename: supported(&capabilities["renameProvider"]),
            prepare_rename: capabilities["renameProvider"]["prepareProvider"] == true,
            code_actions: supported(&capabilities["codeActionProvider"]),
            resolve_actions: capabilities["codeActionProvider"]["resolveProvider"] == true,
            pull_diagnostics: capabilities["diagnosticProvider"].is_object(),
            completion_triggers: capabilities["completionProvider"]["triggerCharacters"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).take(32).collect()).unwrap_or_default(),
            resolve_completion: capabilities["completionProvider"]["resolveProvider"] == true,
            reports_readiness: result["serverInfo"]["name"].as_str() == Some("rust-analyzer"),
            root: root.to_path_buf(),
            documents: HashMap::new(),
        })
    }

    fn keep(&mut self, paths: &[PathBuf]) -> Result<(), String> {
        let closed: Vec<_> = self
            .documents
            .keys()
            .filter(|p| !paths.contains(p))
            .cloned()
            .collect();
        for path in closed {
            self.transport.notify(
                "textDocument/didClose",
                json!({"textDocument":{"uri":file_uri(&path)?}}),
            )?;
            self.transport.clear_diagnostics(&file_uri(&path)?);
            self.documents.remove(&path);
        }
        Ok(())
    }

    fn execute(&mut self, config: &ServerConfig, request: &Request) -> Result<Reply, String> {
        if (matches!(request.action, Action::Hover) && !self.hover)
            || (matches!(request.action, Action::Definition) && !self.definition)
            || (matches!(request.action, Action::Rename(_)) && !self.rename)
        {
            return Err("This language server does not support that command".into());
        }
        if let Action::Complete { cancel, .. } | Action::ResolveCompletion { cancel, .. } = &request.action {
            if cancel.load(Ordering::Relaxed) { return Err("Completion cancelled".into()); }
        }
        let focused = request.documents.first().ok_or("No document supplied")?;
        for doc in &request.documents {
            let uri = file_uri(&doc.path)?;
            if let Some((previous, version)) = self.documents.get_mut(&doc.path) {
                if previous != &doc.text {
                    *version += 1;
                    self.transport.clear_diagnostics(&uri);
                    let change = if self.sync == 2 {
                        incremental_change(previous, &doc.text)?
                    } else {
                        json!({"text":doc.text})
                    };
                    self.transport.notify(
                        "textDocument/didChange",
                        json!({
                            "textDocument":{"uri":uri,"version":version},"contentChanges":[change]
                        }),
                    )?;
                    *previous = doc.text.clone();
                }
            } else {
                self.transport.notify(
                    "textDocument/didOpen",
                    json!({"textDocument":{
                        "uri":uri,"languageId":config.language_id,"version":1,"text":doc.text
                    }}),
                )?;
                self.documents
                    .insert(doc.path.clone(), (doc.text.clone(), 1));
            }
        }
        if matches!(request.action, Action::Start) {
            if self.reports_readiness { self.transport.wait_until_ready(TIMEOUT)?; }
            return Ok(Reply::Started(format!("Connected to {}.\n{}\nProject: {}\nType a dot for member suggestions, or use :lsp hover / :lsp definition.",
                config.name, if self.reports_readiness { "Project loaded." } else { "Server initialized; project indexing may continue in the background." }, self.root.display())));
        }
        if matches!(request.action, Action::Complete { .. } | Action::ResolveCompletion { .. }) {
            return self.execute_completion(request);
        }
        if matches!(request.action, Action::CodeActions { .. } | Action::ResolveCodeAction { .. }) {
            return self.execute_code_action(request);
        }
        if let Action::Rename(name) = &request.action {
            let started = std::time::SystemTime::now();
            if self.reports_readiness { self.transport.wait_until_ready(TIMEOUT)?; }
            let mut params = json!({"textDocument":{"uri":file_uri(&focused.path)?},
                "position":position(&focused.text, request.cursor)?});
            if self.prepare_rename {
                let prepared = self.transport.request("textDocument/prepareRename", params.clone(), TIMEOUT)?;
                if prepared.is_null() { return Err("This symbol cannot be renamed".into()); }
            }
            params["newName"] = json!(name);
            let edit = self.transport.request("textDocument/rename", params, TIMEOUT)?;
            return crate::workspace_edit::prepare(&edit, &self.root, &request.documents,
                &self.documents, started).map(Reply::Rename);
        }
        let method = if matches!(request.action, Action::Hover) {
            "textDocument/hover"
        } else {
            "textDocument/definition"
        };
        let timeout = if matches!(request.action, Action::Hover) { HOVER_TIMEOUT } else { TIMEOUT };
        let result = self.transport.request(method, json!({
            "textDocument":{"uri":file_uri(&focused.path)?},"position":position(&focused.text, request.cursor)?
        }), timeout)?;
        if matches!(request.action, Action::Hover) {
            Ok(Reply::Hover(hover_content(&result)))
        } else {
            Ok(Reply::Definition(definitions(&result)?))
        }
    }
}

/// LSP positions count UTF-16 code units; editor positions count UTF-8 bytes.
pub(crate) fn position(text: &str, byte: usize) -> Result<Value, String> {
    let prefix = text.get(..byte).ok_or("Invalid cursor position")?;
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let tail = prefix.rsplit('\n').next().unwrap_or("");
    let character = tail.trim_end_matches('\r').encode_utf16().count();
    Ok(json!({"line":line,"character":character}))
}

pub(crate) fn utf16_column(text: &str, units: usize) -> Result<usize, String> {
    let mut used = 0;
    for (column, ch) in text.chars().enumerate() {
        if used == units {
            return Ok(column);
        }
        used += ch.len_utf16();
        if used > units {
            return Err("Definition position splits a Unicode character".into());
        }
    }
    if used == units {
        Ok(text.chars().count())
    } else {
        Err("Definition position is outside the line".into())
    }
}

fn incremental_change(old: &str, new: &str) -> Result<Value, String> {
    let mut start = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(start) || !new.is_char_boundary(start) {
        start -= 1;
    }
    // Do not split a CRLF line ending: LSP positions cannot address its middle.
    if start > 0
        && (old.as_bytes().get(start - 1) == Some(&b'\r')
            || new.as_bytes().get(start - 1) == Some(&b'\r'))
    {
        start -= 1;
    }
    let mut suffix = old[start..]
        .bytes()
        .rev()
        .zip(new[start..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    if suffix > 0
        && (old.as_bytes().get(old.len() - suffix) == Some(&b'\n')
            || new.as_bytes().get(new.len() - suffix) == Some(&b'\n'))
    {
        suffix -= 1;
    }
    Ok(
        json!({"range":{"start":position(old,start)?,"end":position(old,old.len()-suffix)?},
        "text":&new[start..new.len()-suffix]}),
    )
}

pub(crate) fn hover_content(result: &Value) -> HoverContent {
    fn append(value: &Value, output: &mut String, documentation: &mut Vec<std::ops::Range<usize>>) {
        if output.len() >= 16384 {
            return;
        }
        if let Some(text) = value.as_str().or_else(|| value["value"].as_str()) {
            if !output.is_empty() {
                output.push('\n');
            }
            let start = output.len();
            output.extend(
                text.chars()
                    .take(4096)
                    .filter(|c| !c.is_control() || *c == '\n' || *c == '\t'),
            );
            let rendered = &output[start..];
            if value["kind"].as_str() == Some("plaintext") {
                // Plaintext has no semantic spans. rust-analyzer separates its
                // signature blocks from documentation with two empty lines.
                if let Some(separator) = rendered.find("\n\n\n") {
                    documentation.push(start + separator + 3..output.len());
                }
            } else if value.get("language").is_none() {
                // MarkedString language blocks are code; prose is documentation.
                // Keep fenced Markdown examples in the normal text color too.
                let mut offset = start;
                let mut fenced = false;
                for line in rendered.split_inclusive('\n') {
                    if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
                        fenced = !fenced;
                    } else if !fenced {
                        if let Some(last) = documentation.last_mut() && last.end == offset {
                            last.end += line.len();
                        } else {
                            documentation.push(offset..offset + line.len());
                        }
                    }
                    offset += line.len();
                }
            }
        } else if let Some(items) = value.as_array() {
            for item in items {
                append(item, output, documentation);
            }
        }
    }
    let mut text = String::new();
    let mut documentation = Vec::new();
    append(&result["contents"], &mut text, &mut documentation);
    if text.trim().is_empty() {
        text = "No hover information at the cursor".into();
        documentation.clear();
    }
    HoverContent { text, documentation }
}

#[cfg(test)]
fn hover_text(result: &Value) -> String {
    hover_content(result).text
}

fn definitions(result: &Value) -> Result<Vec<Location>, String> {
    if result.is_null() {
        return Ok(Vec::new());
    }
    let items = result
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(std::slice::from_ref(result));
    items
        .iter()
        .take(32)
        .map(|item| {
            let (uri, start) = if item.get("targetUri").is_some() {
                (&item["targetUri"], &item["targetSelectionRange"]["start"])
            } else {
                (&item["uri"], &item["range"]["start"])
            };
            let number = |value: &Value| {
                value
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|n| *n <= i32::MAX as usize)
                    .ok_or_else(|| "Invalid definition position".to_string())
            };
            Ok(Location {
                path: uri_path(uri.as_str().ok_or("Missing definition URI")?)?,
                line: number(&start["line"])?,
                character: number(&start["character"])?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
