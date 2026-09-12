// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

//! Bounded, explicit workspace text edits. Images live on disk, not in undo RAM.
use crate::{CursorState, Editor, HistoryEntry, HistoryKind, lsp};
mod resources;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::SystemTime,
};

const MAX_FILES: usize = 64;
const MAX_EDITS: usize = 10_000;
const MAX_TOTAL: usize = 16 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
struct Scratch(PathBuf, AtomicBool);
impl Drop for Scratch {
    fn drop(&mut self) {
        if !self.1.load(Ordering::Relaxed) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
impl Scratch {
    fn new() -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!(
            "potyi-rename-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(|e| e.to_string())?;
        Ok(Self(path, AtomicBool::new(false)))
    }
}

#[derive(Debug)]
pub(crate) struct FileChange {
    pub path: PathBuf,
    before: PathBuf,
    after: PathBuf,
    open: bool,
    edits: Vec<(Range<usize>, usize)>,
    pub preview: String,
    operation: Option<String>,
}
#[derive(Debug)]
pub(crate) struct PreparedRename {
    pub files: Vec<FileChange>,
    _scratch: Scratch,
    root: PathBuf,
    resources: Option<resources::Plan>,
}
impl PreparedRename {
    pub fn choices(&self) -> Vec<String> {
        let mut rows: Vec<_> = self
            .files
            .iter()
            .map(|f| {
                if let Some(operation) = &f.operation { return operation.clone(); }
                format!(
                    "{} · {} change(s) · {}",
                    f.path.strip_prefix(&self.root).unwrap_or(&f.path).display(),
                    f.edits.len(),
                    if f.open {
                        "unsaved buffer"
                    } else {
                        "writes file"
                    }
                )
            })
            .collect();
        rows.push(format!("Apply changes to {} file(s)", self.files.len()));
        rows.push("Cancel changes".into());
        rows
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|f| {
            f.take((lsp::MAX_DOCUMENT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
        })
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > lsp::MAX_DOCUMENT_BYTES {
        return Err("Rename is limited to 2 MiB per file".into());
    }
    Ok(bytes)
}
pub(crate) fn byte_position(text: &str, position: &Value) -> Result<usize, String> {
    let line = position["line"].as_u64().ok_or("Invalid edit line")?;
    let units = position["character"]
        .as_u64()
        .ok_or("Invalid edit column")?;
    let mut offset = 0;
    let mut lines = text.split('\n');
    for _ in 0..line {
        offset += lines
            .next()
            .ok_or("Edit line is outside the document")?
            .len()
            + 1;
    }
    let text = lines
        .next()
        .ok_or("Edit line is outside the document")?
        .trim_end_matches('\r');
    let mut used = 0;
    for (byte, ch) in text.char_indices() {
        if used == units {
            return Ok(offset + byte);
        }
        used += ch.len_utf16() as u64;
        if used > units {
            return Err("Edit splits a Unicode character".into());
        }
    }
    if used == units {
        Ok(offset + text.len())
    } else {
        Err("Edit column is outside the line".into())
    }
}

fn annotation_notes(edit: &Value) -> Result<String, String> {
    let Some(raw) = edit.get("changeAnnotations") else { return Ok(String::new()); };
    let annotations = raw.as_object().ok_or("Invalid change annotations")?;
    if annotations.len() > 64 { return Err("Too many change annotations".into()); }
    let mut notes = String::new();
    for annotation in annotations.values() {
        let label = annotation["label"].as_str().ok_or("Missing annotation label")?;
        notes.push_str(&format!("\nServer note: {}", label.chars().take(240).collect::<String>()));
        if let Some(description) = annotation["description"].as_str() {
            notes.push_str(&format!(" — {}", description.chars().take(500).collect::<String>()));
        }
        if annotation["needsConfirmation"] == true { notes.push_str(" (review before applying)"); }
    }
    Ok(notes)
}

fn validate_edit_metadata(item: &Value, workspace: &Value) -> Result<(), String> {
    if let Some(id) = item.get("annotationId") {
        let id = id.as_str().ok_or("Invalid annotation ID")?;
        if workspace["changeAnnotations"].get(id).is_none() { return Err("Unknown change annotation".into()); }
    }
    if item.get("insertTextFormat").is_some_and(|v| v.as_u64() != Some(1)) {
        return Err("Snippet edits are not supported".into());
    }
    Ok(())
}

pub(crate) fn prepare(
    edit: &Value,
    root: &Path,
    documents: &[lsp::Document],
    versions: &HashMap<PathBuf, (String, i64)>,
    started: SystemTime,
) -> Result<PreparedRename, String> {
    if edit["documentChanges"].as_array().is_some_and(|changes| changes.iter().any(|c| c.get("kind").is_some())) {
        return resources::prepare(edit, root, documents, versions, started);
    }
    if edit.is_null() {
        return Err("No rename changes returned for this symbol".into());
    }
    if !edit.is_object() {
        return Err("Invalid workspace edit".into());
    }
    let notes = annotation_notes(edit)?;
    let mut raw: Vec<(String, Option<i64>, &Vec<Value>)> = Vec::new();
    if let Some(changes) = edit.get("documentChanges") {
        if edit.get("changes").is_some() {
            return Err("Ambiguous workspace edit".into());
        }
        for change in changes.as_array().ok_or("Invalid documentChanges")? {
            let doc = &change["textDocument"];
            let version = match doc.get("version") {
                None | Some(Value::Null) => None,
                Some(v) => Some(v.as_i64().ok_or("Invalid document version")?),
            };
            raw.push((
                doc["uri"].as_str().ok_or("Missing edit URI")?.into(),
                version,
                change["edits"].as_array().ok_or("Missing text edits")?,
            ));
        }
    } else if let Some(changes) = edit.get("changes") {
        for (uri, edits) in changes.as_object().ok_or("Invalid changes")? {
            raw.push((
                uri.clone(),
                None,
                edits.as_array().ok_or("Invalid text edits")?,
            ));
        }
    }
    if raw.len() > MAX_FILES {
        return Err("Rename exceeds the 64-file limit".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let scratch = Scratch::new()?;
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut total = 0;
    let mut count = 0;
    for (uri, version, edits) in raw {
        let path = lsp::uri_path(&uri)?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err("Rename targets a file outside this project".into());
        }
        if !seen.insert(path.clone()) {
            return Err("Repeated document edits are not supported".into());
        }
        let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.permissions().readonly() {
            return Err(format!(
                "Cannot rename in read-only or non-regular file: {}",
                path.display()
            ));
        }
        if let Some(v) = version {
            if versions
                .iter()
                .find(|(p, _)| *p == &path || p.canonicalize().ok().as_ref() == Some(&path))
                .is_none_or(|(_, (_, current))| *current != v)
            {
                return Err("Rename has a stale document version; request it again".into());
            }
        }
        let open = documents
            .iter()
            .find(|d| d.path == path || d.path.canonicalize().ok().as_ref() == Some(&path));
        if open.is_none() && metadata.modified().map_err(|e| e.to_string())? > started {
            return Err("A file changed while rename was running; request it again".into());
        }
        let before = match open {
            Some(doc) => doc.text.clone(),
            None => String::from_utf8(read(&path)?).map_err(|_| "Rename requires UTF-8 files")?,
        };
        if before.len() > lsp::MAX_DOCUMENT_BYTES {
            return Err("Rename file exceeds 2 MiB".into());
        }
        let mut replacements = Vec::new();
        count += edits.len();
        if count > MAX_EDITS {
            return Err("Rename exceeds 10,000 text edits".into());
        }
        for item in edits {
            validate_edit_metadata(item, edit)?;
            let start = byte_position(&before, &item["range"]["start"])?;
            let end = byte_position(&before, &item["range"]["end"])?;
            if start > end {
                return Err("Reversed edit range".into());
            }
            let replacement = item["newText"].as_str().ok_or("Missing replacement text")?;
            replacements.push((start..end, replacement));
        }
        replacements.sort_by_key(|(r, _)| (r.start, r.end));
        if replacements
            .windows(2)
            .any(|w| w[0].0.end > w[1].0.start || w[0].0.start == w[1].0.start)
        {
            return Err("Overlapping rename edits are not supported".into());
        }
        let mut after = String::new();
        let mut offset = 0;
        let mut preview = format!(
            "{}\n{}\n",
            path.display(),
            if open.is_some() {
                "Updates the open buffer; save when ready."
            } else {
                "Apply writes this file to disk."
            }
        );
        preview.push_str(&notes);
        for (index, (range, replacement)) in replacements.iter().enumerate() {
            if after.len() + range.start - offset + replacement.len() > lsp::MAX_DOCUMENT_BYTES {
                return Err("Renamed file exceeds 2 MiB".into());
            }
            after.push_str(&before[offset..range.start]);
            after.push_str(replacement);
            offset = range.end;
            if index < 40 {
                let position = lsp::position(&before, range.start)?;
                let old: String = before[range.clone()].chars().take(160).collect();
                let new: String = replacement.chars().take(160).collect();
                preview.push_str(&format!(
                    "\nLine {}, column {}\n- {old}\n+ {new}\n",
                    position["line"].as_u64().unwrap() + 1,
                    position["character"].as_u64().unwrap() + 1
                ));
            }
        }
        if after.len() + before.len() - offset > lsp::MAX_DOCUMENT_BYTES {
            return Err("Renamed file exceeds 2 MiB".into());
        }
        after.push_str(&before[offset..]);
        if after == before {
            continue;
        }
        total += before.len() + after.len();
        if total > MAX_TOTAL {
            return Err("Rename exceeds the 16 MiB total limit".into());
        }
        if replacements.len() > 40 {
            preview.push_str("\nPreview shows the first 40 edits only.\n");
        }
        let before_path = scratch.0.join(format!("{}.before", files.len()));
        let after_path = scratch.0.join(format!("{}.after", files.len()));
        fs::write(&before_path, before)
            .and_then(|_| fs::write(&after_path, after))
            .map_err(|e| e.to_string())?;
        files.push(FileChange {
            operation: None,
            path,
            before: before_path,
            after: after_path,
            open: open.is_some(),
            edits: replacements
                .iter()
                .map(|(r, t)| (r.clone(), t.len()))
                .collect(),
            preview,
        });
    }
    if files.is_empty() {
        return Err("No rename changes returned for this symbol".into());
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(PreparedRename {
        files,
        _scratch: scratch,
        root,
        resources: None,
    })
}

pub(crate) struct Marker {
    record: Arc<PreparedRename>,
    local: Option<crate::PieceTableSnapshot>,
    resource: Option<resources::BufferHistory>,
}
fn marker(entry: &HistoryEntry) -> Option<&Marker> {
    match &entry.kind {
        HistoryKind::Workspace(m) => Some(m),
        _ => None,
    }
}
fn matching(editor: &Editor, file: &FileChange) -> bool {
    editor
        .path
        .as_ref()
        .and_then(|p| p.canonicalize().ok())
        .as_ref()
        == Some(&file.path)
}
fn content(editor: &Editor) -> Result<Vec<u8>, String> {
    if editor.document.len() > lsp::MAX_DOCUMENT_BYTES {
        return Err("Buffer changed since rename; request it again".into());
    }
    editor
        .document
        .read_range(0, editor.document.len())
        .map_err(|e| e.to_string())
}
fn mapped_position(position: usize, edits: &[(Range<usize>, usize)]) -> usize {
    let mut delta = 0isize;
    for (r, len) in edits {
        if position < r.start {
            break;
        }
        if position <= r.end {
            return r.start.saturating_add_signed(delta) + (position - r.start).min(*len);
        }
        delta += *len as isize - r.len() as isize;
    }
    position.saturating_add_signed(delta)
}
fn cursor_at(text: &[u8], position: usize) -> CursorState {
    let text = std::str::from_utf8(text).expect("validated UTF-8");
    let mut position = position.min(text.len());
    while !text.is_char_boundary(position) {
        position -= 1;
    }
    let prefix = &text[..position];
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let column = prefix.rsplit('\n').next().unwrap_or("").chars().count();
    CursorState {
        secondary_cursors: Vec::new(),
        position,
        line,
        column,
        desired_column: None,
        anchor: position,
        anchor_line: line,
        anchor_column: column,
    }
}

/// A same-directory replacement is fully written before touching its target.
struct DiskWrite {
    path: PathBuf,
    temp: PathBuf,
    old: Vec<u8>,
    new: Vec<u8>,
}
impl Drop for DiskWrite {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.temp);
    }
}
impl DiskWrite {
    fn stage(path: &Path, old: Vec<u8>, new: Vec<u8>) -> Result<Self, String> {
        let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
        if metadata.permissions().readonly() {
            return Err(format!("Read-only file: {}", path.display()));
        }
        if read(path)? != old {
            return Err(format!(
                "File changed: {}. Request rename again.",
                path.display()
            ));
        }
        let temp = path.with_file_name(format!(
            ".potyi-rename-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&temp).map_err(|e| e.to_string())?;
        let result = Self {
            path: path.into(),
            temp,
            old,
            new,
        };
        output
            .write_all(&result.new)
            .and_then(|_| output.set_permissions(metadata.permissions()))
            .and_then(|_| output.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(result)
    }
    fn commit(&self) -> Result<(), String> {
        if self.path.canonicalize().map_err(|e| e.to_string())? != self.path {
            return Err(format!("File path changed: {}", self.path.display()));
        }
        if read(&self.path)? != self.old {
            return Err(format!("File changed: {}", self.path.display()));
        }
        fs::rename(&self.temp, &self.path).map_err(|e| e.to_string())
    }
}
fn commit_disk(writes: &[DiskWrite], record: &Arc<PreparedRename>) -> Result<(), String> {
    for (index, write) in writes.iter().enumerate() {
        if let Err(mut error) = write.commit() {
            for previous in writes[..index].iter().rev() {
                if let Err(rollback) =
                    DiskWrite::stage(&previous.path, previous.new.clone(), previous.old.clone())
                        .and_then(|w| w.commit())
                {
                    // Keep recovery images if an external modification or I/O failure prevents rollback.
                    let backup = record._scratch.0.clone();
                    record._scratch.1.store(true, Ordering::Relaxed);
                    error.push_str(&format!(
                        "; rollback failed: {rollback}. Recovery images: {}",
                        backup.display()
                    ));
                }
            }
            return Err(error);
        }
    }
    Ok(())
}

pub(crate) fn apply(
    record: Arc<PreparedRename>,
    editor: &mut Editor,
    other: &mut Editor,
) -> Result<(), String> {
    if record.resources.is_some() { return resources::apply(record, editor, other); }
    let mut writes = Vec::new();
    for file in &record.files {
        let expected = read(&file.before)?;
        let mut opened = false;
        for pane in [&*editor, &*other] {
            if matching(pane, file) {
                opened = true;
                if pane.read_only || content(pane)? != expected {
                    return Err(format!(
                        "Buffer changed or is read-only: {}",
                        file.path.display()
                    ));
                }
            }
        }
        if opened != file.open {
            return Err("Open files changed during the preview; request rename again".into());
        }
        if !opened {
            writes.push(DiskWrite::stage(&file.path, expected, read(&file.after)?)?);
        }
    }
    // Stage piece layouts, immediately swapping back. Failure leaves every buffer untouched.
    let mut staged = Vec::new();
    for (index, pane) in [&mut *editor, &mut *other].into_iter().enumerate() {
        if let Some(file) = record.files.iter().find(|f| matching(pane, f)) {
            let bytes = read(&file.after)?;
            let before = pane.cursor_state();
            let after = cursor_at(&bytes, mapped_position(before.position, &file.edits));
            let mut snapshot = pane
                .document
                .replace_from_reader(&mut std::io::Cursor::new(bytes), lsp::MAX_DOCUMENT_BYTES)
                .map_err(|e| e.to_string())?
                .ok_or("Rename produced no buffer changes")?;
            pane.document.swap_snapshot(&mut snapshot);
            staged.push((index, snapshot, before, after));
        }
    }
    commit_disk(&writes, &record)?;
    let anchor = !staged.iter().any(|(i, ..)| *i == 0);
    for (index, mut snapshot, before, after) in staged {
        let pane = if index == 0 {
            &mut *editor
        } else {
            &mut *other
        };
        pane.document.swap_snapshot(&mut snapshot);
        pane.restore_cursor(after.clone());
        pane.multi_edit_group = None;
        pane.undo_stack.push(HistoryEntry {
            kind: HistoryKind::Workspace(Marker {
                record: record.clone(),
                local: Some(snapshot),
                resource: None,
            }),
            before,
            after,
        });
        pane.redo_stack.clear();
        pane.dirty = true;
    }
    if anchor {
        let cursor = editor.cursor_state();
        editor.multi_edit_group = None;
        editor.undo_stack.push(HistoryEntry {
            kind: HistoryKind::Workspace(Marker {
                record,
                local: None,
                resource: None,
            }),
            before: cursor.clone(),
            after: cursor,
        });
        editor.redo_stack.clear();
    }
    Ok(())
}

pub(crate) fn recent_disk_changes(editor: &Editor, undone: bool) -> Vec<(PathBuf, u8)> {
    let stack = if undone {
        &editor.redo_stack
    } else {
        &editor.undo_stack
    };
    stack
        .last()
        .and_then(marker)
        .map(|m| {
            if let Some(plan) = &m.record.resources { return plan.paths(undone); }
            m.record
                .files
                .iter()
                .filter(|f| !f.open)
                .map(|f| (f.path.clone(), 2))
                .collect()
        })
        .unwrap_or_default()
}

/// Coordinate a workspace entry before normal per-pane undo/redo handling.
pub(crate) fn history(editor: &mut Editor, other: &mut Editor, redo: bool) -> Result<bool, String> {
    let stack = if redo {
        &editor.redo_stack
    } else {
        &editor.undo_stack
    };
    let Some(record) = stack.last().and_then(marker).map(|m| m.record.clone()) else {
        return Ok(false);
    };
    if record.resources.is_some() { return resources::history(record, editor, other, redo).map(|_| true); }
    let participates = |pane: &Editor| {
        let stack = if redo {
            &pane.redo_stack
        } else {
            &pane.undo_stack
        };
        stack
            .last()
            .and_then(marker)
            .is_some_and(|m| Arc::ptr_eq(&m.record, &record))
    };
    let mut writes = Vec::new();
    for file in &record.files {
        let expected = read(if redo { &file.before } else { &file.after })?;
        let opened: Vec<_> = [&*editor, &*other]
            .into_iter()
            .filter(|p| matching(p, file))
            .collect();
        if file.open {
            if opened.is_empty() {
                return Err(format!(
                    "Cannot undo this rename after closing {}",
                    file.path.display()
                ));
            }
            for pane in opened {
                if pane.read_only || !participates(pane) || content(pane)? != expected {
                    return Err(
                        "Undo newer edits in the other pane before undoing this rename".into(),
                    );
                }
            }
        } else {
            if !opened.is_empty() {
                return Err(format!(
                    "Close {} before undoing this rename",
                    file.path.display()
                ));
            }
            writes.push(DiskWrite::stage(
                &file.path,
                expected,
                read(if redo { &file.after } else { &file.before })?,
            )?);
        }
    }
    if editor.read_only || (participates(other) && other.read_only) {
        return Err("Cannot undo a rename in a read-only pane".into());
    }
    commit_disk(&writes, &record)?;
    for pane in [&mut *editor, &mut *other] {
        if !participates(pane) {
            continue;
        }
        let mut entry = if redo {
            pane.redo_stack.pop().unwrap()
        } else {
            pane.undo_stack.pop().unwrap()
        };
        if let HistoryKind::Workspace(m) = &mut entry.kind {
            if let Some(snapshot) = &mut m.local {
                pane.document.swap_snapshot(snapshot);
                pane.dirty = true;
            }
        }
        pane.restore_cursor(if redo {
            entry.after.clone()
        } else {
            entry.before.clone()
        });
        pane.multi_edit_group = None;
        if redo {
            pane.undo_stack.push(entry);
        } else {
            pane.redo_stack.push(entry);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
