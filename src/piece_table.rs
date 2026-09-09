// Pötyi - Lightweight text editor
// Copyright (C) 2026  Attila Banko
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


use std::fs::{
    self,
    File,
    OpenOptions,
};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::time::Instant;


const PIECE_TABLE_CHUNK_SIZE: usize =
    64 * 1024;


#[cfg(windows)]
use std::os::windows::fs::FileExt;

#[cfg(unix)]
use std::os::unix::fs::{
    FileExt,
    OpenOptionsExt,
};

fn read_at(
    file: &File,
    buffer: &mut [u8],
    offset: u64,
) -> io::Result<usize> {
    #[cfg(windows)]
    {
        file.seek_read(buffer, offset)
    }

    #[cfg(unix)]
    {
        file.read_at(buffer, offset)
    }
}

fn write_at(
    file: &File,
    buffer: &[u8],
    offset: u64,
) -> io::Result<usize> {
    #[cfg(windows)]
    {
        file.seek_write(buffer, offset)
    }

    #[cfg(unix)]
    {
        file.write_at(buffer, offset)
    }
}

fn write_all_at(
    file: &File,
    mut buffer: &[u8],
    mut offset: u64,
) -> io::Result<()> {
    while !buffer.is_empty() {
        let written =
            write_at(
                file,
                buffer,
                offset,
            )?;

        if written == 0 {
            return Err(
                io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to append to edit store",
                )
            );
        }

        buffer = &buffer[written..];
        offset = offset
            .checked_add(written as u64)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "edit store offset overflow",
                )
            })?;
    }

    Ok(())
}


// ==========================================================================
// File-backed edit store
// ==========================================================================

/// Append-only storage for inserted text.
///
/// Only the path, file handle, and current length stay in Pötyi's heap.
/// The operating system may cache active pages, but those pages remain
/// reclaimable instead of becoming a permanently growing `Vec<u8>`.
struct EditStore {
    file: Option<File>,
    path: PathBuf,
    length: usize,
}

impl EditStore {
    fn create() -> io::Result<Self> {
        static NEXT_FILE: AtomicU64 =
            AtomicU64::new(0);

        for _ in 0..1024 {
            let number =
                NEXT_FILE.fetch_add(
                    1,
                    Ordering::Relaxed,
                );

            let path =
                std::env::temp_dir().join(
                    format!(
                        "potyi-edits-{}-{number}.tmp",
                        std::process::id(),
                    )
                );

            let mut options =
                OpenOptions::new();

            options
                .read(true)
                .write(true)
                .create_new(true);

            #[cfg(unix)]
            options.mode(0o600);

            match options.open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        file: Some(file),
                        path,
                        length: 0,
                    });
                }

                Err(error)
                    if error.kind()
                        == io::ErrorKind::AlreadyExists => {}

                Err(error) => return Err(error),
            }
        }

        Err(
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "could not create a unique Pötyi edit store",
            )
        )
    }

    fn file(&self) -> &File {
        self.file
            .as_ref()
            .expect("edit store file must remain open")
    }

    fn len(&self) -> usize {
        self.length
    }

    fn append(
        &mut self,
        bytes: &[u8],
    ) -> io::Result<usize> {
        let start = self.length;

        let new_length =
            start.checked_add(bytes.len())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "edit store length overflow",
                    )
                })?;

        write_all_at(
            self.file(),
            bytes,
            start as u64,
        )?;

        self.length = new_length;

        Ok(start)
    }

    fn read_into(
        &self,
        position: usize,
        buffer: &mut [u8],
    ) -> io::Result<usize> {
        let end =
            position.checked_add(buffer.len())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "edit store range overflow",
                    )
                })?;

        if end > self.length {
            return Err(
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "edit store range is outside the file",
                )
            );
        }

        let mut total = 0usize;

        while total < buffer.len() {
            let read =
                read_at(
                    self.file(),
                    &mut buffer[total..],
                    (position + total) as u64,
                )?;

            if read == 0 {
                break;
            }

            total += read;
        }

        Ok(total)
    }
}

impl Drop for EditStore {
    fn drop(&mut self) {
        // Windows cannot remove an open file, so close it explicitly first.
        drop(self.file.take());
        let _ = fs::remove_file(&self.path);
    }
}


// ==========================================================================
// Piece table
// ==========================================================================

#[derive(Clone, Copy)]
pub struct Piece {
    pub start: usize,
    pub length: usize,
    pub original: bool,
}

/// An opaque piece-table state used to swap a batch edit in and out.
///
/// The add buffer is append-only, so retaining only the previous pieces and
/// logical length is enough to make replace-all undo and redo constant-time.
pub(crate) struct PieceTableSnapshot {
    pieces: Vec<Piece>,
    length: usize,
}

// ==========================================================================
// Cursor
// ==========================================================================

pub struct Cursor {
    pub position: usize,
    pub anchor: usize,
    pub desired_column: Option<usize>,

    pub line: usize,
    pub column: usize,
    pub anchor_line: usize,
    pub anchor_column: usize,
}

// ==========================================================================
// Cached line information
// ==========================================================================

#[derive(Clone, Copy)]
struct LineInfo {
    start: usize,
    end: usize,
    length: usize,
}

// ==========================================================================
// Piece table
// ==========================================================================

pub struct PieceTable {
    original: File,
    original_length: usize,

    add: EditStore,
    pub pieces: Vec<Piece>,
    pub length: usize,

    line_cache: Vec<LineInfo>,
    all_lines_cached: bool,

    pub(crate) cursor: Cursor,
}

impl PieceTable {
    // ---------------------------------------------------------------------
    // Basic
    // ---------------------------------------------------------------------

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[cfg(test)]
    pub(crate) fn edit_store_len(
        &self,
    ) -> usize {
        self.add.len()
    }

    /// Number of lines currently known by the lazy line cache.
    ///
    /// This does NOT scan the remainder of the document.
    pub fn cached_line_count(&self) -> usize {
        self.line_cache.len().max(1)
    }

    // ---------------------------------------------------------------------
    // Open
    // ---------------------------------------------------------------------

