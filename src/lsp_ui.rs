// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

//! Editor integration. LSP cannot mutate documents or execute server commands.
use crate::{CommandOutcome, CursorState, Editor, command_bar::CommandBar, file_is_open_in, lsp};
use sdl3::EventSubsystem;
use std::collections::HashMap;
use std::path::PathBuf;

struct Running {
    config: lsp::ServerConfig,
    client: lsp::Client,
    paths: Vec<PathBuf>,
}

struct Pending {
    id: u64,
    revision: u64,
    other_revision: u64,
    path: Option<PathBuf>,
    cursor: usize,
    epoch: u64,
}

struct Bookmark {
    path: PathBuf,
    cursor: CursorState,
    revision: u64,
}

pub(crate) struct LspUi {
    events: EventSubsystem,
    sessions: HashMap<(String, PathBuf), Running>,
    pending: Option<Pending>,
    next_id: u64,
    back: Vec<Bookmark>,
    last_paths: Vec<Option<PathBuf>>,
}

impl LspUi {
    pub fn new(events: EventSubsystem) -> Self {
        Self {
            events,
            sessions: HashMap::new(),
            pending: None,
            next_id: 1,
            back: Vec::new(),
            last_paths: Vec::new(),
        }
    }

    pub fn stop(&mut self) {
        self.pending = None;
        self.sessions.clear();
    }

    pub fn status(&self, editor: &Editor) -> String {
        match lsp::Config::load() {
            Err(error) => error,
            Ok(config) => {
                let name = editor
                    .path
                    .as_deref()
                    .and_then(|p| config.server_for(p))
                    .map(|s| s.name.as_str())
                    .unwrap_or("none for this file");
                format!(
                    "LSP {}\nConfigured server: {name}\n{} background session(s)\nUse :hover or :definition to start.\nUse :lsp-stop after configuration changes.\nSynchronization happens on explicit requests.",
                    if config.enabled {
                        "enabled"
                    } else {
                        "disabled; set enabled = true in config/lsp.toml"
                    },
                    self.sessions
                        .values()
                        .filter(|s| s.client.is_alive())
                        .count()
                )
            }
        }
    }

    pub fn request(
        &mut self,
        action: lsp::Action,
        editor: &Editor,
        other: &Editor,
        bar: &mut CommandBar,
    ) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("LSP request in progress; use :lsp-stop to cancel".into());
        }
        let config = lsp::Config::load()?;
        if !config.enabled {
            self.stop();
            return Err("LSP is disabled. Set enabled = true in config/lsp.toml".into());
        }
        let path = editor
            .path
            .as_deref()
            .ok_or("Save this document before using LSP")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let server = config
            .server_for(&path)
            .ok_or("No language server configured for this file extension")?
            .clone();
        let root = lsp::project_root(&path, &server);
        let mut documents = vec![snapshot(editor, path.clone())?];
        if let Some(other_path) = other.path.as_deref() {
            if let Ok(other_path) = other_path.canonicalize() {
                if other_path != path
                    && config
                        .server_for(&other_path)
                        .is_some_and(|s| s.name == server.name)
                    && lsp::project_root(&other_path, &server) == root
                {
                    documents.push(snapshot(other, other_path)?);
                }
            }
        }
        let paths = documents.iter().map(|d| d.path.clone()).collect();
        let key = (server.name.clone(), root.clone());
        if !self.sessions.contains_key(&key) {
            if self.sessions.len() >= 2 {
                return Err("Two project sessions are already active; use :lsp-stop first".into());
            }
            let sender = self.events.event_sender();
            let client = lsp::Client::start(server.clone(), root, move |event| {
                let _ = sender.push_custom_event(event);
            })?;
            self.sessions.insert(
                key.clone(),
                Running {
                    config: server,
                    client,
                    paths: Vec::new(),
                },
            );
        }
        let running = self.sessions.get_mut(&key).unwrap();
        let id = self.next_id;
        self.next_id += 1;
        running.client.request(lsp::Request {
            id,
            action,
            documents,
            cursor: editor.document.cursor.position,
        })?;
        running.paths = paths;
        self.last_paths = vec![editor.path.clone(), other.path.clone()];
        self.last_paths.sort();
        self.pending = Some(Pending {
            id,
            revision: editor.document.revision(),
            other_revision: other.document.revision(),
            cursor: editor.document.cursor.position,
            path: editor.path.clone(),
            epoch: bar.epoch(),
        });
        bar.show_info(
            "Waiting for the language server…\nEscape dismisses the result; :lsp-stop cancels.",
        );
        Ok(())
    }

    /// Run after events. Compare path strings first; disabled LSP performs no
    /// file reads, document copies, worker startup, or timer work.
    pub fn reconcile(&mut self, editor: &Editor, other: &Editor) {
        if self.sessions.is_empty() {
            return;
        }
        let mut paths = vec![editor.path.clone(), other.path.clone()];
        paths.sort();
        if paths == self.last_paths {
            return;
        }
        self.pending = None;
        let canonical: Vec<_> = paths
            .iter()
            .flatten()
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        let mut complete = true;
        self.sessions.retain(|(_, root), session| {
            let keep: Vec<_> = canonical
                .iter()
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|extension| {
                            session
                                .config
                                .extensions
                                .iter()
                                .any(|e| e.eq_ignore_ascii_case(extension))
                        })
                        && lsp::project_root(p, &session.config) == *root
                })
                .cloned()
                .collect();
            if keep.is_empty() {
                return false;
            }
            if keep != session.paths {
                if session.client.keep(keep.clone()) {
                    session.paths = keep;
                } else {
                    complete = false;
                }
            }
            true
        });
        if complete {
            self.last_paths = paths;
        }
    }

    pub fn accept(
        &mut self,
        event: lsp::Event,
        editor: &mut Editor,
        other: &mut Editor,
        bar: &mut CommandBar,
    ) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        if self.pending.as_ref().is_none_or(|p| p.id != event.id) {
            return outcome;
        }
        let pending = self.pending.take().unwrap();
        if !pending.matches(editor, other, bar) {
            return outcome;
        }
        match event.result {
            Err(error) => bar.show_info(&format!("LSP: {error}")),
            Ok(lsp::Reply::Hover(text)) => bar.show_info(&text),
            Ok(lsp::Reply::Definition(locations)) => {
                if let Some(location) = locations.first() {
                    let bookmark = Bookmark {
                        path: editor.path.clone().unwrap(),
                        cursor: editor.cursor_state(),
                        revision: editor.document.revision(),
                    };
                    match navigate(location, editor, other) {
                        Ok(result) => {
                            if self.back.len() == 32 {
                                self.back.remove(0);
                            }
                            self.back.push(bookmark);
                            outcome = result;
                            bar.close();
                        }
                        Err(error) => bar.show_info(&error),
                    }
                } else {
                    bar.show_info("No definition found at the cursor");
                }
            }
        }
        outcome
    }

    pub fn go_back(
        &mut self,
        editor: &mut Editor,
        other: &mut Editor,
    ) -> Result<CommandOutcome, String> {
        let bookmark = self
            .back
            .last()
            .ok_or("No definition jump to return from")?;
        let location = lsp::Location {
            path: bookmark.path.clone(),
            line: bookmark.cursor.line,
            character: 0,
        };
        let outcome = navigate(&location, editor, other)?;
        let target = if outcome.focus_other { other } else { editor };
        if target.document.revision() == bookmark.revision {
            target.restore_cursor(bookmark.cursor);
        } else {
            target
                .document
                .move_cursor_to_line_column(bookmark.cursor.line, bookmark.cursor.column)
                .map_err(|e| e.to_string())?;
        }
        self.back.pop();
        Ok(outcome)
    }
}

