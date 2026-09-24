// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Crash recovery uses file-backed text, an immutable original snapshot, and an
//! append-only operation journal. No second in-memory document or piece list.
mod format;
mod snapshot;
#[cfg(test)]
mod tests;
use super::{Piece, PieceTable, read_at};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const COPY_BUFFER: usize = 64 * 1024;
const MAX_ENTRIES: usize = 100;

#[derive(Default)]
pub(super) struct State {
    root: Option<PathBuf>,
    source: Option<PathBuf>,
    journal: Option<Journal>,
    warning: Option<String>,
    failed: bool,
    recovered_from: Option<PathBuf>,
}

struct Journal {
    directory: PathBuf,
    log: Option<File>,
    lock: Option<File>,
    linked_add: bool,
    copied_add: u64,
    dirty: bool,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    version: u32,
    source: Option<PathBuf>,
    original_length: u64,
}

pub(super) enum Change<'a> {
    Insert(usize, &'a [Piece]),
    Delete(usize, usize),
    Snapshot(&'a [Piece]),
    Saved,
}

pub(crate) fn root() -> io::Result<PathBuf> {
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let root = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("Potyi/recovery"))
    } else if cfg!(target_os = "macos") {
        home.map(|p| p.join("Library/Application Support/Potyi/recovery"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home.map(|p| p.join(".local/state")))
            .map(|p| p.join("potyi/recovery"))
    };
    root.filter(|p| p.is_absolute())
        .ok_or_else(|| io::Error::other("Cannot locate the recovery directory"))
}

fn create_private(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn make_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn copy_range(input: &File, output: &mut File, start: u64, length: u64) -> io::Result<()> {
    let mut buffer = [0u8; COPY_BUFFER];
    let mut offset = start;
    let end = start
        .checked_add(length)
        .ok_or_else(|| io::Error::other("Recovery range overflow"))?;
    while offset < end {
        let size = (end - offset).min(buffer.len() as u64) as usize;
        let count = read_at(input, &mut buffer[..size], offset)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Recovery source was shortened",
            ));
        }
        output.write_all(&buffer[..count])?;
        offset += count as u64;
    }
    Ok(())
}

impl Journal {
    fn create(root: &Path, source: Option<&Path>, table: &PieceTable) -> io::Result<Self> {
        make_directory(root)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let directory = root.join(format!(
            "session-{}-{stamp}-{}",
            std::process::id(),
            table.revision()
        ));
        fs::create_dir(&directory)?;
        let result = (|| {
            let lock = create_private(&directory.join("owner.lock"))?;
            lock.try_lock().map_err(io::Error::other)?;
            let original = snapshot::create(
                &table.original,
                &directory.join("original"),
                table.original_length as u64,
            )?;
            original.sync_all()?;
            let linked_add = fs::hard_link(&table.add.path, directory.join("add")).is_ok();
            if !linked_add {
                create_private(&directory.join("add"))?;
            }
            let meta = Metadata {
                version: 1,
                source: source.map(Path::to_path_buf),
                original_length: table.original_length as u64,
            };
            let mut metadata = create_private(&directory.join("metadata.json"))?;
            serde_json::to_writer(&mut metadata, &meta).map_err(io::Error::other)?;
            metadata.sync_all()?;
            let log = create_private(&directory.join("journal"))?;
            let mut journal = Self {
                directory: directory.clone(),
                log: Some(log),
                lock: Some(lock),
                linked_add,
                copied_add: 0,
                dirty: true,
            };
            journal.record(table, Change::Snapshot(&table.pieces))?;
            let ready = create_private(&directory.join("ready"))?;
            ready.sync_all()?;
            sync_directory(&directory)?;
            sync_directory(root)?;
            Ok(journal)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&directory);
        }
        result
    }

