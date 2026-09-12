// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

//! Explicit LSP navigation and reviewed workspace text edits; no server commands.
use crate::{CommandOutcome, CursorState, Editor, command_bar::CommandBar, file_is_open_in, lsp};
use sdl3::EventSubsystem;
use std::collections::HashMap;
use std::path::PathBuf;

struct Running {
    config: lsp::ServerConfig,
    client: lsp::Client,
    paths: Vec<PathBuf>,
}

#[derive(Clone)]
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
    pending_anchor: Option<usize>,
    actions: Option<(Pending, usize, lsp::CodeActions)>,
    preview: Option<(u64, std::sync::Arc<crate::workspace_edit::PreparedRename>)>,
    hover_refresh: Option<Pending>,
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
            pending_anchor: None,
            actions: None,
            preview: None,
            hover_refresh: None,
            next_id: 1,
            back: Vec::new(),
            last_paths: Vec::new(),
        }
    }

    pub fn stop(&mut self) {
        self.pending = None;
        self.pending_anchor = None;
        self.actions = None;
        self.hover_refresh = None;
        self.sessions.clear();
        self.preview = None;
    }

    /// Clicks are explicit hover requests. While a server is busy, retain only
    /// the latest target metadata; snapshot its text when the previous request ends.
    pub fn document_clicked(&mut self, editor: &Editor, other: &Editor, bar: &mut CommandBar) {
        if !bar.is_active() || !bar.is_info()
            || !matches!(bar.parse(), Ok(crate::command_bar::ParsedCommand::Hover))
        {
            self.hover_refresh = None;
            if bar.is_active() && bar.is_info()
                && matches!(bar.parse(), Ok(crate::command_bar::ParsedCommand::Definition))
                && self.pending.as_ref().is_some_and(|p| !p.matches(editor, other, bar))
            {
                bar.show_info("Cursor moved. Press Enter to request a definition here.");
            }
            return;
        }
        if self.pending.is_some() {
            self.hover_refresh = Some(Pending {
                id: 0, revision: editor.document.revision(),
                other_revision: other.document.revision(),
                cursor: editor.document.cursor.position, path: editor.path.clone(), epoch: bar.epoch(),
            });
            bar.show_info("Waiting for hover at the clicked word…\nClick another word to change the target; Escape closes.");
        } else if let Err(error) = self.request(lsp::Action::Hover, editor, other, bar) {
            bar.show_info(&error);
        }
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
        self.hover_refresh = None;
        self.preview = None;
        self.actions = None;
        self.pending_anchor = Some(editor.document.cursor.anchor);
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
        self.hover_refresh = None;
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
        if let Some(refresh) = self.hover_refresh.take() {
            if refresh.matches(editor, other, bar)
                && matches!(bar.parse(), Ok(crate::command_bar::ParsedCommand::Hover))
            {
                if let Err(error) = self.request(lsp::Action::Hover, editor, other, bar) {
                    bar.show_info(&error);
                }
            }
            return outcome;
        }
        if self.pending_anchor.take().is_some_and(|anchor| anchor != editor.document.cursor.anchor)
            || !pending.matches(editor, other, bar) {
            return outcome;
        }
        match event.result {
            Err(error) => bar.show_info(&format!("LSP: {error}")),
            Ok(lsp::Reply::CodeActions(actions)) => {
                if actions.items.is_empty() {
                    bar.show_info("No code actions here. Place the cursor on the problem or select the type/module to refactor.");
                } else {
                    show_actions(bar, &actions);
                    self.actions = Some((pending, editor.document.cursor.anchor, actions));
                }
            }
            Ok(lsp::Reply::Rename(preview)) => {
                bar.show_review(preview.choices());
                self.preview = Some((bar.epoch(), std::sync::Arc::new(preview)));
            }
            Ok(lsp::Reply::Hover(text)) => {
                bar.show_hover(&text);
                bar.set_status("Click another word to inspect it; Up/Down scroll; Escape closes");
            }
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

    pub fn files_changed(&mut self, paths: Vec<(PathBuf, u8)>) {
        if paths.is_empty() { return; }
        self.pending = None;
        self.hover_refresh = None;
        self.sessions.retain(|(_,root), session| {
            let changed: Vec<_> = paths.iter().filter(|(p, _)| p.starts_with(root)).cloned().collect();
            // Reconnect on demand if the bounded queue cannot accept invalidation.
            changed.is_empty() || session.client.files_changed(changed)
        });
    }

    pub fn discard_dismissed_preview(&mut self, bar: &CommandBar) {
        if self.actions.as_ref().is_some_and(|(p,_,_)| !bar.is_active() || p.epoch != bar.epoch()) {
            self.actions = None;
        }
        if self.preview.as_ref().is_some_and(|(epoch,_)| !bar.is_active() || *epoch != bar.epoch()) {
            self.preview = None;
        }
    }

    /// Returns Some only while Enter/click belongs to an existing rename preview.
    pub fn review(&mut self, editor: &mut Editor, other: &mut Editor, bar: &mut CommandBar)
        -> Option<Result<bool,String>> {
        self.discard_dismissed_preview(bar);
        if let Some((guard, anchor, actions)) = self.actions.as_ref() {
            if !guard.matches(editor, other, bar) || *anchor != editor.document.cursor.anchor {
                self.actions = None;
                bar.show_info("The selection or document changed. Run :actions again.");
                return Some(Ok(false));
            }
            if bar.is_info() { show_actions(bar, actions); return Some(Ok(false)); }
            let selected = bar.review_selection()?;
            if selected >= actions.items.len() { self.actions = None; bar.close(); return Some(Ok(false)); }
            let item = &actions.items[selected];
            if let Some(reason) = &item.disabled {
                bar.show_info(reason); bar.set_status("Enter returns to actions; Escape closes");
                return Some(Ok(false));
            }
            let action = lsp::Action::ResolveCodeAction { action: item.value.clone(), started: actions.started };
            return Some(self.request(action, editor, other, bar).map(|_| false));
        }
        let (_, preview) = self.preview.as_ref()?;
        if bar.is_info() {
            bar.show_review(preview.choices());
            return Some(Ok(false));
        }
        let selected = bar.review_selection()?;
        if selected < preview.files.len() {
            bar.show_info(&preview.files[selected].preview);
            bar.set_status("Up/Down scroll; Enter returns to files; Escape cancels changes");
            return Some(Ok(false));
        }
        let apply = selected == preview.files.len();
        let (_, preview) = self.preview.take().unwrap();
        if !apply { bar.close(); return Some(Ok(false)); }
        Some(crate::workspace_edit::apply(preview, editor, other).map(|_| {
            self.files_changed(crate::workspace_edit::recent_disk_changes(editor, false));
            bar.close(); true
        }))
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
            target.restore_cursor(bookmark.cursor.clone());
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

fn show_actions(bar: &mut CommandBar, actions: &lsp::CodeActions) {
    let mut choices: Vec<_> = actions.items.iter().map(|a| {
        if a.disabled.is_some() { format!("{} (unavailable)", a.title) } else { a.title.clone() }
    }).collect();
    choices.push("Cancel".into());
    bar.show_review(choices);
    bar.set_status("Click or Enter to preview an action; Escape cancels");
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
        target.clear_secondary_cursors();
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