    pub fn open(path: &str) -> io::Result<Self> {
        let started = Instant::now();

        let original = File::open(path)?;

        let original_length = usize::try_from(
            original.metadata()?.len(),
        )
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "file is too large for this platform",
            )
        })?;

        let pieces =
            if original_length == 0 {
                Vec::new()
            } else {
                vec![
                    Piece {
                        start: 0,
                        length: original_length,
                        original: true,
                    }
                ]
            };

        let table = Self {
            original,
            original_length,

            add: EditStore::create()?,
            pieces,
            length: original_length,

            // First line is a lazy placeholder.
line_cache: if original_length == 0 {
    vec![LineInfo {
        start: 0,
        end: 0,
        length: 0,
    }]
} else {
    Vec::new()
},

all_lines_cached: original_length == 0,

            cursor: Cursor {
                position: 0,
                anchor: 0,
                desired_column: None,

                line: 0,
                column: 0,

                anchor_line: 0,
                anchor_column: 0,
            },
        };

        println!(
            "PieceTable::open({}): {:?}",
            path,
            started.elapsed()
        );

        Ok(table)
    }

    /// Creates a new empty document with no backing file.
    ///
    /// The original file handle points at the platform null device so the
    /// piece table can retain its file-backed design without allocating a
    /// document-sized buffer.
    pub fn empty() -> io::Result<Self> {
        let original = if cfg!(windows) {
            File::open("NUL")?
        } else {
            File::open("/dev/null")?
        };

        Ok(Self {
            original,
            original_length: 0,

            add: EditStore::create()?,
            pieces: Vec::new(),
            length: 0,

            line_cache: Vec::new(),
            all_lines_cached: false,

            cursor: Cursor {
                position: 0,
                anchor: 0,
                desired_column: None,

                line: 0,
                column: 0,
                anchor_line: 0,
                anchor_column: 0,
            },
        })
    }

    pub fn move_cursor_to_line_column(
    &mut self,
    line: usize,
    column: usize,
) -> io::Result<()> {
    self.ensure_line_cached(line)?;

    let start = self.line_start(line)?;

    let position = self.position_at_column(
        start,
        column,
    )?;

    self.move_cursor(position)
}
    // ---------------------------------------------------------------------
    // Saving
    // ---------------------------------------------------------------------

    fn temporary_path(
        path: &Path,
    ) -> PathBuf {
        let file_name =
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("potyi");

        let mut temporary =
            path.to_path_buf();

        temporary.set_file_name(
            format!(
                ".{file_name}.potyi.tmp"
            ),
        );

        temporary
    }

    pub fn write_to(
        &self,
        path: &Path,
    ) -> io::Result<()> {
        self.write_to_destination(path, true)
    }

    pub fn write_to_new(&self, path: &Path) -> io::Result<()> {
        self.write_to_destination(path, false)
    }

    fn write_to_destination(&self, path: &Path, overwrite: bool) -> io::Result<()> {
        let temporary_path =
            Self::temporary_path(path);

        let mut output =
            File::create(&temporary_path)?;

        let mut buffer =
            [0u8; 64 * 1024];

        for piece in &self.pieces {
            if piece.length == 0 {
                continue;
            }

            if piece.original {
                let mut remaining =
                    piece.length;

                let mut source_position =
                    piece.start;

                while remaining > 0 {
                    let amount =
                        remaining.min(
                            buffer.len()
                        );

                    let read =
                        read_at(
                            &self.original,
                            &mut buffer[..amount],
                            source_position as u64,
                        )?;

                    if read != amount {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected end of original file",
                            )
                        );
                    }

                    output.write_all(
                        &buffer[..read],
                    )?;

                    source_position += read;
                    remaining -= read;
                }
            } else {
                let end =
                    piece.start
                        .checked_add(piece.length)
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "piece range overflow",
                            )
                        })?;

                if end > self.add.len() {
                    return Err(
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece references data outside add buffer",
                        )
                    );
                }

                let mut remaining =
                    piece.length;

                let mut source_position =
                    piece.start;

                while remaining > 0 {
                    let amount =
                        remaining.min(
                            buffer.len()
                        );

                    let read =
                        self.add.read_into(
                            source_position,
                            &mut buffer[..amount],
                        )?;

                    if read != amount {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected end of edit store",
                            )
                        );
                    }

                    output.write_all(
                        &buffer[..read],
                    )?;

                    source_position += read;
                    remaining -= read;
                }
            }
        }

        output.sync_all()?;
        drop(output);

        if !overwrite {
            // Publish the completed file without replacing a destination that
            // already exists, including one created while we were writing.
            let result = fs::hard_link(&temporary_path, path);
            let _ = fs::remove_file(&temporary_path);
            return result;
        }

        if path.exists() {
            fs::remove_file(path)?;
        }

        fs::rename(
            temporary_path,
            path,
        )?;

        Ok(())
    }

    // ---------------------------------------------------------------------
    // Reading
    // ---------------------------------------------------------------------

    /// Visits the document in logical order without reconstructing it.
    ///
    /// Add-buffer pieces are borrowed directly. Original-file pieces use one
    /// fixed-size buffer whose contents are valid for the duration of each
    /// callback.
    pub(crate) fn visit_chunks<F>(
        &self,
        mut visit: F,
    ) -> io::Result<()>
    where
        F: FnMut(&[u8]) -> io::Result<()>,
    {
        let mut buffer =
            vec![0u8;
                PIECE_TABLE_CHUNK_SIZE];

        for piece in &self.pieces {
            if piece.length == 0 {
                continue;
            }

            let piece_end =
                piece.start
                    .checked_add(
                        piece.length
                    )
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece source range overflow",
                        )
                    })?;

            if piece.original {
                if piece_end
                    > self.original_length
                {
                    return Err(
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece references data outside original file",
                        )
                    );
                }

                let mut source_position =
                    piece.start;

                while source_position
                    < piece_end
                {
                    let amount =
                        (piece_end
                            - source_position)
                            .min(buffer.len());

                    let read =
                        read_at(
                            &self.original,
                            &mut buffer[..amount],
                            source_position
                                as u64,
                        )?;

                    if read != amount {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected end of original file",
                            )
                        );
                    }

                    visit(&buffer[..read])?;

                    source_position += read;
                }
            } else {
                if piece_end > self.add.len() {
                    return Err(
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece references data outside edit store",
                        )
                    );
                }

                let mut source_position =
                    piece.start;

                while source_position
                    < piece_end
                {
                    let amount =
                        (piece_end
                            - source_position)
                            .min(buffer.len());

                    let read =
                        self.add.read_into(
                            source_position,
                            &mut buffer[..amount],
                        )?;

                    if read != amount {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected end of edit store",
                            )
                        );
                    }

                    visit(&buffer[..read])?;

                    source_position += read;
                }
            }
        }

        Ok(())
    }

    pub fn read_range_into(
        &self,
        position: usize,
        buffer: &mut [u8],
    ) -> io::Result<usize> {
        if buffer.is_empty()
            || position >= self.len()
        {
            return Ok(0);
        }

        let requested_end =
            position
                .saturating_add(buffer.len())
                .min(self.len());

        let mut current_position =
            0usize;

        let mut output_position =
            0usize;

        for piece in &self.pieces {
            let piece_start =
                current_position;

            let piece_end =
                current_position
                    .checked_add(piece.length)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece range overflow",
                        )
                    })?;

            if piece_end <= position {
                current_position =
                    piece_end;

                continue;
            }

            if piece_start >= requested_end {
                break;
            }

            let read_start =
                position.max(piece_start);

            let read_end =
                requested_end.min(piece_end);

            let read_length =
                read_end - read_start;

            let piece_offset =
                read_start - piece_start;

            let source_position =
                piece.start + piece_offset;

            if piece.original {
                let read =
                    read_at(
                        &self.original,
                        &mut buffer[
                            output_position
                                ..output_position
                                    + read_length
                        ],
                        source_position as u64,
                    )?;

                if read != read_length {
                    return Err(
                        io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "unexpected end of original file",
                        )
                    );
                }
            } else {
                let read =
                    self.add.read_into(
                        source_position,
                        &mut buffer[
                            output_position
                                ..output_position
                                    + read_length
                        ],
                    )?;

                if read != read_length {
                    return Err(
                        io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "unexpected end of edit store",
                        )
                    );
                }
            }

            output_position +=
                read_length;

            current_position =
                piece_end;
        }

        Ok(output_position)
    }

    pub fn read_range(
        &self,
        position: usize,
        length: usize,
    ) -> io::Result<Vec<u8>> {
        if length == 0 {
            return Ok(Vec::new());
        }

        let mut buffer =
            vec![0u8; length];

        let count =
            self.read_range_into(
                position,
                &mut buffer,
            )?;

        buffer.truncate(count);

        Ok(buffer)
    }

    pub fn byte_at(
        &self,
        position: usize,
    ) -> io::Result<Option<u8>> {
        let mut byte =
            [0u8; 1];

        if self.read_range_into(
            position,
            &mut byte,
        )? == 0
        {
            Ok(None)
        } else {
            Ok(Some(byte[0]))
        }
    }

    // ---------------------------------------------------------------------
    // UTF-8
    // ---------------------------------------------------------------------

    fn char_width_from_byte(
        byte: u8,
    ) -> io::Result<usize> {
        match byte {
            0x00..=0x7F => Ok(1),

            0xC2..=0xDF => Ok(2),

            0xE0..=0xEF => Ok(3),

            0xF0..=0xF4 => Ok(4),

            _ => Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid UTF-8 leading byte: 0x{byte:02X}"
                    ),
                )
            ),
        }
    }

    fn char_width_at(
        &self,
        position: usize,
    ) -> io::Result<usize> {
        if position >= self.len() {
            return Err(
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "position is at or beyond end of document",
                )
            );
        }

        let remaining =
            self.len() - position;

        let width =
            Self::char_width_from_byte(
                self.byte_at(position)?
                    .unwrap(),
            )?;

        if width > remaining {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "incomplete UTF-8 character at byte {position}"
                    ),
                )
            );
        }

        let bytes =
            self.read_range(
                position,
                width,
            )?;

        std::str::from_utf8(&bytes)
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid UTF-8 at byte {position}: {error}"
                    ),
                )
            })?;

        Ok(width)
    }

    fn validate_utf8(
        bytes: &[u8],
        absolute_position: usize,
    ) -> io::Result<()> {
        std::str::from_utf8(bytes)
            .map_err(|error| {
                let offset =
                    error.valid_up_to();

                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid UTF-8 at byte {}",
                        absolute_position
                            + offset
                    ),
                )
            })?;

        Ok(())
    }

    // ---------------------------------------------------------------------
    // Line cache
    // ---------------------------------------------------------------------


    fn discover_next_line(
    &mut self,
) -> io::Result<bool> {
    if self.all_lines_cached {
        return Ok(false);
    }

    let start =
        if let Some(previous) =
            self.line_cache.last().copied()
        {
            if previous.end < self.len() {
                previous.end + 1
            } else {
                self.all_lines_cached = true;
                return Ok(false);
            }
        } else {
            0
        };

    if start > self.len() {
        self.all_lines_cached = true;
        return Ok(false);
    }

    if start == self.len() {
        self.line_cache.push(
            LineInfo {
                start,
                end: start,
                length: 0,
            }
        );

        self.all_lines_cached = true;

        return Ok(true);
    }

    const SCAN_SIZE: usize = 64 * 1024;

    let line_start = start;
    let mut position = start;
    let mut char_count = 0usize;

    loop {
        let remaining =
            self.len() - position;

        if remaining == 0 {
            self.line_cache.push(
                LineInfo {
                    start: line_start,
                    end: position,
                    length: char_count,
                }
            );

            self.all_lines_cached = true;

            return Ok(true);
        }

        let amount =
            remaining.min(SCAN_SIZE);

        let bytes =
            self.read_range(
                position,
                amount,
            )?;

        if bytes.is_empty() {
            return Err(
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unexpected end of document",
                )
            );
        }

        let mut offset = 0usize;

        while offset < bytes.len() {
            let byte = bytes[offset];

            if byte == b'\n' {
                let end =
                    position + offset;
                let has_carriage_return =
                    if offset > 0 {
                        bytes[offset - 1] == b'\r'
                    } else {
                        position > line_start
                            && self.byte_at(
                                position - 1
                            )? == Some(b'\r')
                    };

                self.line_cache.push(
                    LineInfo {
                        start: line_start,
                        end,
                        length: char_count
                            .saturating_sub(
                                has_carriage_return as usize
                            ),
                    }
                );

                return Ok(true);
            }

            let width =
                Self::char_width_from_byte(
                    byte,
                )?;

            if offset + width > bytes.len() {
                let absolute =
                    position + offset;

                let width =
                    self.char_width_at(
                        absolute,
                    )?;

                offset += width;
            } else {
                Self::validate_utf8(
                    &bytes[
                        offset
                            ..offset + width
                    ],
                    position + offset,
                )?;

                offset += width;
            }

            char_count += 1;
        }

        position += bytes.len();
    }
}
    
    pub fn ensure_line_cached(
        &mut self,
        line: usize,
    ) -> io::Result<()> {
        while self.line_cache.len()
            <= line
            && !self.all_lines_cached
        {
            self.discover_next_line()?;
        }

        Ok(())
    }


    fn ensure_position_cached(
    &mut self,
    position: usize,
) -> io::Result<()> {
    if position > self.len() {
        return Err(
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "position is outside document",
            )
        );
    }

    while !self.all_lines_cached {
        if let Some(last) =
            self.line_cache.last().copied()
        {
            if position <= last.end {
                break;
            }
        }

        if !self.discover_next_line()? {
            break;
        }
    }

    Ok(())
}
    
    pub fn line_count(
        &mut self,
    ) -> io::Result<usize> {
        while !self.all_lines_cached {
            self.discover_next_line()?;
        }

        Ok(self.line_cache.len().max(1))
    }

    // ---------------------------------------------------------------------
    // Line access
    // ---------------------------------------------------------------------

    pub fn line_start(
        &mut self,
        line: usize,
    ) -> io::Result<usize> {
        self.ensure_line_cached(line)?;

        if line >= self.line_cache.len() {
            return Ok(self.len());
        }

        Ok(
            self.line_cache[line].start
        )
    }

    pub fn line_length(
        &mut self,
        line: usize,
    ) -> io::Result<usize> {
        self.ensure_line_cached(line)?;

        if line >= self.line_cache.len() {
            return Ok(0);
        }

        Ok(
            self.line_cache[line].length
        )
    }

    pub fn line_text(
        &mut self,
        line: usize,
    ) -> io::Result<String> {
        self.ensure_line_cached(line)?;

        if line >= self.line_cache.len() {
            return Ok(String::new());
        }

        let info =
            self.line_cache[line];

        if info.end <= info.start {
            return Ok(String::new());
        }

        let bytes =
            self.read_range(
                info.start,
                info.end - info.start,
            )?;

        let mut text = String::from_utf8(bytes)
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid UTF-8: {error}"
                    ),
                )
            })?;

        if text.ends_with('\r') {
            text.pop();
        }

        Ok(text)
    }

    pub fn line_text_from_start(
        &mut self,
        start: usize,
    ) -> io::Result<(String, usize)> {
        if start >= self.len() {
            return Ok((
                String::new(),
                self.len(),
            ));
        }

        self.ensure_position_cached(
            start
        )?;

        for info in &self.line_cache {
            if info.start == start {
                let length =
                    info.end - info.start;

                if length == 0 {
                    let next =
                        if info.end < self.len()
                            && self.byte_at(
                                info.end
                            )? == Some(b'\n')
                        {
                            info.end + 1
                        } else {
                            info.end
                        };

                    return Ok((
                        String::new(),
                        next,
                    ));
                }

                let bytes =
                    self.read_range(
                        info.start,
                        length,
                    )?;

                    let mut text =
                        String::from_utf8(
                            bytes
                        )
                    .map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "invalid UTF-8: {error}"
                            ),
                        )
                    })?;

                    if text.ends_with('\r') {
                        text.pop();
                    }

                let next =
                    if info.end < self.len() {
                        info.end + 1
                    } else {
                        info.end
                    };

                return Ok((
                    text,
                    next,
                ));
            }
        }

        const SCAN_SIZE: usize =
            64 * 1024;

        let mut position =
            start;

        loop {
            let remaining =
                self.len() - position;

            let amount =
                remaining.min(
                    SCAN_SIZE
                );

            let bytes =
                self.read_range(
                    position,
                    amount,
                )?;

            if bytes.is_empty() {
                break;
            }

            if let Some(offset) =
                bytes.iter().position(
                    |&byte| byte == b'\n'
                )
            {
                let end =
                    position + offset;

                let text_bytes =
                    self.read_range(
                        start,
                        end - start,
                    )?;

                let text =
                    String::from_utf8(
                        text_bytes,
                    )
                    .map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "invalid UTF-8: {error}"
                            ),
                        )
                    })?;

                return Ok((
                    text,
                    end + 1,
                ));
            }

            position +=
                bytes.len();
        }

        let text =
            String::from_utf8(
                self.read_range(
                    start,
                    self.len() - start,
                )?,
            )
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid UTF-8: {error}"
                    ),
                )
            })?;

        Ok((
            text,
            self.len(),
        ))
    }

    // ---------------------------------------------------------------------
    // Line navigation
    // ---------------------------------------------------------------------

    pub fn next_line_start_from(
        &mut self,
        start: usize,
    ) -> io::Result<Option<usize>> {
        self.ensure_position_cached(
            start
        )?;

        for info in &self.line_cache {
            if info.start == start {
                if info.end < self.len() {
                    return Ok(
                        Some(info.end + 1)
                    );
                }

                return Ok(None);
            }
        }

        let mut position =
            start;

        while position < self.len() {
            let bytes =
                self.read_range(
                    position,
                    (self.len() - position)
                        .min(64 * 1024),
                )?;

            if let Some(offset) =
                bytes.iter().position(
                    |&byte| byte == b'\n'
                )
            {
                return Ok(
                    Some(
                        position
                            + offset
                            + 1
                    )
                );
            }

            position +=
                bytes.len();
        }

        Ok(None)
    }

    pub fn previous_line_start_from(
        &mut self,
        start: usize,
    ) -> io::Result<usize> {
        self.ensure_position_cached(
            start
        )?;

        if start == 0 {
            return Ok(0);
        }

        let mut position =
            start;

        if position > 0 {
            position -= 1;
        }

        loop {
            let window_start =
                position.saturating_sub(
                    64 * 1024
                );

            let bytes =
                self.read_range(
                    window_start,
                    position
                        .saturating_sub(
                            window_start
                        ),
                )?;

            if !bytes.is_empty() {
                for i in
                    (0..bytes.len()).rev()
                {
                    if bytes[i] == b'\n' {
                        return Ok(
                            window_start
                                + i
                                + 1
                        );
                    }
                }
            }

            if position == 0 {
                return Ok(0);
            }

            position =
                position.saturating_sub(
                    64 * 1024
                );
        }
    }

    // ---------------------------------------------------------------------
    // Cursor
    // ---------------------------------------------------------------------

    fn ensure_boundary(
        &self,
        position: usize,
    ) -> io::Result<()> {
        if position > self.len() {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "position is outside document",
                )
            );
        }

        if position == 0
            || position == self.len()
        {
            return Ok(());
        }

        let byte =
            self.byte_at(position)?
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "invalid document position",
                    )
                })?;

        if (byte & 0xC0) == 0x80 {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "position is not a UTF-8 character boundary",
                )
            );
        }

        Ok(())
    }

    pub fn previous_char_boundary(
        &self,
        position: usize,
    ) -> io::Result<usize> {
        if position == 0 {
            return Ok(0);
        }

        if position > self.len() {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "cursor position is outside document",
                )
            );
        }

        let window_start =
            position.saturating_sub(4);

        let bytes =
            self.read_range(
                window_start,
                position - window_start,
            )?;

        let mut index =
            bytes.len()
                .saturating_sub(1);

        while index > 0
            && (bytes[index] & 0xC0)
                == 0x80
        {
            index -= 1;
        }

        Ok(
            window_start + index
        )
    }

    pub fn next_char_boundary(
        &self,
        position: usize,
    ) -> io::Result<usize> {
        if position >= self.len() {
            return Ok(self.len());
        }

        let width =
            self.char_width_at(
                position
            )?;

        Ok(
            position
                .checked_add(width)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "cursor position overflow",
                    )
                })?
        )
    }

    pub fn line_column_at(
        &mut self,
        position: usize,
    ) -> io::Result<(usize, usize)> {
        let position =
            position.min(self.len());

        self.ensure_boundary(
            position
        )?;

        self.ensure_position_cached(
            position
        )?;

        let mut line_index =
            0usize;

        for (
            index,
            info
        ) in self.line_cache
            .iter()
            .enumerate()
        {
            if position >= info.start
                && position <= info.end
            {
                line_index =
                    index;

                break;
            }
        }

        let info =
            self.line_cache[
                line_index
            ];

        let bytes =
            self.read_range(
                info.start,
                position - info.start,
            )?;

        let text =
            std::str::from_utf8(
                &bytes
            )
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    error,
                )
            })?;

        Ok((
            line_index,
            text.chars().count(),
        ))
    }

    pub fn cursor_line_column(
        &self,
    ) -> io::Result<(usize, usize)> {
        Ok((
            self.cursor.line,
            self.cursor.column,
        ))
    }

    pub fn move_cursor(
        &mut self,
        position: usize,
    ) -> io::Result<()> {
        let position =
            position.min(
                self.len()
            );

        self.ensure_boundary(
            position
        )?;

        let (line, column) =
            self.line_column_at(
                position
            )?;

        self.cursor.position =
            position;

        self.cursor.line =
            line;

        self.cursor.column =
            column;

        self.cursor.anchor =
            position;

        self.cursor.anchor_line =
            line;

        self.cursor.anchor_column =
            column;

        self.cursor.desired_column =
            None;

        Ok(())
    }

    pub fn selection_start(
        &self,
    ) -> usize {
        self.cursor.position
            .min(
                self.cursor.anchor
            )
    }

    pub fn selection_end(
        &self,
    ) -> usize {
        self.cursor.position
            .max(
                self.cursor.anchor
            )
    }

    pub fn has_selection(
        &self,
    ) -> bool {
        self.cursor.position
            != self.cursor.anchor
    }

    // ---------------------------------------------------------------------
    // Cursor movement
    // ---------------------------------------------------------------------

    /// Move the active end, retaining all anchor coordinates when selecting.
    fn navigate_to(&mut self, position: usize, selecting: bool) -> io::Result<()> {
        let anchor = (
            self.cursor.anchor,
            self.cursor.anchor_line,
            self.cursor.anchor_column,
        );
        self.move_cursor(position)?;
        if selecting {
            self.cursor.anchor = anchor.0;
            self.cursor.anchor_line = anchor.1;
            self.cursor.anchor_column = anchor.2;
        }
        Ok(())
    }

    pub fn select_all(&mut self) -> io::Result<()> {
        self.move_cursor(self.len())?;
        self.cursor.anchor = 0;
        self.cursor.anchor_line = 0;
        self.cursor.anchor_column = 0;
        Ok(())
    }

    // Whitespace, identifier characters, and punctuation form separate runs.
    // Read at most one UTF-8 character without copying the document.
    fn word_class_at(&self, position: usize) -> io::Result<u8> {
        let end = self.next_char_boundary(position)?;
        let bytes = self.read_range(position, end - position)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let character = text.chars().next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "expected a word character")
        })?;
        Ok(if character.is_whitespace() {
            0
        } else if character.is_alphanumeric() || character == '_' {
            1
        } else {
            2
        })
    }

    pub fn move_word(&mut self, forward: bool, selecting: bool) -> io::Result<()> {
        let mut position = self.cursor.position;
        if forward && position < self.len() {
            let class = self.word_class_at(position)?;
            while position < self.len() && self.word_class_at(position)? == class {
                position = self.next_char_boundary(position)?;
            }
            while position < self.len() && self.word_class_at(position)? == 0 {
                position = self.next_char_boundary(position)?;
            }
        } else if !forward && position > 0 {
            while position > 0 {
                let previous = self.previous_char_boundary(position)?;
                if self.word_class_at(previous)? != 0 {
                    break;
                }
                position = previous;
            }
            if position > 0 {
                let previous = self.previous_char_boundary(position)?;
                let class = self.word_class_at(previous)?;
                position = previous;
                while position > 0 {
                    let previous = self.previous_char_boundary(position)?;
                    if self.word_class_at(previous)? != class {
                        break;
                    }
                    position = previous;
                }
            }
        }
        self.navigate_to(position, selecting)
    }

    pub fn move_page(
        &mut self,
        forward: bool,
        lines: usize,
        selecting: bool,
    ) -> io::Result<()> {
        let desired = self.cursor.desired_column.unwrap_or(self.cursor.column);
        let target = if forward {
            self.cursor.line.saturating_add(lines.max(1))
        } else {
            self.cursor.line.saturating_sub(lines.max(1))
        };
        self.ensure_line_cached(target)?;
        let target = target.min(self.line_cache.len().saturating_sub(1));
        let info = self.line_cache[target];
        let position = self.position_at_column(info.start, desired.min(info.length))?;
        self.navigate_to(position, selecting)?;
        self.cursor.desired_column = Some(desired);
        Ok(())
    }

    pub fn select_home(&mut self) -> io::Result<()> {
        let position = self.current_line_start()?;
        self.navigate_to(position, true)
    }

    pub fn select_end(&mut self) -> io::Result<()> {
        let position = self.current_line_end()?;
        self.navigate_to(position, true)
    }


    fn position_at_column(
    &self,
    start: usize,
    column: usize,
) -> io::Result<usize> {
    if column == 0 {
        return Ok(start);
    }

    if start >= self.len() {
        return Ok(start);
    }

    // Read only the target line, not the rest of the document.
    let mut position = start;
    let mut current_column = 0usize;

    const SCAN_SIZE: usize = 64 * 1024;

    while position < self.len()
        && current_column < column
    {
        let remaining =
            self.len() - position;

        let amount =
            remaining.min(SCAN_SIZE);

        let bytes =
            self.read_range(
                position,
                amount,
            )?;

        if bytes.is_empty() {
            break;
        }

        let mut offset = 0usize;

        while offset < bytes.len()
            && current_column < column
        {
            let byte =
                bytes[offset];

            if byte == b'\n' {
                return Ok(position + offset);
            }

            let width =
                Self::char_width_from_byte(
                    byte,
                )?;

            if offset + width > bytes.len() {
                // The UTF-8 character crosses the read boundary.
                let absolute =
                    position + offset;

                let width =
                    self.char_width_at(
                        absolute,
                    )?;

                position +=
                    offset + width;

                current_column += 1;

                break;
            }

            Self::validate_utf8(
                &bytes[
                    offset
                        ..offset + width
                ],
                position + offset,
            )?;

            offset += width;
            current_column += 1;
        }

        position += offset;

        // We reached the end of this chunk.
        if offset == bytes.len() {
            continue;
        }

        if current_column >= column {
            break;
        }
    }

    Ok(position)
}
    
    fn current_line_start(
        &mut self,
    ) -> io::Result<usize> {
        self.ensure_position_cached(
            self.cursor.position
        )?;

        for info in
            self.line_cache.iter()
        {
            if self.cursor.position
                >= info.start
                && self.cursor.position
                    <= info.end
            {
                return Ok(
                    info.start
                );
            }
        }

        Ok(0)
    }

    fn previous_line_start(
        &mut self,
    ) -> io::Result<usize> {
        let current =
            self.current_line_start()?;

        if current == 0 {
            return Ok(0);
        }

        self.previous_line_start_from(
            current
        )
    }

    fn line_length_from_start(
    &self,
    start: usize,
) -> io::Result<usize> {
    let mut position = start;

    const SCAN_SIZE: usize = 64 * 1024;

    let mut character_count = 0usize;

    while position < self.len() {
        let remaining =
            self.len() - position;

        let amount =
            remaining.min(SCAN_SIZE);

        let bytes =
            self.read_range(
                position,
                amount,
            )?;

        if bytes.is_empty() {
            break;
        }

        let mut offset = 0usize;

        while offset < bytes.len() {
            let byte =
                bytes[offset];

            if byte == b'\n' {
                let has_carriage_return =
                    if offset > 0 {
                        bytes[offset - 1] == b'\r'
                    } else {
                        position > start
                            && self.byte_at(
                                position - 1
                            )? == Some(b'\r')
                    };

                return Ok(
                    character_count
                        .saturating_sub(
                            has_carriage_return as usize
                        )
                );
            }

            let width =
                Self::char_width_from_byte(
                    byte,
                )?;

            if offset + width > bytes.len() {
                let absolute =
                    position + offset;

                let width =
                    self.char_width_at(
                        absolute,
                    )?;

                offset += width;
                character_count += 1;
                continue;
            }

            Self::validate_utf8(
                &bytes[
                    offset
                        ..offset + width
                ],
                position + offset,
            )?;

            offset += width;
            character_count += 1;
        }

        position += bytes.len();
    }

    Ok(character_count)
}
    
    pub fn cursor_left(
        &mut self,
    ) -> io::Result<()> {
        if self.cursor.position == 0 {
            return Ok(());
        }

        let position =
            self.previous_char_boundary(
                self.cursor.position
            )?;

        let byte =
            self.byte_at(position)?;

        self.cursor.position =
            position;

        self.cursor.desired_column =
            None;

        if byte == Some(b'\n') {
            self.cursor.line =
                self.cursor.line
                    .saturating_sub(1);

            let start =
                self.current_line_start()?;

            self.cursor.column =
                self.line_length_from_start(
                    start
                )?;
        } else {
            self.cursor.column =
                self.cursor.column
                    .saturating_sub(1);
        }

        self.cursor.anchor =
            position;

        self.cursor.anchor_line =
            self.cursor.line;

        self.cursor.anchor_column =
            self.cursor.column;

        Ok(())
    }

    pub fn cursor_right(
        &mut self,
    ) -> io::Result<()> {
        if self.cursor.position
            >= self.len()
        {
            return Ok(());
        }

        let byte =
            self.byte_at(
                self.cursor.position
            )?;

        let position =
            self.next_char_boundary(
                self.cursor.position
            )?;

        self.cursor.position =
            position;

        self.cursor.desired_column =
            None;

        if byte == Some(b'\n') {
            self.cursor.line += 1;
            self.cursor.column = 0;
        } else {
            self.cursor.column += 1;
        }

        self.cursor.anchor =
            position;

        self.cursor.anchor_line =
            self.cursor.line;

        self.cursor.anchor_column =
            self.cursor.column;

        Ok(())
    }

    pub fn cursor_up(
        &mut self,
    ) -> io::Result<()> {
        if self.has_selection() {
            return self.move_cursor(
                self.selection_start()
            );
        }

        if self.cursor.line == 0 {
            return Ok(());
        }

        let desired =
            *self.cursor
                .desired_column
                .get_or_insert(
                    self.cursor.column
                );

        let start =
            self.previous_line_start()?;

        let length =
            self.line_length_from_start(
                start
            )?;

        let column =
            desired.min(length);

        let position =
            self.position_at_column(
                start,
                column,
            )?;

        self.cursor.position =
            position;

        self.cursor.line -= 1;

        self.cursor.column =
            column;

        self.cursor.anchor =
            position;

        self.cursor.anchor_line =
            self.cursor.line;

        self.cursor.anchor_column =
            column;

        Ok(())
    }

    pub fn cursor_down(
        &mut self,
    ) -> io::Result<()> {
        if self.has_selection() {
            return self.move_cursor(
                self.selection_end()
            );
        }

        self.ensure_line_cached(
            self.cursor.line + 1
        )?;

        if self.cursor.line + 1
            >= self.line_cache.len()
        {
            return Ok(());
        }

        let desired =
            *self.cursor
                .desired_column
                .get_or_insert(
                    self.cursor.column
                );

        let info =
            self.line_cache[
                self.cursor.line + 1
            ];

        let column =
            desired.min(info.length);

        let position =
            self.position_at_column(
                info.start,
                column,
            )?;

        self.cursor.position =
            position;

        self.cursor.line += 1;

        self.cursor.column =
            column;

        self.cursor.anchor =
            position;

        self.cursor.anchor_line =
            self.cursor.line;

        self.cursor.anchor_column =
            column;

        Ok(())
    }

    pub fn select_left(
        &mut self,
    ) -> io::Result<()> {
        if self.cursor.position == 0 {
            return Ok(());
        }

        let position =
            self.previous_char_boundary(
                self.cursor.position
            )?;

        let byte =
            self.byte_at(position)?;

        self.cursor.position =
            position;

        if byte == Some(b'\n') {
            self.cursor.line =
                self.cursor.line
                    .saturating_sub(1);

            let start =
                self.current_line_start()?;

            self.cursor.column =
                self.line_length_from_start(
                    start
                )?;
        } else {
            self.cursor.column =
                self.cursor.column
                    .saturating_sub(1);
        }

        self.cursor.desired_column =
            None;

        Ok(())
    }

    pub fn select_right(
        &mut self,
    ) -> io::Result<()> {
        if self.cursor.position
            >= self.len()
        {
            return Ok(());
        }

        let byte =
            self.byte_at(
                self.cursor.position
            )?;

        let position =
            self.next_char_boundary(
                self.cursor.position
            )?;

        self.cursor.position =
            position;

        if byte == Some(b'\n') {
            self.cursor.line += 1;
            self.cursor.column = 0;
        } else {
            self.cursor.column += 1;
        }

        self.cursor.desired_column =
            None;

        Ok(())
    }

    pub fn select_up(
        &mut self,
    ) -> io::Result<()> {
        let anchor =
            self.cursor.anchor;

        if self.cursor.line == 0 {
            return Ok(());
        }

        let start =
            self.previous_line_start()?;

        let length =
            self.line_length_from_start(
                start
            )?;

        let column =
            self.cursor.column
                .min(length);

        let position =
            self.position_at_column(
                start,
                column,
            )?;

        self.cursor.position =
            position;

        self.cursor.line -= 1;

        self.cursor.column =
            column;

        self.cursor.anchor =
            anchor;

        self.cursor.desired_column =
            None;

        Ok(())
    }

    pub fn select_down(
        &mut self,
    ) -> io::Result<()> {
        let anchor =
            self.cursor.anchor;

        self.ensure_line_cached(
            self.cursor.line + 1
        )?;

        if self.cursor.line + 1
            >= self.line_cache.len()
        {
            return Ok(());
        }

        let info =
            self.line_cache[
                self.cursor.line + 1
            ];

        let column =
            self.cursor.column
                .min(info.length);

        let position =
            self.position_at_column(
                info.start,
                column,
            )?;

        self.cursor.position =
            position;

        self.cursor.line += 1;

        self.cursor.column =
            column;

        self.cursor.anchor =
            anchor;

        self.cursor.desired_column =
            None;

        Ok(())
    }

    pub fn cursor_home(
        &mut self,
    ) -> io::Result<()> {
        let position =
            self.current_line_start()?;

        self.cursor.position =
            position;

        self.cursor.column =
            0;

        self.cursor.anchor =
            position;

        self.cursor.anchor_line =
            self.cursor.line;

        self.cursor.anchor_column =
            0;

        self.cursor.desired_column =
            None;

        Ok(())
    }

    pub fn current_line_end(
        &mut self,
    ) -> io::Result<usize> {
        self.ensure_position_cached(
            self.cursor.position
        )?;

        for info in
            self.line_cache.iter()
        {
            if self.cursor.position
                >= info.start
                && self.cursor.position
                    <= info.end
            {
                return Ok(info.end);
            }
        }

        Ok(self.len())
    }

    pub fn cursor_end(
    &mut self,
) -> io::Result<()> {
    let start =
        self.current_line_start()?;

    let position =
        self.current_line_end()?;

    let length =
        self.line_length_from_start(
            start,
        )?;

    self.cursor.position =
        position;

    self.cursor.column =
        length;

    self.cursor.anchor =
        position;

    self.cursor.anchor_line =
        self.cursor.line;

    self.cursor.anchor_column =
        length;

    self.cursor.desired_column =
        None;

    Ok(())
}

    // ---------------------------------------------------------------------
    // Editing
    // ---------------------------------------------------------------------

    fn push_merged_piece(
        pieces: &mut Vec<Piece>,
        piece: Piece,
    ) -> io::Result<()> {
        if piece.length == 0 {
            return Ok(());
        }

        if let Some(previous) =
            pieces.last_mut()
        {
            let previous_end =
                previous.start
                    .checked_add(
                        previous.length
                    )
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece source range overflow",
                        )
                    })?;

            if previous.original
                == piece.original
                && previous_end
                    == piece.start
            {
                previous.length =
                    previous.length
                        .checked_add(
                            piece.length
                        )
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "merged piece length overflow",
                            )
                        })?;

                return Ok(());
            }
        }

        pieces.push(piece);

        Ok(())
    }

    fn consume_pieces_until(
        &self,
        target: usize,
        copy: bool,
        output: &mut Vec<Piece>,
        piece_index: &mut usize,
        piece_offset: &mut usize,
        logical_position: &mut usize,
    ) -> io::Result<()> {
        if target < *logical_position
            || target > self.len()
        {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "replacement ranges are not in ascending order",
                )
            );
        }

        while *logical_position < target {
            let piece =
                self.pieces
                    .get(*piece_index)
                    .copied()
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "pieces end before the logical document",
                        )
                    })?;

            if *piece_offset
                > piece.length
            {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "piece cursor is outside its piece",
                    )
                );
            }

            let available =
                piece.length
                    - *piece_offset;

            if available == 0 {
                *piece_index += 1;
                *piece_offset = 0;
                continue;
            }

            let amount =
                (target - *logical_position)
                    .min(available);

            let source_start =
                piece.start
                    .checked_add(
                        *piece_offset
                    )
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece source range overflow",
                        )
                    })?;

            let source_end =
                source_start
                    .checked_add(amount)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece source range overflow",
                        )
                    })?;

            let source_length =
                if piece.original {
                    self.original_length
                } else {
                    self.add.len()
                };

            if source_end > source_length {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "piece references data outside its source buffer",
                    )
                );
            }

            if copy {
                Self::push_merged_piece(
                    output,
                    Piece {
                        start: source_start,
                        length: amount,
                        original:
                            piece.original,
                    },
                )?;
            }

            *piece_offset += amount;
            *logical_position =
                logical_position
                    .checked_add(amount)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "logical document position overflow",
                        )
                    })?;

            if *piece_offset
                == piece.length
            {
                *piece_index += 1;
                *piece_offset = 0;
            }
        }

        Ok(())
    }

    fn ensure_piece_cursor_boundary(
        &self,
        logical_position: usize,
        piece_index: &mut usize,
        piece_offset: &mut usize,
    ) -> io::Result<()> {
        if logical_position == 0
            || logical_position == self.len()
        {
            return Ok(());
        }

        while let Some(piece) =
            self.pieces.get(*piece_index)
        {
            if *piece_offset
                > piece.length
            {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "piece cursor is outside its piece",
                    )
                );
            }

            if *piece_offset
                == piece.length
            {
                *piece_index += 1;
                *piece_offset = 0;
                continue;
            }

            let source_position =
                piece.start
                    .checked_add(
                        *piece_offset
                    )
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece source range overflow",
                        )
                    })?;

            let byte =
                if piece.original {
                    if source_position
                        >= self.original_length
                    {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "piece references data outside original file",
                            )
                        );
                    }

                    let mut byte = [0u8; 1];

                    if read_at(
                        &self.original,
                        &mut byte,
                        source_position as u64,
                    )? != 1
                    {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected end of original file",
                            )
                        );
                    }

                    byte[0]
                } else {
                    let mut byte = [0u8; 1];

                    if self.add.read_into(
                        source_position,
                        &mut byte,
                    )? != 1
                    {
                        return Err(
                            io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "unexpected end of edit store",
                            )
                        );
                    }

                    byte[0]
                };

            if (byte & 0xC0) == 0x80 {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "position is not a UTF-8 character boundary",
                    )
                );
            }

            return Ok(());
        }

        Err(
            io::Error::new(
                io::ErrorKind::InvalidData,
                "pieces end before the logical document",
            )
        )
    }

    /// Replaces ascending, non-overlapping runs from the current document.
    ///
    /// Each run is `(start, end, repetitions)`: the original byte range is
    /// removed once and `replacement` is inserted `repetitions` times. Runs
    /// use original UTF-8 byte offsets and must have a non-zero repetition
    /// count. The method validates and builds in one forward traversal of the
    /// existing pieces, without reconstructing the document or rescanning it
    /// for character boundaries.
    ///
    /// Repeated output shares one add-buffer block capped near 64 KiB, or one
    /// complete replacement when that text is larger than the cap. Large
    /// dense runs therefore use bounded add-buffer space and a small number
    /// of pieces. An empty iterator is a no-op and returns `None`.
    pub(crate) fn replace_runs<I>(
        &mut self,
        runs: I,
        replacement: &str,
    ) -> io::Result<Option<PieceTableSnapshot>>
    where
        I: Iterator<
            Item = (usize, usize, usize),
        >,
    {
        let document_length =
            self.len();

        let replacement_length =
            replacement.len();

        let block_repetition_limit =
            if replacement_length == 0 {
                1
            } else {
                (PIECE_TABLE_CHUNK_SIZE
                    / replacement_length)
                    .max(1)
            };

        let block_length_limit =
            replacement_length
                .checked_mul(
                    block_repetition_limit
                )
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "replacement block length overflow",
                    )
                })?;

        let add_start =
            self.add.len();

        let mut deleted_length = 0usize;
        let mut inserted_length = 0usize;
        let mut previous_end = 0usize;
        let mut first_position = None;
        let mut required_block_length = 0usize;

        let mut pieces =
            Vec::with_capacity(
                self.pieces.len()
            );

        let mut piece_index = 0usize;
        let mut piece_offset = 0usize;
        let mut logical_position = 0usize;

        for (start, end, repetitions)
            in runs
        {
            if start > end {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "replacement range start is after its end",
                    )
                );
            }

            if end > document_length {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "replacement range is outside document",
                    )
                );
            }

            if first_position.is_some()
                && start < previous_end
            {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "replacement ranges overlap or are not ascending",
                    )
                );
            }

            if repetitions == 0 {
                return Err(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "replacement run repetition count is zero",
                    )
                );
            }

            deleted_length =
                deleted_length
                    .checked_add(end - start)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "total replacement range length overflow",
                        )
                    })?;

            let run_inserted_length =
                replacement_length
                    .checked_mul(repetitions)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "replacement result length overflow",
                        )
                    })?;

            inserted_length =
                inserted_length
                    .checked_add(
                        run_inserted_length
                    )
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "replacement result length overflow",
                        )
                    })?;

            first_position
                .get_or_insert(start);

            document_length
                .checked_sub(deleted_length)
                .and_then(|length| {
                    length.checked_add(
                        inserted_length
                    )
                })
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "replacement result length overflow",
                    )
                })?;

            self.consume_pieces_until(
                start,
                true,
                &mut pieces,
                &mut piece_index,
                &mut piece_offset,
                &mut logical_position,
            )?;

            self.ensure_piece_cursor_boundary(
                start,
                &mut piece_index,
                &mut piece_offset,
            )?;

            if replacement_length > 0 {
                let full_blocks =
                    repetitions
                        / block_repetition_limit;

                let remainder =
                    repetitions
                        % block_repetition_limit;

                let insertion_pieces =
                    full_blocks
                        .checked_add(
                            usize::from(
                                remainder > 0
                            )
                        )
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "replacement piece count overflow",
                            )
                        })?;

                pieces.try_reserve(
                    insertion_pieces
                )
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        "could not allocate replacement pieces",
                    )
                })?;

                for _ in 0..full_blocks {
                    Self::push_merged_piece(
                        &mut pieces,
                        Piece {
                            start: add_start,
                            length:
                                block_length_limit,
                            original: false,
                        },
                    )?;
                }

                if remainder > 0 {
                    let remainder_length =
                        replacement_length
                            .checked_mul(
                                remainder
                            )
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "replacement block length overflow",
                                )
                            })?;

                    Self::push_merged_piece(
                        &mut pieces,
                        Piece {
                            start: add_start,
                            length:
                                remainder_length,
                            original: false,
                        },
                    )?;
                }

                let run_block_length =
                    if full_blocks > 0 {
                        block_length_limit
                    } else {
                        replacement_length
                            .checked_mul(
                                remainder
                            )
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "replacement block length overflow",
                                )
                            })?
                    };

                required_block_length =
                    required_block_length
                        .max(run_block_length);
            }

            self.consume_pieces_until(
                end,
                false,
                &mut pieces,
                &mut piece_index,
                &mut piece_offset,
                &mut logical_position,
            )?;

            self.ensure_piece_cursor_boundary(
                end,
                &mut piece_index,
                &mut piece_offset,
            )?;

            previous_end = end;
        }

        let Some(first_position) =
            first_position
        else {
            return Ok(None);
        };

        self.consume_pieces_until(
            document_length,
            true,
            &mut pieces,
            &mut piece_index,
            &mut piece_offset,
            &mut logical_position,
        )?;

        while piece_index
            < self.pieces.len()
            && self.pieces[piece_index]
                .length
                == 0
        {
            piece_index += 1;
        }

        if piece_index
            != self.pieces.len()
        {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "pieces extend beyond the logical document",
                )
            );
        }

        let final_length =
            document_length
                .checked_sub(deleted_length)
                .and_then(|length| {
                    length.checked_add(
                        inserted_length
                    )
                })
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "replacement result length overflow",
                    )
                })?;

        add_start
            .checked_add(
                required_block_length
            )
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "add buffer length overflow",
                )
            })?;

        if required_block_length > 0 {
            let repetitions =
                required_block_length
                    / replacement_length;

            if repetitions == 1 {
                self.add.append(
                    replacement.as_bytes()
                )?;
            } else {
                let mut block =
                    Vec::with_capacity(
                        required_block_length
                    );

                for _ in 0..repetitions {
                    block.extend_from_slice(
                        replacement.as_bytes()
                    );
                }

                self.add.append(&block)?;
            }
        }

        let previous_pieces =
            std::mem::replace(
                &mut self.pieces,
                pieces,
            );

        let previous_length =
            std::mem::replace(
                &mut self.length,
                final_length,
            );

        self.invalidate_line_cache_from_position(
            first_position
        );

        Ok(Some(PieceTableSnapshot {
            pieces: previous_pieces,
            length: previous_length,
        }))
    }

    /// Replaces ascending, non-overlapping byte ranges once each.
    ///
    /// This compatibility wrapper retains the simple range API while using
    /// the single-pass run implementation.
    pub(crate) fn replace_ranges<I>(
        &mut self,
        ranges: I,
        replacement: &str,
    ) -> io::Result<Option<PieceTableSnapshot>>
    where
        I: Iterator<Item = (usize, usize)>,
    {
        self.replace_runs(
            ranges.map(|(start, end)| {
                (start, end, 1)
            }),
            replacement,
        )
    }

    /// Swaps the current logical state with a snapshot returned by
    /// [`Self::replace_ranges`]. Calling this repeatedly toggles undo/redo.
    pub(crate) fn swap_snapshot(
        &mut self,
        snapshot: &mut PieceTableSnapshot,
    ) {
        std::mem::swap(
            &mut self.pieces,
            &mut snapshot.pieces,
        );

        std::mem::swap(
            &mut self.length,
            &mut snapshot.length,
        );

        self.invalidate_line_cache_from_position(0);
    }

    /// Captures a logical range as references to the original file or edit
    /// store. Undo can retain these small records without retaining the text.
    pub(crate) fn capture_range(
        &self,
        position: usize,
        length: usize,
    ) -> io::Result<Vec<Piece>> {
        let end =
            position.checked_add(length)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "capture range overflow",
                    )
                })?;

        if end > self.len() {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "capture range is outside document",
                )
            );
        }

        self.ensure_boundary(position)?;
        self.ensure_boundary(end)?;

        if length == 0 {
            return Ok(Vec::new());
        }

        let mut captured = Vec::new();
        let mut logical_position = 0usize;

        for piece in &self.pieces {
            let piece_end =
                logical_position
                    .checked_add(piece.length)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "piece range overflow",
                        )
                    })?;

            if piece_end <= position {
                logical_position = piece_end;
                continue;
            }

            if logical_position >= end {
                break;
            }

            let overlap_start =
                position.max(logical_position);

            let overlap_end =
                end.min(piece_end);

            Self::push_merged_piece(
                &mut captured,
                Piece {
                    start:
                        piece.start
                            + overlap_start
                            - logical_position,
                    length:
                        overlap_end
                            - overlap_start,
                    original: piece.original,
                },
            )?;

            logical_position = piece_end;
        }

        let captured_length =
            captured.iter()
                .try_fold(
                    0usize,
                    |total, piece| {
                        total.checked_add(
                            piece.length
                        )
                    },
                )
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "captured range length overflow",
                    )
                })?;

        if captured_length != length {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "pieces end before capture range",
                )
            );
        }

        Ok(captured)
    }

    /// Compares a document range without allocating a range-sized buffer.
    pub(crate) fn range_equals(
        &self,
        position: usize,
        length: usize,
        expected: &[u8],
    ) -> io::Result<bool> {
        if length != expected.len() {
            return Ok(false);
        }

        let end =
            position.checked_add(length)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "comparison range overflow",
                    )
                })?;

        if end > self.len() {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "comparison range is outside document",
                )
            );
        }

        let mut buffer = [0u8; 4096];
        let mut compared = 0usize;

        while compared < length {
            let amount =
                (length - compared)
                    .min(buffer.len());

            let read =
                self.read_range_into(
                    position + compared,
                    &mut buffer[..amount],
                )?;

            if read != amount {
                return Err(
                    io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "unexpected end of comparison range",
                    )
                );
            }

            if buffer[..read]
                != expected[
                    compared..compared + read
                ]
            {
                return Ok(false);
            }

            compared += read;
        }

        Ok(true)
    }

    fn validate_source_piece(
        &self,
        piece: Piece,
    ) -> io::Result<()> {
        let source_end =
            piece.start
                .checked_add(piece.length)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "stored piece range overflow",
                    )
                })?;

        let source_length =
            if piece.original {
                self.original_length
            } else {
                self.add.len()
            };

        if source_end > source_length {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "stored piece is outside its source",
                )
            );
        }

        Ok(())
    }

    /// Inserts existing source references without copying their text.
    pub(crate) fn insert_pieces(
        &mut self,
        position: usize,
        pieces: &[Piece],
    ) -> io::Result<()> {
        if position > self.len() {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "insert position is outside document",
                )
            );
        }

        self.ensure_boundary(position)?;

        let mut insertion_length = 0usize;

        for piece in pieces {
            self.validate_source_piece(*piece)?;

            insertion_length =
                insertion_length
                    .checked_add(piece.length)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "inserted piece length overflow",
                        )
                    })?;
        }

        if insertion_length == 0 {
            return Ok(());
        }

        self.length
            .checked_add(insertion_length)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "document length overflow",
                )
            })?;

        if self.pieces.is_empty() {
            for piece in pieces {
                Self::push_merged_piece(
                    &mut self.pieces,
                    *piece,
                )?;
            }
        } else {
            let mut current = 0usize;

            for i in 0..self.pieces.len() {
                let piece = self.pieces[i];
                let end = current + piece.length;

                if position <= end {
                    let offset = position - current;

                    let mut replacement =
                        Vec::with_capacity(
                            pieces.len() + 2
                        );

                    if offset > 0 {
                        Self::push_merged_piece(
                            &mut replacement,
                            Piece {
                                start: piece.start,
                                length: offset,
                                original: piece.original,
                            },
                        )?;
                    }

                    for inserted in pieces {
                        Self::push_merged_piece(
                            &mut replacement,
                            *inserted,
                        )?;
                    }

                    if offset < piece.length {
                        Self::push_merged_piece(
                            &mut replacement,
                            Piece {
                                start:
                                    piece.start + offset,
                                length:
                                    piece.length - offset,
                                original: piece.original,
                            },
                        )?;
                    }

                    self.pieces.splice(
                        i..=i,
                        replacement,
                    );

                    break;
                }

                current = end;
            }
        }

        self.length += insertion_length;

        self.invalidate_line_cache_from_position(
            position
        );

        Ok(())
    }

    /// Appends valid UTF-8 text without making it part of the document yet.
    pub(crate) fn store_text(
        &mut self,
        text: &str,
    ) -> io::Result<Option<Piece>> {
        if text.is_empty() {
            return Ok(None);
        }

        let bytes = text.as_bytes();

        std::str::from_utf8(bytes)
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "inserted text is not valid UTF-8: {error}"
                    ),
                )
            })?;

        let piece = Piece {
            start: self.add.append(bytes)?,
            length: bytes.len(),
            original: false,
        };

        Ok(Some(piece))
    }

    pub fn insert(
        &mut self,
        position: usize,
        text: &str,
    ) -> io::Result<()> {
        if position > self.len() {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "insert position is outside document",
                )
            );
        }

        self.ensure_boundary(position)?;

        if let Some(piece) =
            self.store_text(text)?
        {
            self.insert_pieces(
                position,
                std::slice::from_ref(
                    &piece
                ),
            )?;
        }

        Ok(())
    }

    pub fn delete(
        &mut self,
        position: usize,
        length: usize,
    ) -> io::Result<()> {
        if length == 0 {
            return Ok(());
        }

        let end =
            position
                .checked_add(length)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "delete range overflow",
                    )
                })?;

        if position > self.len()
            || end > self.len()
        {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "delete range outside document",
                )
            );
        }

        self.ensure_boundary(
            position
        )?;

        self.ensure_boundary(
            end
        )?;

        let mut current =
            0usize;

        let mut pieces =
            Vec::with_capacity(
                self.pieces.len()
            );

        for piece in
            &self.pieces
        {
            let piece_start =
                current;

            let piece_end =
                current + piece.length;

            if piece_end <= position
                || piece_start >= end
            {
                pieces.push(*piece);
            } else {
                if piece_start < position {
                    pieces.push(
                        Piece {
                            start:
                                piece.start,
                            length:
                                position
                                    - piece_start,
                            original:
                                piece.original,
                        }
                    );
                }

                if piece_end > end {
                    let skip =
                        end - piece_start;

                    pieces.push(
                        Piece {
                            start:
                                piece.start
                                    + skip,
                            length:
                                piece.length
                                    - skip,
                            original:
                                piece.original,
                        }
                    );
                }
            }

            current =
                piece_end;
        }

        self.pieces =
            pieces;

        self.length -=
            length;

        self.invalidate_line_cache_from_position(
            position
        );

        Ok(())
    }

 
 fn invalidate_line_cache_from_position(
    &mut self,
    _position: usize,
) {
    self.line_cache.clear();

    if self.len() == 0 {
        self.line_cache.push(
            LineInfo {
                start: 0,
                end: 0,
                length: 0,
            }
        );

        self.all_lines_cached = true;
    } else {
        self.all_lines_cached = false;
    }
}
 
    
    // ---------------------------------------------------------------------
    // Text
    // ---------------------------------------------------------------------

    pub fn text(
        &self,
    ) -> io::Result<String> {
        let bytes =
            self.read_range(
                0,
                self.len(),
            )?;

        String::from_utf8(bytes)
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "document contains invalid UTF-8: {error}"
                    ),
                )
            })
    }

    // ---------------------------------------------------------------------
    // Debug
    // ---------------------------------------------------------------------

    #[allow(dead_code)]
    pub fn debug(
        &self,
    ) {
        println!(
            "Original file: {} bytes",
            self.original_length
        );

        println!(
            "Add buffer: {} bytes",
            self.add.len()
        );

        println!(
            "Pieces: {}",
            self.pieces.len()
        );

        println!(
            "Line cache: {} lines, complete={}",
            self.line_cache.len(),
            self.all_lines_cached
        );

        println!(
            "Cursor: position={} line={} column={}",
            self.cursor.position,
            self.cursor.line,
            self.cursor.column
        );
    }
}