    fn record(&mut self, table: &PieceTable, change: Change<'_>) -> io::Result<()> {
        // Data reaches disk before a frame can commit references to it.
        if !self.linked_add {
            let mut output = OpenOptions::new()
                .append(true)
                .open(self.directory.join("add"))?;
            let length = table.add.len() as u64;
            copy_range(
                table.add.file(),
                &mut output,
                self.copied_add,
                length
                    .checked_sub(self.copied_add)
                    .ok_or_else(|| io::Error::other("Recovery edit store shrank"))?,
            )?;
            output.sync_data()?;
            self.copied_add = length;
        } else {
            table.add.file().sync_data()?;
        }
        let saved = matches!(change, Change::Saved);
        let log = self.log.as_mut().unwrap();
        format::write_frame(log, change, table.add.len() as u64)?;
        log.sync_data()?;
        self.dirty = !saved;
        Ok(())
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        drop(self.log.take());
        drop(self.lock.take());
        if !self.dirty {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }
}

impl PieceTable {
    pub(crate) fn enable_recovery(&mut self, source: Option<&Path>) {
        match root() {
            Ok(root) => self.enable_recovery_at(root, source),
            Err(error) => {
                self.recovery.lock().unwrap().warning = Some(format!(
                    "Crash recovery unavailable: {error}. Save your work regularly."
                ))
            }
        }
    }

    pub(crate) fn enable_recovery_at(&mut self, root: PathBuf, source: Option<&Path>) {
        let mut state = self.recovery.lock().unwrap();
        state.root = Some(root);
        state.source = source.and_then(|p| std::path::absolute(p).ok());
    }

    pub(crate) fn take_recovery_warning(&mut self) -> Option<String> {
        self.recovery.lock().unwrap().warning.take()
    }

    pub(super) fn record_recovery(&mut self, change: Change<'_>) {
        let recovery = self.recovery.clone();
        let mut guard = recovery.lock().unwrap();
        let state = &mut *guard;
        if !state.failed {
            if let Some(root) = &state.root {
                let result = if let Some(journal) = &mut state.journal {
                    journal.record(self, change)
                } else {
                    Journal::create(root, state.source.as_deref(), self).and_then(|journal| {
                        // Pin the live original to the same immutable backing as
                        // recovery, including after saves or outside replacements.
                        self.original = std::sync::Arc::new(File::open(journal.directory.join("original"))?);
                        state.journal = Some(journal);
                        Ok(())
                    })
                };
                if let Err(error) = result {
                    state.failed = true;
                    // Preserve the last good frame, even if failure followed a save.
                    if let Some(journal) = &mut state.journal {
                        journal.dirty = true;
                    }
                    state.warning = Some(format!(
                        "Crash recovery stopped: {error}. Your current edits remain available; save them now."
                    ));
                }
            }
        }
    }

    pub(super) fn recovery_snapshot(&mut self) {
        // Borrow the existing pieces directly; do not clone them for the journal.
        let recovery = self.recovery.clone();
        let mut guard = recovery.lock().unwrap();
        let state = &mut *guard;
        if !state.failed {
            if let Some(root) = &state.root {
                let result = if let Some(journal) = &mut state.journal {
                    journal.record(self, Change::Snapshot(&self.pieces))
                } else {
                    Journal::create(root, state.source.as_deref(), self).and_then(|journal| {
                        // Pin the live original to the same immutable backing as
                        // recovery, including after saves or outside replacements.
                        self.original = std::sync::Arc::new(File::open(journal.directory.join("original"))?);
                        state.journal = Some(journal);
                        Ok(())
                    })
                };
                if let Err(error) = result {
                    state.failed = true;
                    if let Some(journal) = &mut state.journal {
                        journal.dirty = true;
                    }
                    state.warning = Some(format!(
                        "Crash recovery stopped: {error}. Save your current edits now."
                    ));
                }
            }
        }
    }

    pub(crate) fn recovered_from(&mut self, directory: PathBuf) {
        self.recovery.lock().unwrap().recovered_from = Some(directory);
    }

