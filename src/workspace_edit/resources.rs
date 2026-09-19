// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later
//! Ordered resource edits simulated before any filesystem mutation.
use super::*;
use std::collections::BTreeMap;
use std::io::Write as _;

#[derive(Debug)]
struct DiskImage {
    path: PathBuf,
    before: Option<PathBuf>,
    after: Option<PathBuf>,
    permissions: Option<fs::Permissions>,
}
#[derive(Debug)]
struct BufferImage {
    before_path: PathBuf,
    after_path: Option<PathBuf>,
    before: PathBuf,
    after: PathBuf,
}
#[derive(Debug)]
pub(super) struct Plan {
    disks: Vec<DiskImage>,
    buffers: Vec<BufferImage>,
}
impl Plan {
    pub fn paths(&self, undone: bool) -> Vec<(PathBuf, u8)> {
        self.disks
            .iter()
            .map(|d| {
                let (before, after) = if undone {
                    (&d.after, &d.before)
                } else {
                    (&d.before, &d.after)
                };
                (
                    d.path.clone(),
                    if after.is_none() {
                        3
                    } else if before.is_none() {
                        1
                    } else {
                        2
                    },
                )
            })
            .collect()
    }
}

#[derive(Clone)]
pub(super) struct BufferHistory {
    index: usize,
    before_dirty: bool,
    after_dirty: bool,
}

struct Entity {
    original: Option<PathBuf>,
    path: Option<PathBuf>,
    disk: Option<Vec<u8>>,
    before: String,
    text: String,
    open: bool,
    permissions: Option<fs::Permissions>,
    edits: usize,
}

fn path(uri: &str, root: &Path) -> Result<PathBuf, String> {
    let raw = lsp::uri_path(uri)?;
    let parent = raw
        .parent()
        .ok_or("File operation has no parent directory")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let name = raw.file_name().ok_or("File operation has no file name")?;
    let path = parent.join(name);
    if !path.starts_with(root)
        || path == root
        || path
            .strip_prefix(root)
            .unwrap()
            .components()
            .any(|c| c.as_os_str() == ".git")
    {
        return Err("File operation targets a protected path or a path outside the project".into());
    }
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if !meta.is_file() || meta.file_type().is_symlink() || meta.permissions().readonly() {
            return Err(format!(
                "File operation requires a writable regular file: {}",
                path.display()
            ));
        }
    }
    Ok(path)
}

fn load(
    path: &Path,
    entities: &mut Vec<Entity>,
    live: &mut BTreeMap<PathBuf, usize>,
    docs: &[lsp::Document],
    started: SystemTime,
) -> Result<usize, String> {
    if let Some(index) = live.get(path) {
        return Ok(*index);
    }
    // A path removed earlier in this edit must not be resurrected from disk.
    if entities.iter().any(|e| e.original.as_deref() == Some(path)) {
        return Err("Edit refers to a moved or deleted file".into());
    }
    let disk = read(path)?;
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let doc = docs
        .iter()
        .find(|d| d.path.canonicalize().ok().as_deref() == Some(path));
    if meta.modified().map_err(|e| e.to_string())? > started {
        return Err("A file changed while actions were being prepared; request them again".into());
    }
    let text = match doc {
        Some(d) => d.text.clone(),
        None => {
            String::from_utf8(disk.clone()).map_err(|_| "File operations require UTF-8 text")?
        }
    };
    let index = entities.len();
    entities.push(Entity {
        original: Some(path.into()),
        path: Some(path.into()),
        disk: Some(disk),
        before: text.clone(),
        text,
        open: doc.is_some(),
        permissions: Some(meta.permissions()),
        edits: 0,
    });
    live.insert(path.into(), index);
    Ok(index)
}