// ==========================================================================
// Timing test
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{
        AtomicUsize,
        Ordering,
    };
    use std::time::Instant;

    fn table_with_text(
        text: &str,
    ) -> PieceTable {
        let mut table =
            PieceTable::empty()
                .unwrap();

        table.insert(0, text)
            .unwrap();

        table
    }

    #[test]
    fn crlf_line_text_excludes_carriage_return() {
        let mut table = table_with_text(
            "one\r\ntwo\r\n",
        );

        assert_eq!(
            table.line_text(0).unwrap(),
            "one",
        );
        assert_eq!(
            table.line_length(0).unwrap(),
            3,
        );
        assert_eq!(
            table.line_text(1).unwrap(),
            "two",
        );
    }

    #[test]
    fn inserted_text_is_file_backed_and_store_is_removed_on_drop() {
        let path;

        {
            let mut table =
                PieceTable::empty()
                    .unwrap();

            path = table.add.path.clone();

            assert!(path.exists());

            table.insert(0, "édit")
                .unwrap();

            assert_eq!(
                table.add.len(),
                "édit".len(),
            );

            assert_eq!(
                std::fs::metadata(&path)
                    .unwrap()
                    .len(),
                "édit".len() as u64,
            );

            assert_eq!(
                table.text().unwrap(),
                "édit",
            );
        }

        assert!(!path.exists());
    }

    #[test]
    fn captured_pieces_restore_text_without_appending_again() {
        let mut table =
            table_with_text("aéz");

        let stored_length =
            table.add.len();

        let captured =
            table.capture_range(
                1,
                "é".len(),
            )
            .unwrap();

        table.delete(
            1,
            "é".len(),
        )
        .unwrap();

        table.insert_pieces(
            1,
            &captured,
        )
        .unwrap();

        assert_eq!(
            table.text().unwrap(),
            "aéz",
        );

        assert_eq!(
            table.add.len(),
            stored_length,
        );
    }

    fn piece_layout(
        table: &PieceTable,
    ) -> Vec<(usize, usize, bool)> {
        table.pieces
            .iter()
            .map(|piece| {
                (
                    piece.start,
                    piece.length,
                    piece.original,
                )
            })
            .collect()
    }

    fn original_table(
        text: &str,
    ) -> (PieceTable, PathBuf) {
        static NEXT_PATH:
            AtomicUsize =
                AtomicUsize::new(0);

        let mut path =
            std::env::temp_dir();

        path.push(format!(
            "potyi_replace_ranges_{}_{}.txt",
            std::process::id(),
            NEXT_PATH.fetch_add(
                1,
                Ordering::Relaxed,
            ),
        ));

        std::fs::write(
            &path,
            text.as_bytes(),
        )
        .unwrap();

        let table =
            PieceTable::open(
                &path.to_string_lossy()
            )
            .unwrap();

        (table, path)
    }

    #[test]
    fn replace_ranges_reuses_one_add_copy_and_snapshot_swaps() {
        let mut table =
            table_with_text(
                "one two one"
            );

        table.line_count()
            .unwrap();

        let add_length =
            table.add.len();

        let mut snapshot =
            table.replace_ranges(
                vec![
                    (0, 3),
                    (8, 11),
                ]
                .into_iter(),
                "X",
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            table.text().unwrap(),
            "X two X",
        );

        assert_eq!(
            table.add.len(),
            add_length + 1,
        );

        assert_eq!(
            table.pieces
                .iter()
                .filter(|piece| {
                    !piece.original
                        && piece.start
                            == add_length
                        && piece.length == 1
                })
                .count(),
            2,
        );

        assert!(table.line_cache.is_empty());

        table.swap_snapshot(
            &mut snapshot
        );

        assert_eq!(
            table.text().unwrap(),
            "one two one",
        );

        assert_eq!(
            table.add.len(),
            add_length + 1,
        );

        table.swap_snapshot(
            &mut snapshot
        );

        assert_eq!(
            table.text().unwrap(),
            "X two X",
        );
    }

    #[test]
    fn replace_ranges_supports_adjacent_deletions() {
        let mut table =
            table_with_text("abcdef");

        table.replace_ranges(
            vec![(1, 3), (3, 5)]
                .into_iter(),
            "",
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            table.text().unwrap(),
            "af",
        );
    }

    #[test]
    fn replace_ranges_supports_zero_width_ranges_in_empty_document() {
        let mut table =
            PieceTable::empty()
                .unwrap();

        let add_length =
            table.add.len();

        table.replace_ranges(
            vec![(0, 0), (0, 0)]
                .into_iter(),
            "é",
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            table.text().unwrap(),
            "éé",
        );

        assert_eq!(
            table.add.len(),
            add_length + "é".len(),
        );
    }

    #[test]
    fn replace_ranges_uses_utf8_byte_ranges() {
        let mut table =
            table_with_text("aé😀z");

        table.replace_ranges(
            vec![(1, 3), (3, 7)]
                .into_iter(),
            "λ",
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            table.text().unwrap(),
            "aλλz",
        );
    }

    #[test]
    fn replace_ranges_validation_failures_are_atomic() {
        let mut table =
            table_with_text("aébcd");

        table.line_count()
            .unwrap();

        let text =
            table.text().unwrap();

        let layout =
            piece_layout(&table);

        let add_length =
            table.add.len();

        let length =
            table.len();

        let cache =
            table.line_cache
                .iter()
                .map(|line| {
                    (
                        line.start,
                        line.end,
                        line.length,
                    )
                })
                .collect::<Vec<_>>();

        let all_lines_cached =
            table.all_lines_cached;

        for ranges in [
            vec![(3, 2)],
            vec![(0, 3), (2, 4)],
            vec![(0, 99)],
            vec![(2, 3)],
        ] {
            assert!(
                table.replace_ranges(
                    ranges.into_iter(),
                    "replacement",
                )
                .is_err()
            );

            assert_eq!(
                table.text().unwrap(),
                text,
            );

            assert_eq!(
                piece_layout(&table),
                layout,
            );

            assert_eq!(
                table.add.len(),
                add_length,
            );

            assert_eq!(
                table.len(),
                length,
            );

            assert_eq!(
                table.line_cache
                    .iter()
                    .map(|line| {
                        (
                            line.start,
                            line.end,
                            line.length,
                        )
                    })
                    .collect::<Vec<_>>(),
                cache,
            );

            assert_eq!(
                table.all_lines_cached,
                all_lines_cached,
            );
        }
    }

    #[test]
    fn replace_ranges_handles_mixed_sources_and_merges_neighbors() {
        let (mut table, path) =
            original_table("abcdef");

        table.insert(3, "XYZ")
            .unwrap();

        assert!(
            table.pieces.iter()
                .any(|piece| piece.original)
        );

        assert!(
            table.pieces.iter()
                .any(|piece| !piece.original)
        );

        let mut deletion =
            table.replace_ranges(
                vec![(3, 6)].into_iter(),
                "",
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            table.text().unwrap(),
            "abcdef",
        );

        assert_eq!(table.pieces.len(), 1);
        assert!(table.pieces[0].original);
        assert_eq!(table.pieces[0].start, 0);
        assert_eq!(table.pieces[0].length, 6);

        table.swap_snapshot(
            &mut deletion
        );

        table.replace_ranges(
            vec![(2, 7)].into_iter(),
            "Q",
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            table.text().unwrap(),
            "abQef",
        );

        drop(table);
        std::fs::remove_file(path)
            .ok();
    }

    #[test]
    fn replace_ranges_empty_iterator_is_no_op() {
        let mut table =
            table_with_text("unchanged");

        let layout =
            piece_layout(&table);

        let add_length =
            table.add.len();

        let snapshot =
            table.replace_ranges(
                std::iter::empty(),
                "unused",
            )
            .unwrap();

        assert!(snapshot.is_none());
        assert_eq!(
            table.text().unwrap(),
            "unchanged",
        );
        assert_eq!(
            piece_layout(&table),
            layout,
        );
        assert_eq!(
            table.add.len(),
            add_length,
        );
    }

    #[test]
    fn replace_runs_compacts_dense_output_and_snapshot_swaps() {
        let mut table =
            PieceTable::empty()
                .unwrap();

        let repetitions =
            10 * 1024 * 1024;

        let mut snapshot =
            table.replace_runs(
                vec![(0, 0, repetitions)]
                    .into_iter(),
                "x",
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            table.len(),
            repetitions,
        );

        assert_eq!(
            table.add.len(),
            PIECE_TABLE_CHUNK_SIZE,
        );

        assert_eq!(
            table.pieces.len(),
            repetitions.div_ceil(
                PIECE_TABLE_CHUNK_SIZE
            ),
        );

        assert_eq!(
            table.read_range(0, 1)
                .unwrap(),
            b"x",
        );

        assert_eq!(
            table.read_range(
                repetitions - 1,
                1,
            )
            .unwrap(),
            b"x",
        );

        table.swap_snapshot(
            &mut snapshot
        );

        assert!(table.is_empty());
        assert!(table.pieces.is_empty());

        table.swap_snapshot(
            &mut snapshot
        );

        assert_eq!(
            table.len(),
            repetitions,
        );

        assert_eq!(
            table.add.len(),
            PIECE_TABLE_CHUNK_SIZE,
        );
    }

    #[test]
    fn replace_runs_validation_failures_are_atomic() {
        let mut table =
            table_with_text("aébcd");

        let text =
            table.text().unwrap();

        let layout =
            piece_layout(&table);

        let add_length =
            table.add.len();

        for runs in [
            vec![(0, 1, 0)],
            vec![(0, 1, 2), (2, 3, 1)],
            vec![(3, 5, 1), (1, 1, 1)],
        ] {
            assert!(
                table.replace_runs(
                    runs.into_iter(),
                    "replacement",
                )
                .is_err()
            );

            assert_eq!(
                table.text().unwrap(),
                text,
            );

            assert_eq!(
                piece_layout(&table),
                layout,
            );

            assert_eq!(
                table.add.len(),
                add_length,
            );
        }
    }

    #[test]
    fn visit_chunks_preserves_mixed_fragment_order() {
        let (mut table, path) =
            original_table("ace");

        table.insert(1, "B")
            .unwrap();

        table.insert(3, "D")
            .unwrap();

        let mut chunks = Vec::new();

        table.visit_chunks(|chunk| {
            chunks.push(
                std::str::from_utf8(chunk)
                    .unwrap()
                    .to_owned()
            );

            Ok(())
        })
        .unwrap();

        assert_eq!(
            chunks,
            ["a", "B", "c", "D", "e"],
        );

        assert_eq!(
            chunks.concat(),
            "aBcDe",
        );

        drop(table);
        std::fs::remove_file(path)
            .ok();
    }

    #[test]
    fn timing_open_and_first_line() {
        let mut path =
            std::env::temp_dir();

        path.push(
            "potyi_timing_test.txt"
        );

        {
            let mut file =
                File::create(&path)
                    .unwrap();

            for i in 0..100_000 {
                write!(
                    file,
                    "The quick brown fox jumps over the lazy dog.{}",
                    if i + 1 < 100_000 { "\n" } else { "" }
                )
                .unwrap();
            }
        }

        let path_string =
            path.to_string_lossy()
                .to_string();

        let start =
            Instant::now();

        let mut table =
            PieceTable::open(
                &path_string
            )
            .unwrap();

        let open_time =
            start.elapsed();

        let start =
            Instant::now();

        let line =
            table.line_text(0)
                .unwrap();

        let first_line_time =
            start.elapsed();

        let start =
            Instant::now();

        let line_count =
            table.line_count()
                .unwrap();

        let line_count_time =
            start.elapsed();

        println!(
            "Timing test:"
        );

        println!(
            "  open:       {:?}",
            open_time
        );

        println!(
            "  first line: {:?}",
            first_line_time
        );

        println!(
            "  line count: {:?}",
            line_count_time
        );

        println!(
            "  lines:      {}",
            line_count
        );

        assert_eq!(
            line,
            "The quick brown fox jumps over the lazy dog."
        );

        assert_eq!(
            line_count,
            100_000
        );

        fs::remove_file(
            path
        )
        .ok();
    }
}
