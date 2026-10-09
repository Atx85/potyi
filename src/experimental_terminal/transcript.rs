// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! One capped disk transcript shared by native browsing and terminal output.
//! Record positions are transient; anchors use an epoch and a monotonic ID.
use super::{
    history::History,
    output_colors::{self, Colors, Decoration, Tone},
};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use vt100::ScrollbackSink;

const MAX_RECORD_BYTES: usize = 256 * 1024;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_READ_ROWS: usize = 256;
const MAX_READ_BYTES: usize = 256 * 1024;
const HEADER_BYTES: usize = 24;
pub(crate) type SharedTranscript = Arc<Mutex<Transcript>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Anchor {
    pub(crate) epoch: u64,
    pub(crate) record_id: u64,
    pub(crate) utf8_byte_offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RecordData {
    Native(Vec<u8>),
    Terminal {
        cols: u16,
        wrapped: bool,
        formatted: Vec<u8>,
    },
    Header(String),
    Result {
        relative_safe: bool,
        status: i32,
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Record {
    pub(crate) id: u64,
    pub(crate) epoch: u64,
    pub(crate) source: u64,
    pub(crate) cwd: PathBuf,
    pub(crate) data: RecordData,
}

pub(crate) struct Transcript {
    history: History,
    epoch: u64,
    revision: u64,
    source: u64,
    failed: bool,
    error: Option<String>,
    colors: Colors,
}

impl Transcript {
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self {
            history: History::new()?,
            epoch: 1,
            revision: 0,
            source: 0,
            failed: false,
            error: None,
            colors: Colors::default(),
        })
    }
    pub(crate) fn shared() -> io::Result<SharedTranscript> {
        Ok(Arc::new(Mutex::new(Self::new()?)))
    }
    pub(crate) fn len(&self) -> usize {
        self.history.len()
    }
    pub(crate) fn base_id(&self) -> u64 {
        self.history.serial - self.history.len() as u64
    }
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Mutation revision, rather than next record ID. Clear changes it even if
    /// there were no records. Use base_id/read_id for record identity.
    pub(crate) fn serial(&self) -> u64 {
        self.revision
    }
    pub(crate) fn next_source(&mut self) -> u64 {
        self.source = self
            .source
            .checked_add(1)
            .expect("Transcript source ID exhausted");
        self.source
    }
    pub(crate) fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }
    pub(crate) fn flush(&mut self) -> io::Result<()> {
        let result = self.history.flush();
        self.check_io(result)
    }
    pub(crate) fn clear(&mut self) -> io::Result<()> {
        let next_epoch = self
            .epoch
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Transcript epoch exhausted"))?;
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Transcript revision exhausted"))?;
        let result = self.history.clear();
        self.check_io(result)?;
        self.epoch = next_epoch;
        self.revision = next_revision;
        self.colors.clear();
        Ok(())
    }
    pub(crate) fn append_native(
        &mut self,
        cwd: &Path,
        source: u64,
        bytes: &[u8],
    ) -> io::Result<Anchor> {
        self.append(cwd, source, 0, 0, false, 0, bytes)
    }
    pub(crate) fn append_terminal(
        &mut self,
        cwd: &Path,
        source: u64,
        cols: u16,
        wrapped: bool,
        formatted: &[u8],
    ) -> io::Result<Anchor> {
        if cols == 0 {
            return Err(io::Error::other("Terminal row must have a width"));
        }
        self.validate_append(cwd, 1, formatted)?;
        let mut parser = vt100::Parser::new(1, cols.clamp(1, 256), 0);
        parser.process(formatted);
        let change = self.colors.row(
            source,
            &parser.screen().contents(),
            wrapped,
            Some(self.history.serial),
        );
        let anchor =
            self.append_styled(cwd, source, 1, cols, wrapped, 0, formatted, change.current)?;
        for (id, decoration) in change.previous {
            self.set_terminal_style(id, decoration)?;
        }
        Ok(anchor)
    }
    /// Literal legacy command text. Zero columns are decoded as plain UTF-8
    /// only when the durable raw flag is present; ordinary VT rows stay strict.
    pub(crate) fn append_plain(
        &mut self,
        cwd: &Path,
        source: u64,
        text: &str,
        continued: bool,
    ) -> io::Result<Anchor> {
        self.validate_append(cwd, 1, text.as_bytes())?;
        let change = self
            .colors
            .row(source, text, continued, Some(self.history.serial));
        let anchor = self.append_styled(
            cwd,
            source,
            1,
            0,
            continued,
            0,
            text.as_bytes(),
            change.current,
        )?;
        for (id, decoration) in change.previous {
            self.set_terminal_style(id, decoration)?;
        }
        Ok(anchor)
    }
    pub(crate) fn append_header(
        &mut self,
        cwd: &Path,
        source: u64,
        text: &str,
    ) -> io::Result<Anchor> {
        let anchor = self.append(cwd, source, 2, 0, false, 0, text.as_bytes())?;
        self.colors.begin(source, text);
        Ok(anchor)
    }
    /// Only the logical prefix/annotation context is cloned for a bounded
    /// live-tail projection. Historical tones remain in the disk row header.
    pub(super) fn color_preview(&self, source: u64) -> Colors {
        self.colors.preview(source)
    }
    pub(super) fn current_command_context(
        &self,
        source: u64,
    ) -> Option<super::diagnostic_links::CommandContext> {
        self.colors.current_command_context(source)
    }
    fn set_terminal_style(&mut self, id: u64, decoration: Decoration) -> io::Result<()> {
        if id < self.base_id() {
            return Ok(());
        }
        let index = usize::try_from(id - self.base_id()).map_err(io::Error::other)?;
        let mut bytes = self.history.row(index)?.bytes;
        if bytes.len() < HEADER_BYTES || bytes[1] != 1 {
            return Err(io::Error::other("Style anchor is not a terminal row"));
        }
        let (tone, start, end) = style_metadata(bytes.len(), decoration);
        if bytes[3] != tone
            || bytes[6..8] != start.to_le_bytes()
            || bytes[20..24] != end.to_le_bytes()
        {
            bytes[3] = tone;
            bytes[6..8].copy_from_slice(&start.to_le_bytes());
            bytes[20..24].copy_from_slice(&end.to_le_bytes());
            let result = self.history.replace(index, &bytes);
            self.check_io(result)?;
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or_else(|| io::Error::other("Transcript revision exhausted"))?;
        }
        Ok(())
    }
    pub(crate) fn append_result(
        &mut self,
        cwd: &Path,
        source: u64,
        status: i32,
        text: &str,
    ) -> io::Result<Anchor> {
        self.append_result_with_provenance(cwd, source, status, text, true)
    }
    pub(crate) fn append_result_with_provenance(
        &mut self,
        cwd: &Path,
        source: u64,
        status: i32,
        text: &str,
        relative_safe: bool,
    ) -> io::Result<Anchor> {
        self.append(cwd, source, 3, 0, !relative_safe, status, text.as_bytes())
    }
    pub(crate) fn invalidate_result(&mut self, anchor: Anchor) -> io::Result<()> {
        if self.failed || self.revision == u64::MAX {
            return Err(io::Error::other("Transcript storage is unavailable"));
        }
        let record = self.read_anchor(anchor)?;
        if !matches!(record.data, RecordData::Result { .. }) {
            return Err(io::Error::other(
                "Transcript record is not a command result",
            ));
        }
        let index = usize::try_from(anchor.record_id - self.base_id()).map_err(io::Error::other)?;
        let mut bytes = self.history.row(index)?.bytes;
        if bytes[2] == 0 {
            bytes[2] = 1;
            let result = self.history.replace(index, &bytes);
            self.check_io(result)?;
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or_else(|| io::Error::other("Transcript revision exhausted"))?;
        }
        Ok(())
    }
    /// Persist native decoration without shifting any stable record identity.
    /// Replacement must preserve the encoded length, so it cannot alter the ring.
    pub(crate) fn replace_native(&mut self, id: u64, payload: &[u8]) -> io::Result<()> {
        if self.failed || self.revision == u64::MAX {
            return Err(io::Error::other("Transcript storage is unavailable"));
        }
        let record = self.read_id(id)?;
        let RecordData::Native(previous) = &record.data else {
            return Err(io::Error::other("Transcript record is not native"));
        };
        if previous.len() != payload.len() {
            return Err(io::Error::other(
                "Transcript replacement changes record length",
            ));
        }
        let index = usize::try_from(id - self.base_id()).map_err(io::Error::other)?;
        let mut bytes = self.history.row(index)?.bytes;
        let start = bytes.len() - payload.len();
        bytes[start..].copy_from_slice(payload);
        let result = self.history.replace(index, &bytes);
        self.check_io(result)?;
        self.revision += 1;
        Ok(())
    }
    fn append(
        &mut self,
        cwd: &Path,
        source: u64,
        tag: u8,
        cols: u16,
        wrapped: bool,
        status: i32,
        payload: &[u8],
    ) -> io::Result<Anchor> {
        self.append_styled(
            cwd,
            source,
            tag,
            cols,
            wrapped,
            status,
            payload,
            Decoration {
                tone: None,
                underline: None,
            },
        )
    }
    fn validate_append(&self, cwd: &Path, tag: u8, payload: &[u8]) -> io::Result<usize> {
        let limit = MAX_PAYLOAD_BYTES + if tag == 2 { 2 } else { 0 };
        if payload.len() > limit {
            return Err(io::Error::other(
                "Transcript payload exceeds its storage limit",
            ));
        }
        if self.failed {
            return Err(io::Error::other("Transcript storage is unavailable"));
        }
        if !cwd.is_absolute() {
            return Err(io::Error::other("Transcript directory must be absolute"));
        }
        if self.history.serial == u64::MAX || self.revision == u64::MAX {
            return Err(io::Error::other("Transcript record ID exhausted"));
        }
        HEADER_BYTES
            .checked_add(native_len(cwd))
            .and_then(|n| n.checked_add(payload.len()))
            .filter(|n| *n <= MAX_RECORD_BYTES)
            .ok_or_else(|| io::Error::other("Transcript record exceeds 256 KiB including metadata"))
    }
    fn append_styled(
        &mut self,
        cwd: &Path,
        source: u64,
        tag: u8,
        cols: u16,
        wrapped: bool,
        status: i32,
        payload: &[u8],
        decoration: Decoration,
    ) -> io::Result<Anchor> {
        let size = self.validate_append(cwd, tag, payload)?;
        let directory = native_bytes(cwd);
        // Validate the complete encoded size before touching disk or evicting rows.
        let mut bytes = Vec::with_capacity(size);
        let (tone, start, end) = if tag == 1 {
            style_metadata(size, decoration)
        } else {
            (0, 0, 0)
        };
        bytes.extend_from_slice(&[
            1,
            tag,
            u8::from(wrapped) | if tag == 1 && cols == 0 { 2 } else { 0 },
            tone,
        ]);
        bytes.extend_from_slice(&cols.to_le_bytes());
        bytes.extend_from_slice(&start.to_le_bytes());
        bytes.extend_from_slice(&source.to_le_bytes());
        bytes.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&if tag == 1 {
            end.to_le_bytes()
        } else {
            status.to_le_bytes()
        });
        bytes.extend_from_slice(&directory);
        bytes.extend_from_slice(payload);
        let id = self.history.serial;
        self.history.push(0, false, &bytes);
        if let Some(error) = self.history.error.take() {
            self.failed = true;
            self.error = Some(error.clone());
            return Err(io::Error::other(error));
        }
        self.revision += 1;
        Ok(Anchor {
            epoch: self.epoch,
            record_id: id,
            utf8_byte_offset: 0,
        })
    }
    pub(crate) fn read(&mut self, index: usize) -> io::Result<Record> {
        if index >= self.len() {
            return Err(io::Error::other("Transcript row is out of range"));
        }
        let id = self.base_id() + index as u64;
        let bytes = self.history.row(index)?.bytes;
        decode(self.epoch, id, &bytes)
    }
    pub(crate) fn read_id(&mut self, id: u64) -> io::Result<Record> {
        let index = id
            .checked_sub(self.base_id())
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < self.len())
            .ok_or_else(|| io::Error::other("Transcript anchor has been evicted or cleared"))?;
        self.read(index)
    }
    pub(crate) fn read_anchor(&mut self, anchor: Anchor) -> io::Result<Record> {
        if anchor.epoch != self.epoch {
            return Err(io::Error::other(
                "Transcript anchor belongs to a cleared view",
            ));
        }
        self.read_id(anchor.record_id)
    }
    pub(crate) fn read_rows(&mut self, start: usize, count: usize) -> io::Result<Vec<Record>> {
        let end = start
            .saturating_add(count.min(MAX_READ_ROWS))
            .min(self.len());
        let mut records = Vec::with_capacity(end.saturating_sub(start));
        let mut bytes = 0;
        for index in start..end {
            let record = self.read(index)?;
            let size = record_size(&record);
            if bytes + size > MAX_READ_BYTES {
                break;
            }
            bytes += size;
            records.push(record);
        }
        Ok(records)
    }
    fn check_io(&mut self, result: io::Result<()>) -> io::Result<()> {
        if let Err(error) = &result {
            self.failed = true;
            self.error = Some(format!("Transcript storage failed: {error}"));
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn buffer_capacity(&self) -> usize {
        self.history.buffer_capacity()
    }
}

fn style_metadata(size: usize, decoration: Decoration) -> (u8, u16, i32) {
    // Decoded presentation prefixes still fit the single/batch-read byte cap.
    if size > MAX_READ_BYTES - output_colors::MAX_STYLE_BYTES {
        return (0, 0, 0);
    }
    let range = decoration
        .underline
        .filter(|range| range.start < range.end && range.end <= u16::MAX as usize)
        .unwrap_or(0..0);
    (
        decoration.tone.map_or(0, |tone| tone as u8),
        range.start as u16,
        range.end as i32,
    )
}

/// Batch reads can return fewer rows than requested. Metadata and payload both
/// count toward the byte budget; a valid individual record always fits.
fn record_size(record: &Record) -> usize {
    HEADER_BYTES
        + native_bytes(&record.cwd).len()
        + match &record.data {
            RecordData::Native(bytes)
            | RecordData::Terminal {
                formatted: bytes, ..
            } => bytes.len(),
            RecordData::Header(text) | RecordData::Result { text, .. } => text.len(),
        }
}

fn decode(epoch: u64, id: u64, bytes: &[u8]) -> io::Result<Record> {
    if bytes.len() < HEADER_BYTES || bytes[0] != 1 {
        return Err(io::Error::other("Invalid transcript record"));
    }
    let directory_len = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let payload_start = HEADER_BYTES
        .checked_add(directory_len)
        .filter(|n| *n <= bytes.len())
        .ok_or_else(|| io::Error::other("Invalid transcript metadata"))?;
    let cwd = native_path(&bytes[HEADER_BYTES..payload_start])?;
    if !cwd.is_absolute() {
        return Err(io::Error::other("Invalid transcript directory"));
    }
    let payload = &bytes[payload_start..];
    let data = match bytes[1] {
        0 => RecordData::Native(payload.into()),
        1 => {
            let cols = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
            let raw = bytes[2] & 2 != 0;
            if (cols == 0) != raw || bytes[2] & !3 != 0 {
                return Err(io::Error::other("Invalid terminal row width/format"));
            }
            if raw {
                std::str::from_utf8(payload).map_err(io::Error::other)?;
            }
            RecordData::Terminal {
                cols,
                wrapped: bytes[2] & 1 != 0,
                formatted: output_colors::formatted(
                    Tone::from_byte(bytes[3]),
                    {
                        let start = u16::from_le_bytes(bytes[6..8].try_into().unwrap()) as usize;
                        let end = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;
                        (start < end && end <= MAX_PAYLOAD_BYTES).then_some(start..end)
                    },
                    payload,
                ),
            }
        }
        2 => RecordData::Header(
            std::str::from_utf8(payload)
                .map_err(io::Error::other)?
                .into(),
        ),
        3 => RecordData::Result {
            relative_safe: bytes[2] == 0,
            status: i32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            text: std::str::from_utf8(payload)
                .map_err(io::Error::other)?
                .into(),
        },
        _ => return Err(io::Error::other("Invalid transcript record type")),
    };
    Ok(Record {
        id,
        epoch,
        source: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        cwd,
        data,
    })
}

fn native_len(path: &Path) -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().len()
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str().encode_wide().count().saturating_mul(2)
    }
    #[cfg(not(any(unix, windows)))]
    {
        path.to_string_lossy().len()
    }
}
fn native_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect()
    }
    #[cfg(not(any(unix, windows)))]
    {
        path.to_string_lossy().as_bytes().to_vec()
    }
}
fn native_path(bytes: &[u8]) -> io::Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(bytes.into()).into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        if bytes.len() % 2 != 0 {
            return Err(io::Error::other("Invalid transcript native path"));
        }
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        Ok(std::ffi::OsString::from_wide(&words).into())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(PathBuf::from(
            std::str::from_utf8(bytes).map_err(io::Error::other)?,
        ))
    }
}

#[cfg(test)]
#[path = "transcript_tests.rs"]
mod tests;