impl Pending {
    fn matches(&self, editor: &Editor, other: &Editor, bar: &CommandBar) -> bool {
        bar.is_active()
            && bar.epoch() == self.epoch
            && editor.path == self.path
            && editor.document.revision() == self.revision
            && other.document.revision() == self.other_revision
            && editor.document.cursor.position == self.cursor
    }
}

fn snapshot(editor: &Editor, path: PathBuf) -> Result<lsp::Document, String> {
    if editor.document.len() > lsp::MAX_DOCUMENT_BYTES {
        return Err("LSP is limited to documents up to 2 MiB; editing remains available".into());
    }
    let bytes = editor
        .document
        .read_range(0, editor.document.len())
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8(bytes).map_err(|_| "LSP requires UTF-8 text")?;
    Ok(lsp::Document { path, text })
}

/// Validate a destination before replacing any pane, and reuse open buffers
/// so unsaved changes, read-only state and undo history survive navigation.
fn navigate(
    location: &lsp::Location,
    editor: &mut Editor,
    other: &mut Editor,
) -> Result<CommandOutcome, String> {
    let path = location
        .path
        .to_str()
        .ok_or("Definition path is not valid Unicode")?;
    let current = file_is_open_in(path, editor);
    let in_other = file_is_open_in(path, other);
    let focus_other = !current && (in_other || editor.dirty);
    let target = if focus_other { other } else { editor };
    let reloaded = !current && !in_other;
    let move_to = |target: &mut Editor| -> Result<(), String> {
        target
            .document
            .ensure_line_cached(location.line)
            .map_err(|e| e.to_string())?;
        if location.line >= target.document.cached_line_count() {
            return Err("Definition line is outside the document".into());
        }
        let text = target
            .document
            .line_text(location.line)
            .map_err(|e| e.to_string())?;
        let column = lsp::utf16_column(&text, location.character)?;
        target
            .document
            .move_cursor_to_line_column(location.line, column)
            .map_err(|e| e.to_string())
    };
    if reloaded {
        if target.dirty {
            return Err(
                "Both panes have unsaved changes. Save one before opening this definition.".into(),
            );
        }
        let mut staged = Editor::new(target.config.clone()).map_err(|e| e.to_string())?;
        staged.open(path).map_err(|e| e.to_string())?;
        move_to(&mut staged)?;
        *target = staged;
    } else {
        move_to(target)?;
    }
    Ok(CommandOutcome {
        focus_other,
        document_reloaded: reloaded,
        path_changed: true,
        cursor_changed: true,
        ..CommandOutcome::default()
    })
}

#[cfg(test)]
mod tests;