    pub(crate) fn recovery_saved(&mut self, path: &Path) {
        let recovery = self.recovery.clone();
        let mut guard = recovery.lock().unwrap();
        let state = &mut *guard;
        let restart_after_failure = state.failed;
        state.source = std::path::absolute(path).ok();
        if state.journal.is_some() {
            // Saving the complete document retires recovery even after an I/O
            // failure stopped journaling. A failed retirement keeps the entry.
            let journal = state.journal.as_mut().unwrap();
            let result = (|| -> io::Result<()> {
                let log = journal.log.as_mut().unwrap();
                format::write_frame(log, Change::Saved, self.add.len() as u64)?;
                log.sync_all()?;
                journal.dirty = false;
                Ok(())
            })();
            if let Err(error) = result {
                state.warning = Some(format!(
                    "File saved, but the previous recovery entry was kept: {error}"
                ));
            }
            if let Some(journal) = &state.journal {
                let update = (|| -> io::Result<()> {
                    let meta = Metadata {
                        version: 1,
                        source: state.source.clone(),
                        original_length: self.original_length as u64,
                    };
                    let temporary = journal.directory.join("metadata.new");
                    let mut output = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&temporary)?;
                    serde_json::to_writer(&mut output, &meta).map_err(io::Error::other)?;
                    output.sync_all()?;
                    drop(output);
                    fs::rename(temporary, journal.directory.join("metadata.json"))
                })();
                if let Err(error) = update {
                    state.warning = Some(format!(
                        "File saved, but its recovery label could not be updated: {error}"
                    ));
                }
            }
        }
        if restart_after_failure {
            // Edits made while journaling was unavailable need a fresh full
            // checkpoint on the next edit, not a delta against the old state.
            state.journal = None;
            state.failed = false;
        }
        if let Some(directory) = &state.recovered_from {
            let result = (|| -> io::Result<()> {
                let _lock = match claim(directory) {
                    Ok(Some(lock)) => lock,
                    Ok(None) => return Err(io::Error::other("Recovery session is in use")),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                    Err(error) => return Err(error),
                };
                let mut journal = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(directory.join("journal"))?;
                let add_length = fs::metadata(directory.join("add"))?.len();
                format::write_frame(&mut journal, Change::Saved, add_length)?;
                journal.sync_all()
            })();
            match result {
                Ok(()) => {
                    let _ = fs::remove_dir_all(directory);
                    state.recovered_from = None;
                }
                Err(error) => {
                    state.warning = Some(format!(
                        "File saved. The older recovery entry was kept: {error}"
                    ))
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Entry {
    pub id: String,
    pub source: Option<PathBuf>,
}

fn claim(directory: &Path) -> io::Result<Option<File>> {
    if fs::symlink_metadata(directory)?.file_type().is_symlink() {
        return Ok(None);
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.join("owner.lock"))?;
    match lock.try_lock() {
        Ok(()) => Ok(Some(lock)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error),
    }
}

fn metadata(directory: &Path) -> io::Result<Metadata> {
    let mut bytes = Vec::new();
    File::open(directory.join("metadata.json"))?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(io::Error::other("Recovery metadata exceeds its limit"));
    }
    let meta: Metadata = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if meta.version != 1 {
        return Err(io::Error::other("Unsupported recovery version"));
    }
    Ok(meta)
}

pub(crate) fn list(root: &Path) -> io::Result<Vec<Entry>> {
    let directories = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e),
    };
    let mut result = Vec::new();
    for entry in directories {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) || !entry.path().join("ready").is_file() {
            continue;
        }
        let Ok(Some(_lock)) = claim(&entry.path()) else {
            continue;
        };
        if format::last_is_saved(&entry.path().join("journal")).unwrap_or(false) {
            drop(_lock);
            let _ = fs::remove_dir_all(entry.path());
            continue;
        }
        // One damaged session must not prevent recovering the other documents.
        let source = metadata(&entry.path()).ok().and_then(|meta| meta.source);
        result.push(Entry { id, source });
        if result.len() >= MAX_ENTRIES {
            break;
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

fn valid_id(id: &str) -> bool {
    id.starts_with("session-")
        && id.len() < 120
        && id[8..].bytes().all(|b| b.is_ascii_digit() || b == b'-')
}

pub(crate) fn describe() -> io::Result<String> {
    let entries = list(&root()?)?;
    Ok(describe_entries(&entries))
}

fn describe_entries(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return "No unsaved recovery sessions. Sessions open in another Potyi window are left alone."
            .into();
    }
    let mut lines =
        vec!["Unsaved work is available. Recover a separate copy with :recover <number>.".into()];
    for (index, entry) in entries.iter().enumerate() {
        lines.push(format!(
            "{}: {}",
            index + 1,
            entry
                .source
                .as_deref()
                .map(display_path)
                .unwrap_or_else(|| "Untitled document".into())
        ));
    }
    lines.push("Recovery keeps the originals unchanged. Save As chooses where the recovered work belongs. Up to 100 sessions are listed.".into());
    lines.join("\n")
}

// Only simplify the displayed spelling. File operations keep the original
// extended-length Windows path, including paths longer than MAX_PATH.
fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{unc}");
    }
    if let Some(drive) = text.strip_prefix(r"\\?\") {
        let bytes = drive.as_bytes();
        if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':' && bytes[2] == b'\\'
        {
            return drive.to_owned();
        }
    }
    text.into_owned()
}

pub(crate) struct StartupNotice {
    pub text: String,
    directories: Vec<PathBuf>,
}

impl StartupNotice {
    // A dismissed offer does not discard the recovery data. Explicit :recover
    // continues to list all unsaved sessions, including acknowledged ones.
    pub fn acknowledge(self) {
        for directory in self.directories {
            if let Ok(Some(_lock)) = claim(&directory) {
                let _ = OpenOptions::new().write(true).create_new(true)
                    .open(directory.join("notice-dismissed"));
            }
        }
    }
}

pub(crate) fn startup_notice(root: &Path) -> io::Result<Option<StartupNotice>> {
    let entries = list(root)?;
    if !entries.iter().any(|entry| !root.join(&entry.id).join("notice-dismissed").is_file()) {
        return Ok(None);
    }
    let text = format!("{}\nDismiss this panel to stop reminders for these sessions. They remain available through :recover.",
        describe_entries(&entries));
    Ok(Some(StartupNotice {
        text,
        directories: entries.iter().map(|entry| root.join(&entry.id)).collect(),
    }))
}

pub(crate) fn restore_number_at(
    root: &Path,
    number: usize,
) -> io::Result<(PathBuf, bool, PathBuf)> {
    let entries = list(&root)?;
    let entry = number
        .checked_sub(1)
        .and_then(|i| entries.get(i))
        .ok_or_else(|| io::Error::other("Choose a recovery number from :recover"))?;
    let (path, incomplete) = restore(&root, &entry.id)?;
    Ok((path, incomplete, root.join(&entry.id)))
}

fn restore(root: &Path, id: &str) -> io::Result<(PathBuf, bool)> {
    if !valid_id(id) {
        return Err(io::Error::other("Invalid recovery session"));
    }
    let directory = root.join(id);
    let _lock = claim(&directory)?
        .ok_or_else(|| io::Error::other("This document is still open in another Potyi window"))?;
    let meta = metadata(&directory)?;
    let (table, incomplete) = format::replay(&directory, &meta)?;
    let extension = meta
        .source
        .as_deref()
        .and_then(Path::extension)
        .and_then(|s| s.to_str())
        .filter(|s| s.len() < 20 && s.bytes().all(|b| b.is_ascii_alphanumeric()))
        .unwrap_or("txt");
    let output = root.join(format!(
        "recovered-{}-{}.{}",
        &id[8..],
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos(),
        extension
    ));
    table.write_to_new(&output)?;
    // Keep the journal until the recovered document has actually been saved.
    Ok((output, incomplete))
}

static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(crate) fn enable_for_application() {
    ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
}
pub(crate) fn arm(table: &mut PieceTable, source: Option<&Path>) {
    if ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        table.enable_recovery(source);
    }
}

// Directory entries must be committed before a saved frame can retire recovery.
pub(super) fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