fn image(scratch: &Scratch, index: &mut usize, bytes: &[u8]) -> Result<PathBuf, String> {
    let path = scratch.0.join(format!("image-{}", *index));
    *index += 1;
    fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

pub(super) fn prepare(
    edit: &Value,
    root: &Path,
    docs: &[lsp::Document],
    versions: &HashMap<PathBuf, (String, i64)>,
    started: SystemTime,
) -> Result<PreparedRename, String> {
    if edit.get("changes").is_some() {
        return Err("Ambiguous workspace edit".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let changes = edit["documentChanges"]
        .as_array()
        .ok_or("Invalid documentChanges")?;
    if changes.len() > MAX_EDITS {
        return Err("Too many workspace operations".into());
    }
    let notes = annotation_notes(edit)?;
    let mut entities = Vec::new();
    let mut live = BTreeMap::new();
    let mut count = 0;
    for change in changes {
        validate_edit_metadata(change, edit)?;
        match change["kind"].as_str() {
            Some("create") => {
                let p = path(change["uri"].as_str().ok_or("Missing create URI")?, &root)?;
                if live.contains_key(&p) || p.exists() {
                    return Err(format!("Destination already exists: {}", p.display()));
                }
                let index = entities.len();
                entities.push(Entity {
                    original: None,
                    path: Some(p.clone()),
                    disk: None,
                    before: String::new(),
                    text: String::new(),
                    open: false,
                    permissions: None,
                    edits: 0,
                });
                live.insert(p, index);
            }
            Some("rename") => {
                let old = path(
                    change["oldUri"].as_str().ok_or("Missing source URI")?,
                    &root,
                )?;
                let new = path(
                    change["newUri"].as_str().ok_or("Missing destination URI")?,
                    &root,
                )?;
                if old == new {
                    continue;
                }
                if live.contains_key(&new) || new.exists() {
                    return Err(format!("Destination already exists: {}", new.display()));
                }
                let i = load(&old, &mut entities, &mut live, docs, started)?;
                live.remove(&old);
                live.insert(new.clone(), i);
                entities[i].path = Some(new);
            }
            Some("delete") => {
                let p = path(change["uri"].as_str().ok_or("Missing delete URI")?, &root)?;
                let i = load(&p, &mut entities, &mut live, docs, started)?;
                live.remove(&p);
                entities[i].path = None;
            }
            Some(_) => return Err("Unknown workspace file operation".into()),
            None => {
                let p = path(
                    change["textDocument"]["uri"]
                        .as_str()
                        .ok_or("Missing edit URI")?,
                    &root,
                )?;
                let i = load(&p, &mut entities, &mut live, docs, started)?;
                let entity = &mut entities[i];
                if let Some(v) = change["textDocument"]
                    .get("version")
                    .filter(|v| !v.is_null())
                {
                    let v = v.as_i64().ok_or("Invalid document version")?;
                    if entity
                        .original
                        .as_ref()
                        .and_then(|original| {
                            versions
                                .iter()
                                .find(|(p, _)| {
                                    *p == original
                                        || p.canonicalize().ok().as_ref() == Some(original)
                                })
                                .map(|(_, version)| version)
                        })
                        .is_none_or(|(_, current)| *current != v)
                    {
                        return Err("Workspace edit has a stale document version".into());
                    }
                }
                let edits = change["edits"].as_array().ok_or("Missing text edits")?;
                count += edits.len();
                if count > MAX_EDITS {
                    return Err("Too many text edits".into());
                }
                let mut replacements = Vec::new();
                for e in edits {
                    validate_edit_metadata(e, edit)?;
                    let start = byte_position(&entity.text, &e["range"]["start"])?;
                    let end = byte_position(&entity.text, &e["range"]["end"])?;
                    if start > end {
                        return Err("Reversed edit range".into());
                    }
                    replacements.push((
                        start..end,
                        e["newText"].as_str().ok_or("Missing replacement text")?,
                    ));
                }
                replacements.sort_by_key(|(r, _)| (r.start, r.end));
                if replacements
                    .windows(2)
                    .any(|w| w[0].0.end > w[1].0.start || w[0].0.start == w[1].0.start)
                {
                    return Err("Overlapping edits".into());
                }
                let mut size = entity.text.len();
                for (r, t) in &replacements {
                    size = size
                        .checked_sub(r.len())
                        .and_then(|s| s.checked_add(t.len()))
                        .ok_or("Edit size overflow")?;
                }
                if size > lsp::MAX_DOCUMENT_BYTES {
                    return Err("Edited file exceeds 2 MiB".into());
                }
                entity.edits += edits.len();
                for (r, t) in replacements.into_iter().rev() {
                    entity.text.replace_range(r, t);
                }
            }
        }
        if entities.len() > MAX_FILES
            || entities
                .iter()
                .map(|e| e.before.len() + e.text.len() + e.disk.as_ref().map_or(0, Vec::len))
                .sum::<usize>()
                > MAX_TOTAL
        {
            return Err("Workspace operation exceeds the 64-file or 16 MiB limit".into());
        }
    }
    let scratch = Scratch::new()?;
    let mut index = 0;
    let mut files = Vec::new();
    let mut disks =
        BTreeMap::<PathBuf, (Option<Vec<u8>>, Option<Vec<u8>>, Option<fs::Permissions>)>::new();
    let mut buffers = Vec::new();
    for e in entities {
        if e.original == e.path && e.before == e.text {
            continue;
        }
        let display = e
            .path
            .as_ref()
            .or(e.original.as_ref())
            .ok_or("Empty file operation")?
            .clone();
        let before = image(&scratch, &mut index, e.before.as_bytes())?;
        let after = image(&scratch, &mut index, e.text.as_bytes())?;
        let operation = match (&e.original, &e.path) {
            (None, Some(p)) => format!("Create {}", p.display()),
            (Some(p), None) => format!("Delete {}", p.display()),
            (Some(a), Some(b)) if a != b => format!("Move {} → {}", a.display(), b.display()),
            _ => format!("Edit {}", display.display()),
        };
        let preview = format!(
            "{operation}\n{notes}\n{}\n\nBefore:\n{}\n\nAfter:\n{}\n\nText preview is limited to 4,000 characters per side.",
            if e.open {
                "Open text remains in its buffer; moved buffers keep unsaved changes. Deleted open files become unsaved, untitled buffers."
            } else {
                "Apply writes these filesystem changes."
            },
            e.before.chars().take(4000).collect::<String>(),
            e.text.chars().take(4000).collect::<String>()
        );
        let short = |p: &Path| p.strip_prefix(&root).unwrap_or(p).display().to_string();
        let label = match (&e.original, &e.path) {
            (None, Some(p)) => format!("Create {}", short(p)),
            (Some(p), None) => format!("Delete {}", short(p)),
            (Some(a), Some(b)) if a != b => format!("Move {} → {}", short(a), short(b)),
            _ => format!(
                "Edit {} · {}",
                short(&display),
                if e.open {
                    "unsaved buffer"
                } else {
                    "writes file"
                }
            ),
        };
        files.push(FileChange {
            operation: Some(label),
            path: display,
            before: before.clone(),
            after: after.clone(),
            open: e.open,
            edits: vec![(0..e.before.len(), e.text.len())],
            preview,
        });
        if e.open {
            buffers.push(BufferImage {
                before_path: e.original.clone().unwrap(),
                after_path: e.path.clone(),
                before,
                after,
            });
        }
        if e.original != e.path || !e.open {
            if let Some(old) = &e.original {
                disks.insert(old.clone(), (e.disk.clone(), None, e.permissions.clone()));
            }
            if let Some(new) = &e.path {
                let desired = if e.open {
                    e.disk.clone().unwrap()
                } else {
                    e.text.into_bytes()
                };
                let initial = if e.original.as_ref() == Some(new) {
                    e.disk
                } else {
                    None
                };
                disks.insert(new.clone(), (initial, Some(desired), e.permissions));
            }
        }
    }
    if files.is_empty() {
        return Err("No workspace changes returned".into());
    }
    let mut disk_images = Vec::new();
    for (path, (before, after, permissions)) in disks {
        if before == after {
            continue;
        }
        disk_images.push(DiskImage {
            path,
            before: before
                .as_ref()
                .map(|b| image(&scratch, &mut index, b))
                .transpose()?,
            after: after
                .as_ref()
                .map(|b| image(&scratch, &mut index, b))
                .transpose()?,
            permissions,
        });
    }
    Ok(PreparedRename {
        files,
        _scratch: scratch,
        root,
        resources: Some(Plan {
            disks: disk_images,
            buffers,
        }),
    })
}

fn optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && !m.permissions().readonly() => {
            read(path).map(Some)
        }
        Ok(_) => Err(format!("Not a writable regular file: {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
struct Write {
    path: PathBuf,
    temp: PathBuf,
    old: Option<Vec<u8>>,
    new: Option<Vec<u8>>,
    permissions: Option<fs::Permissions>,
}
impl Drop for Write {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.temp);
    }
}
impl Write {
    fn stage(
        path: &Path,
        old: Option<Vec<u8>>,
        new: Option<Vec<u8>>,
        permissions: Option<fs::Permissions>,
    ) -> Result<Self, String> {
        if optional(path)? != old {
            return Err(format!("File changed since preview: {}", path.display()));
        }
        let parent = path
            .parent()
            .unwrap()
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if parent.join(path.file_name().unwrap()) != path {
            return Err("Destination directory changed".into());
        }
        let temp = parent.join(format!(
            ".potyi-action-{}-{}",
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
        let write = Self {
            path: path.into(),
            temp,
            old,
            new,
            permissions,
        };
        if let Some(bytes) = &write.new {
            output.write_all(bytes).map_err(|e| e.to_string())?;
        }
        if let Some(p) = &write.permissions {
            output
                .set_permissions(p.clone())
                .map_err(|e| e.to_string())?;
        }
        output.sync_all().map_err(|e| e.to_string())?;
        Ok(write)
    }
    fn commit(&self) -> Result<(), String> {
        if self
            .path
            .parent()
            .unwrap()
            .canonicalize()
            .map_err(|e| e.to_string())?
            .join(self.path.file_name().unwrap())
            != self.path
            || optional(&self.path)? != self.old
        {
            return Err(format!(
                "File or directory changed: {}",
                self.path.display()
            ));
        }
        match (&self.old, &self.new) {
            (None, Some(_)) => fs::hard_link(&self.temp, &self.path), // atomic, never overwrite a new destination
            (Some(_), Some(_)) => fs::rename(&self.temp, &self.path),
            (Some(_), None) => fs::remove_file(&self.path),
            (None, None) => Ok(()),
        }
        .map_err(|e| e.to_string())
    }
}
fn commit(writes: &[Write], record: &Arc<PreparedRename>) -> Result<(), String> {
    for (i, w) in writes.iter().enumerate() {
        if let Err(mut error) = w.commit() {
            for previous in writes[..i].iter().rev() {
                if let Err(e) = Write::stage(
                    &previous.path,
                    previous.new.clone(),
                    previous.old.clone(),
                    previous.permissions.clone(),
                )
                .and_then(|w| w.commit())
                {
                    record._scratch.1.store(true, Ordering::Relaxed);
                    error.push_str(&format!(
                        "; rollback failed: {e}. Recovery files: {}",
                        record._scratch.0.display()
                    ));
                }
            }
            return Err(error);
        }
    }
    Ok(())
}
fn stage_disks(plan: &Plan, redo: bool) -> Result<Vec<Write>, String> {
    plan.disks
        .iter()
        .map(|d| {
            let (old, new) = if redo {
                (&d.before, &d.after)
            } else {
                (&d.after, &d.before)
            };
            Write::stage(
                &d.path,
                old.as_ref().map(|p| read(p)).transpose()?,
                new.as_ref().map(|p| read(p)).transpose()?,
                d.permissions.clone(),
            )
        })
        .collect()
}
fn same(pane: &Editor, path: &Path) -> bool {
    pane.path
        .as_ref()
        .and_then(|p| p.canonicalize().ok())
        .as_deref()
        == Some(path)
        || pane.path.as_deref() == Some(path)
}

pub(super) fn apply(
    record: Arc<PreparedRename>,
    editor: &mut Editor,
    other: &mut Editor,
) -> Result<(), String> {
    let plan = record.resources.as_ref().unwrap();
    let mut staged = Vec::new();
    for (i, b) in plan.buffers.iter().enumerate() {
        let panes: Vec<_> = [&*editor, &*other]
            .into_iter()
            .enumerate()
            .filter(|(_, p)| same(p, &b.before_path))
            .map(|(i, _)| i)
            .collect();
        if panes.is_empty() {
            return Err("An open buffer was closed during the preview".into());
        }
        for pane_index in panes {
            let pane = if pane_index == 0 {
                &mut *editor
            } else {
                &mut *other
            };
            if pane.read_only || content(pane)? != read(&b.before)? {
                return Err("A buffer changed or is read-only; request the action again".into());
            }
            let bytes = read(&b.after)?;
            let before = pane.cursor_state();
            let after = cursor_at(&bytes, before.position);
            let mut snapshot = pane
                .document
                .replace_from_reader(&mut std::io::Cursor::new(bytes), lsp::MAX_DOCUMENT_BYTES)
                .map_err(|e| e.to_string())?;
            if let Some(s) = &mut snapshot {
                pane.document.swap_snapshot(s);
            }
            let state = BufferHistory {
                index: i,
                before_dirty: pane.dirty,
                after_dirty: pane.dirty || snapshot.is_some() || b.after_path.is_none(),
            };
            staged.push((pane_index, snapshot, before, after, state));
        }
    }
    for pane in [&*editor, &*other] {
        if plan.disks.iter().any(|d| same(pane, &d.path))
            && !plan.buffers.iter().any(|b| same(pane, &b.before_path))
        {
            return Err("Open files changed during the preview; request the action again".into());
        }
    }
    let writes = stage_disks(plan, true)?;
    commit(&writes, &record)?;
    let anchor = !staged.iter().any(|(i, ..)| *i == 0);
    for (i, mut snapshot, before, after, state) in staged {
        let pane = if i == 0 { &mut *editor } else { &mut *other };
        if let Some(s) = &mut snapshot {
            pane.document.swap_snapshot(s);
        }
        pane.path = plan.buffers[state.index].after_path.clone();
        pane.dirty = state.after_dirty;
        pane.restore_cursor(after.clone());
        pane.multi_edit_group = None;
        pane.undo_stack.push(HistoryEntry {
            kind: HistoryKind::Workspace(Marker {
                record: record.clone(),
                local: snapshot,
                resource: Some(state),
            }),
            before,
            after,
        });
        pane.redo_stack.clear();
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

pub(super) fn history(
    record: Arc<PreparedRename>,
    editor: &mut Editor,
    other: &mut Editor,
    redo: bool,
) -> Result<(), String> {
    let plan = record.resources.as_ref().unwrap();
    let mut found = std::collections::HashSet::new();
    for pane in [&*editor, &*other] {
        let stack = if redo {
            &pane.redo_stack
        } else {
            &pane.undo_stack
        };
        let state = stack
            .last()
            .and_then(marker)
            .filter(|m| Arc::ptr_eq(&m.record, &record));
        if let Some(m) = state {
            if pane.read_only {
                return Err("Cannot undo in a read-only pane".into());
            }
            if let Some(h) = &m.resource {
                let b = &plan.buffers[h.index];
                let expected_path = if redo {
                    Some(b.before_path.clone())
                } else {
                    b.after_path.clone()
                };
                if pane.path != expected_path
                    || content(pane)? != read(if redo { &b.before } else { &b.after })?
                {
                    return Err("Buffer path or text changed since this action".into());
                }
                found.insert(h.index);
            }
        } else if stack
            .iter()
            .filter_map(marker)
            .any(|m| Arc::ptr_eq(&m.record, &record))
            || plan.disks.iter().any(|d| same(pane, &d.path))
            || plan.buffers.iter().any(|b| {
                let expected = if redo {
                    Some(&b.before_path)
                } else {
                    b.after_path.as_ref()
                };
                expected.is_some_and(|p| same(pane, p))
            })
        {
            return Err("Close files opened after this action, or undo newer edits first".into());
        }
    }
    if found.len() != plan.buffers.len() {
        return Err(
            "Undo newer edits in the other pane or reopen the participating buffer first".into(),
        );
    }
    let writes = stage_disks(plan, redo)?;
    commit(&writes, &record)?;
    for pane in [&mut *editor, &mut *other] {
        let stack = if redo {
            &mut pane.redo_stack
        } else {
            &mut pane.undo_stack
        };
        if stack
            .last()
            .and_then(marker)
            .is_none_or(|m| !Arc::ptr_eq(&m.record, &record))
        {
            continue;
        }
        let mut entry = stack.pop().unwrap();
        if let HistoryKind::Workspace(m) = &mut entry.kind {
            if let Some(snapshot) = &mut m.local {
                pane.document.swap_snapshot(snapshot);
            }
            if let Some(h) = &m.resource {
                let b = &plan.buffers[h.index];
                pane.path = if redo {
                    b.after_path.clone()
                } else {
                    Some(b.before_path.clone())
                };
                pane.dirty = if redo { h.after_dirty } else { h.before_dirty };
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_late_collision_rolls_back_earlier_committed_files() {
        let root = Scratch::new().unwrap();
        let canonical_root = root.0.canonicalize().unwrap();
        let a = canonical_root.join("a");
        let b = canonical_root.join("b");
        fs::write(&a, "before").unwrap();
        let preview = prepare(
            &serde_json::json!({"documentChanges":[
                {"kind":"create","uri":lsp::file_uri(&b).unwrap()}
            ]}),
            &root.0,
            &[],
            &HashMap::new(),
            SystemTime::now(),
        )
        .unwrap();
        let record = Arc::new(preview);
        let writes = vec![
            Write::stage(&a, Some(b"before".to_vec()), Some(b"after".to_vec()), None).unwrap(),
            Write::stage(&b, None, Some(b"new".to_vec()), None).unwrap(),
        ];
        fs::write(&b, "external").unwrap();
        assert!(commit(&writes, &record).is_err());
        assert_eq!(fs::read_to_string(&a).unwrap(), "before");
        assert_eq!(fs::read_to_string(&b).unwrap(), "external");
    }
}
