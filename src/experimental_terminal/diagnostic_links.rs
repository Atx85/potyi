// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Click-scoped diagnostics. Retained text stays on disk; file probes and
//! command provenance checks run off the UI thread and are cancellable.
use super::{
    links::FileLink,
    output_selection::{self, Bounds, RecordText, TextSource},
    output_source::FrozenSource,
    session::Wake,
    transcript::{Anchor, Record, RecordData, SharedTranscript},
};
use std::{
    io,
    ops::Range,
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
};

const MAX_LINE_BYTES: usize = 64 * 1024;
const MAX_LINE_ROWS: usize = 256;
const MAX_CANDIDATES: usize = 64;
const MAX_SOURCE_ROWS: usize = 65_536;
#[path = "diagnostic_context.rs"]
mod context;

/// Pure, bounded classification shared by click activation and persisted
/// underlining. Cache one context per source; this never probes the filesystem.
#[derive(Clone, Default)]
pub(super) struct CommandContext(context::Context);
impl CommandContext {
    pub(super) fn new(command: &str) -> Self {
        Self(context::Context::new(command))
    }
    pub(super) fn underline(&self, line: &str) -> Option<Range<usize>> {
        self.0.underline(line)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Provenance {
    pub(super) epoch: u64,
    pub(super) source: u64,
    pub(super) cwd: PathBuf,
    /// False if an authenticated helper revealed another cwd. Final cwd
    /// equality cannot undo an observed intermediate directory change.
    pub(super) relative_safe: bool,
}

struct Resolved {
    link: FileLink,
    relative_guard: Option<Anchor>,
}

pub(super) struct Job {
    receiver: Receiver<io::Result<Option<Resolved>>>,
    store: SharedTranscript,
    click: Anchor,
    provenance: Provenance,
    cancelled: Arc<AtomicBool>,
    pending: bool,
}
impl Job {
    pub(super) fn start(
        click: Anchor,
        store: SharedTranscript,
        frozen: Option<FrozenSource>,
        provenance: Provenance,
        wake: Wake,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = cancelled.clone();
        let publish_store = store.clone();
        let publish_provenance = provenance.clone();
        std::thread::spawn(move || {
            let result = if let Some(mut source) = frozen {
                resolve_guarded(click, &store, &mut source, &provenance, &cancellation)
            } else {
                let mut source = DiskSource::new(store.clone());
                resolve_guarded(click, &store, &mut source, &provenance, &cancellation)
            };
            if !cancellation.load(Ordering::Acquire) && sender.send(result).is_ok() {
                wake();
            }
        });
        Self {
            receiver,
            store: publish_store,
            click,
            provenance: publish_provenance,
            cancelled,
            pending: true,
        }
    }
    // Called only after helper requests have been drained. This fixed-size
    // check also closes the worker-finished-to-UI-publication eviction race.
    fn validate_publication(&self, resolved: &Resolved) -> io::Result<bool> {
        let mut store = self.store.lock().unwrap();
        if store.epoch() != self.click.epoch || self.click.record_id < store.base_id() {
            return Ok(false);
        }
        if self.click.record_id < store.base_id() + store.len() as u64 {
            let record = store.read_anchor(self.click)?;
            if !terminal_origin(&record, &self.provenance) || record.cwd != self.provenance.cwd {
                return Ok(false);
            }
        }
        if let Some(anchor) = resolved.relative_guard {
            if anchor.epoch != store.epoch()
                || anchor.record_id < store.base_id()
                || anchor.record_id >= store.base_id() + store.len() as u64
            {
                return Ok(false);
            }
            let record = store.read_anchor(anchor)?;
            if record.source != self.provenance.source
                || record.cwd != self.provenance.cwd
                || !matches!(
                    record.data,
                    RecordData::Result {
                        relative_safe: true,
                        ..
                    }
                )
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn pending(&self) -> bool {
        self.pending
    }
    pub(super) fn poll(&mut self) -> Option<io::Result<Option<FileLink>>> {
        match self.receiver.try_recv() {
            Ok(result) => {
                self.pending = false;
                Some(result.and_then(|resolved| {
                    match resolved {
                        Some(resolved) => self
                            .validate_publication(&resolved)
                            .map(|valid| valid.then_some(resolved.link)),
                        None => Ok(None),
                    }
                }))
            }
            Err(mpsc::TryRecvError::Disconnected) if self.pending => {
                self.pending = false;
                Some(Err(io::Error::other("Diagnostic lookup cancelled")))
            }
            _ => None,
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

struct DiskSource {
    store: SharedTranscript,
    bounds: Bounds,
}
impl DiskSource {
    fn new(store: SharedTranscript) -> Self {
        let bounds = {
            let transcript = store.lock().unwrap();
            Bounds {
                epoch: transcript.epoch(),
                first: transcript.base_id(),
                end: transcript.base_id() + transcript.len() as u64,
            }
        };
        Self { store, bounds }
    }
}
impl TextSource for DiskSource {
    fn bounds(&self) -> Bounds {
        self.bounds
    }
    fn text(&mut self, id: u64) -> io::Result<RecordText> {
        let mut store = self.store.lock().unwrap();
        validate_store(&store, self.bounds)?;
        let record = store.read_id(id)?;
        let join = if id + 1 < self.bounds.end {
            output_selection::joins_next(&record, &store.read_id(id + 1)?)
        } else {
            false
        };
        drop(store);
        let mut text = output_selection::record_text(&record)?;
        text.join_next &= join;
        Ok(text)
    }
}

fn check_cancel(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Acquire) {
        Err(io::Error::other("Diagnostic lookup cancelled"))
    } else {
        Ok(())
    }
}
fn validate_store(store: &super::transcript::Transcript, bounds: Bounds) -> io::Result<()> {
    if store.epoch() != bounds.epoch || store.base_id() > bounds.first {
        Err(io::Error::other("History changed during diagnostic lookup"))
    } else {
        Ok(())
    }
}
fn metadata(
    store: &SharedTranscript,
    bounds: Bounds,
    id: u64,
    cancelled: &AtomicBool,
) -> io::Result<Option<Record>> {
    check_cancel(cancelled)?;
    let mut store = store.lock().unwrap();
    validate_store(&store, bounds)?;
    if id >= store.base_id() + store.len() as u64 {
        return Ok(None);
    }
    store.read_id(id).map(Some)
}
fn terminal_origin(record: &Record, provenance: &Provenance) -> bool {
    record.epoch == provenance.epoch
        && record.source == provenance.source
        && matches!(record.data, RecordData::Terminal { .. })
}

#[cfg(test)]
fn resolve(
    click: Anchor,
    store: &SharedTranscript,
    source: &mut impl TextSource,
    provenance: &Provenance,
    cancelled: &AtomicBool,
) -> io::Result<Option<FileLink>> {
    resolve_guarded(click, store, source, provenance, cancelled)
        .map(|result| result.map(|resolved| resolved.link))
}

fn resolve_guarded(
    click: Anchor,
    store: &SharedTranscript,
    source: &mut impl TextSource,
    provenance: &Provenance,
    cancelled: &AtomicBool,
) -> io::Result<Option<Resolved>> {
    check_cancel(cancelled)?;
    let bounds = source.bounds();
    if click.epoch != bounds.epoch
        || click.epoch != provenance.epoch
        || click.record_id < bounds.first
        || click.record_id >= bounds.end
        || provenance.source == 0
    {
        return Ok(None);
    }
    let saved_click = metadata(store, bounds, click.record_id, cancelled)?;
    if saved_click
        .as_ref()
        .is_some_and(|record| !terminal_origin(record, provenance) || record.cwd != provenance.cwd)
    {
        return Ok(None);
    }
    let Some((line, byte)) = logical_line(click, store, source, provenance, cancelled)? else {
        return Ok(None);
    };
    let (command, completion) = source_info(store, bounds, click.record_id, provenance, cancelled)?;
    if let Some(location) = command.parse(&line) {
        if !location.range.contains(&byte) {
            return Ok(None);
        }
        let original = Path::new(&location.path);
        let relative_guard = if original.is_absolute() {
            None
        } else {
            let Some(guard) = (if saved_click.is_some() {
                completion
            } else {
                None
            }) else {
                return Ok(None);
            };
            Some(guard)
        };
        let exact = if original.is_absolute() {
            original.to_path_buf()
        } else {
            provenance.cwd.join(original)
        };
        let mut path = None;
        check_cancel(cancelled)?;
        match std::fs::metadata(&exact) {
            Ok(metadata) if metadata.is_file() => path = Some(exact),
            Err(error)
                if location.compiler
                    && error.kind() == io::ErrorKind::NotFound
                    && original.is_relative()
                    && original.components().all(|part| {
                        matches!(
                            part,
                            std::path::Component::Normal(_) | std::path::Component::CurDir
                        )
                    }) =>
            {
                // Cargo may print workspace-relative paths from a member cwd.
                // Only a missing safe relative path searches bounded ancestors.
                for parent in provenance.cwd.ancestors().skip(1).take(32) {
                    check_cancel(cancelled)?;
                    let candidate = parent.join(original);
                    if std::fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
                        path = Some(candidate);
                        break;
                    }
                }
            }
            _ => (),
        }
        validate_store(&store.lock().unwrap(), bounds)?;
        return Ok(path.map(|path| Resolved {
            link: FileLink {
                path,
                line: Some(location.line),
                column: location.column,
                byte_column: location.byte_column,
                read_only: false,
                other_pane: false,
            },
            relative_guard,
        }));
    }
    if context::rust_marker(&line) {
        return Ok(None);
    }
    let ranges = candidates(&line, byte);
    let mut relative = Some(if saved_click.is_some() {
        completion
    } else {
        None
    });
    let mut probes = 0;
    for range in ranges {
        check_cancel(cancelled)?;
        if !range.contains(&byte) {
            continue;
        }
        for (path, location) in parse_candidate(&line[range]) {
            let path = Path::new(&path);
            let absolute = path.is_absolute();
            if !absolute {
                let allowed = if let Some(allowed) = relative {
                    allowed
                } else {
                    let allowed = if saved_click.is_some() {
                        completion
                    } else {
                        None
                    };
                    relative = Some(allowed);
                    allowed
                };
                if allowed.is_none() {
                    continue;
                }
            }
            let path = if absolute {
                path.to_path_buf()
            } else {
                provenance.cwd.join(path)
            };
            if probes == MAX_CANDIDATES {
                return Ok(None);
            }
            probes += 1;
            if path.is_file() {
                check_cancel(cancelled)?;
                // A stale click must not publish after a concurrent Clear or
                // ring eviction, even if the filesystem probe succeeded.
                validate_store(&store.lock().unwrap(), bounds)?;
                return Ok(Some(Resolved {
                    link: FileLink {
                        path,
                        line: location.map(|v| v.0),
                        column: location.and_then(|v| v.1),
                        byte_column: false,
                        read_only: false,
                        other_pane: false,
                    },
                    relative_guard: if absolute { None } else { relative.flatten() },
                }));
            }
        }
    }
    Ok(None)
}

fn logical_line(
    click: Anchor,
    store: &SharedTranscript,
    source: &mut impl TextSource,
    provenance: &Provenance,
    cancelled: &AtomicBool,
) -> io::Result<Option<(String, usize)>> {
    let bounds = source.bounds();
    let row = source.text(click.record_id)?;
    if click.utf8_byte_offset >= row.text.len()
        || !row.text.is_char_boundary(click.utf8_byte_offset)
    {
        return Ok(None);
    }
    let mut bytes = row.text.len();
    if bytes > MAX_LINE_BYTES {
        return Ok(None);
    }
    let mut rows = vec![(click.record_id, row)];
    let mut first = click.record_id;
    while first > bounds.first {
        check_cancel(cancelled)?;
        let id = first - 1;
        if metadata(store, bounds, id, cancelled)?
            .as_ref()
            .is_some_and(|record| !terminal_origin(record, provenance))
        {
            break;
        }
        let previous = source.text(id)?;
        if !previous.join_next {
            break;
        }
        bytes += previous.text.len();
        if rows.len() == MAX_LINE_ROWS || bytes > MAX_LINE_BYTES {
            return Ok(None);
        }
        rows.push((id, previous));
        first = id;
    }
    rows.reverse();
    let mut next = click.record_id + 1;
    while rows.last().unwrap().1.join_next && next < bounds.end {
        check_cancel(cancelled)?;
        if metadata(store, bounds, next, cancelled)?
            .as_ref()
            .is_some_and(|record| !terminal_origin(record, provenance))
        {
            break;
        }
        let row = source.text(next)?;
        bytes += row.text.len();
        if rows.len() == MAX_LINE_ROWS || bytes > MAX_LINE_BYTES {
            return Ok(None);
        }
        rows.push((next, row));
        next += 1;
    }
    let mut text = String::with_capacity(bytes);
    let mut byte = click.utf8_byte_offset;
    for (id, row) in rows {
        if id < click.record_id {
            byte += row.text.len();
        }
        text.push_str(&row.text);
    }
    Ok(Some((text, byte)))
}

/// Origin/final equality is the same launch-directory heuristic as :term.
/// It cannot prove a program did not chdir internally and return. Explicit
/// helper cwd changes veto it, and unfinished/evicted sources never qualify.
fn source_info(
    store: &SharedTranscript,
    bounds: Bounds,
    click: u64,
    provenance: &Provenance,
    cancelled: &AtomicBool,
) -> io::Result<(context::Context, Option<Anchor>)> {
    // The current source keeps exactly one bounded command context for color
    // and underline classification. Its header can evict during a long run;
    // reuse that same context, never a growing historical source map.
    let mut context = store
        .lock()
        .unwrap()
        .current_command_context(provenance.source)
        .map_or_else(context::Context::default, |command| command.0);
    let mut safe = provenance.relative_safe;
    let mut first = click;
    let mut visited = 0;
    while first > bounds.first {
        visited += 1;
        if visited >= MAX_SOURCE_ROWS {
            return Ok((context, None));
        }
        let Some(record) = metadata(store, bounds, first - 1, cancelled)? else {
            first -= 1;
            continue;
        };
        if record.source != provenance.source {
            break;
        }
        safe &= record.cwd == provenance.cwd
            && matches!(
                record.data,
                RecordData::Terminal { .. } | RecordData::Header(_)
            );
        if let RecordData::Header(command) = record.data
            && record.cwd == provenance.cwd
        {
            context = context::Context::new(&command);
        }
        first -= 1;
    }
    for id in click..bounds.end {
        visited += 1;
        if visited > MAX_SOURCE_ROWS {
            return Ok((context, None));
        }
        let Some(record) = metadata(store, bounds, id, cancelled)? else {
            return Ok((context, None));
        };
        if record.source != provenance.source {
            return Ok((context, None));
        }
        safe &= record.cwd == provenance.cwd;
        match record.data {
            RecordData::Result { relative_safe, .. } => {
                return Ok((
                    context,
                    (safe && relative_safe).then_some(Anchor {
                        epoch: record.epoch,
                        record_id: record.id,
                        utf8_byte_offset: 0,
                    }),
                ));
            }
            RecordData::Terminal { .. } | RecordData::Header(_) => (),
            RecordData::Native(_) => return Ok((context, None)),
        }
    }
    Ok((context, None))
}

fn add_range(ranges: &mut Vec<Range<usize>>, range: Range<usize>, click: usize) {
    if ranges.len() < MAX_CANDIDATES && range.contains(&click) && !ranges.contains(&range) {
        ranges.push(range);
    }
}
fn message_prefix(text: &str) -> bool {
    let prefix = text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(':')
        .to_ascii_lowercase();
    matches!(
        prefix.as_str(),
        "error" | "warning" | "note" | "fatal" | "-->" | "at" | "file"
    )
}
fn path_starts(text: &str, end: usize) -> Vec<usize> {
    let mut starts = vec![];
    let mut start = text.len() - text.trim_start().len();
    if start < end && !message_prefix(&text[start..end]) {
        starts.push(start);
    }
    for (index, character) in text[..end].char_indices() {
        if character.is_whitespace() {
            start = index + character.len_utf8();
        } else if start == index && starts.last() != Some(&start) {
            starts.push(start);
        }
    }
    starts.retain(|start| !message_prefix(&text[*start..end]));
    if starts.len() > 8 {
        starts.drain(1..starts.len() - 7);
    }
    starts
}
fn candidates(text: &str, click: usize) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let start = text.len() - text.trim_start().len();
    let end = text.trim_end().len();
    if !message_prefix(&text[start..end]) {
        add_range(&mut ranges, start..end, click);
    }
    // Quote-delimited names can contain spaces, Unicode and colons. Include a
    // numeric suffix outside the closing quote without swallowing the message.
    let mut opening = None;
    for (index, character) in text.char_indices() {
        if !matches!(character, '\'' | '"') {
            continue;
        }
        match opening {
            Some((first, quote)) if quote == character => {
                let end = index + character.len_utf8();
                let suffix = location_regex()
                    .find(&text[end..])
                    .filter(|m| m.start() == 0 && suffix_boundary(text, end + m.end()));
                if let Some(suffix) = suffix {
                    add_range(&mut ranges, first..end + suffix.end(), click);
                }
                add_range(&mut ranges, first..end, click);
                opening = None;
            }
            None => opening = Some((index, character)),
            _ => {}
        }
    }
    for matched in location_regex().find_iter(text).take(MAX_CANDIDATES) {
        if !suffix_boundary(text, matched.end()) {
            continue;
        }
        for start in path_starts(text, matched.start()) {
            add_range(&mut ranges, start..matched.end(), click);
        }
    }
    for matched in parenthesis_regex().find_iter(text).take(MAX_CANDIDATES) {
        if !suffix_boundary(text, matched.end()) {
            continue;
        }
        for start in path_starts(text, matched.start()) {
            add_range(&mut ranges, start..matched.end(), click);
        }
    }
    let mut token = 0;
    for (index, character) in text.char_indices() {
        if character.is_whitespace() {
            add_range(&mut ranges, token..index, click);
            token = index + character.len_utf8();
        }
    }
    add_range(&mut ranges, token..text.len(), click);
    ranges
}
fn suffix_boundary(text: &str, end: usize) -> bool {
    text[end..].chars().next().is_none_or(|character| {
        character.is_whitespace() || matches!(character, ':' | '\'' | '"' | ')' | ']' | ',' | ';')
    })
}
fn location_regex() -> &'static regex::Regex {
    static VALUE: OnceLock<regex::Regex> = OnceLock::new();
    VALUE.get_or_init(|| regex::Regex::new(r":[1-9][0-9]*(?::[1-9][0-9]*)?").unwrap())
}
fn parenthesis_regex() -> &'static regex::Regex {
    static VALUE: OnceLock<regex::Regex> = OnceLock::new();
    VALUE.get_or_init(|| regex::Regex::new(r"\([1-9][0-9]*(?:,[1-9][0-9]*)?\)").unwrap())
}
type Location = Option<(usize, Option<usize>)>;
fn clean_path(path: &str) -> &str {
    path.trim().trim_matches(['\'', '"', '[', ']', ',', ';'])
}
fn parse_candidate(value: &str) -> Vec<(String, Location)> {
    let value = value.trim().trim_matches([',', ';']);
    let value = if value.len() >= 2
        && ((value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    };
    let mut result = vec![(clean_path(value).to_owned(), None)];
    if let Some((path, last)) = value.rsplit_once(':') {
        if let Ok(last) = last.parse::<usize>()
            && last > 0
        {
            result.push((clean_path(path).to_owned(), Some((last, None))));
            if let Some((path, line)) = path.rsplit_once(':') {
                if let Ok(line) = line.parse::<usize>()
                    && line > 0
                {
                    result.push((clean_path(path).to_owned(), Some((line, Some(last)))));
                }
            }
        }
    }
    if let Some(open) = value.rfind('(')
        && value.ends_with(')')
    {
        let numbers = &value[open + 1..value.len() - 1];
        let (line, column) = numbers
            .split_once(',')
            .map_or((numbers, None), |(line, col)| (line, Some(col)));
        if let Ok(line) = line.parse::<usize>()
            && line > 0
        {
            let column = column
                .and_then(|column| column.parse::<usize>().ok())
                .filter(|column| *column > 0);
            result.push((clean_path(&value[..open]).to_owned(), Some((line, column))));
        }
    }
    result.retain(|(path, _)| !path.is_empty() && !path.contains('\0') && !path.contains("://"));
    result
}

#[cfg(test)]
#[path = "diagnostic_links_tests.rs"]
mod tests;
