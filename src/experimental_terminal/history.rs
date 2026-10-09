// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Fixed-size disk ring. The row index is on disk, not a growing heap vector.
use std::{
    fs::{File, OpenOptions},
    io::{self, BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};
pub(super) const HISTORY_BYTES: usize = 8 * 1024 * 1024;
const INDEX_ROWS: usize = 65536;
const INDEX_RECORD: usize = 16;
const DATA_BYTES: usize = HISTORY_BYTES - INDEX_ROWS * INDEX_RECORD;
// Encoded records include a bounded command plus native path metadata.
const MAX_ROW_BYTES: usize = 256 * 1024;
const DATA_READ_BYTES: usize = 64 * 1024;
const INDEX_READ_BYTES: usize = 16 * 1024;

/// One fixed read-ahead block per file region. Appends and in-place updates
/// invalidate only overlapping bytes; immutable rows can be scanned without
/// one seek/read pair for every small record. This is a cache, never an index
/// or a second transcript, and its allocation is independent of output size.
#[derive(Debug)]
struct ReadBlock {
    offset: usize,
    bytes: Vec<u8>,
    valid: bool,
    #[cfg(test)]
    fills: usize,
}
impl ReadBlock {
    fn new(size: usize) -> Self {
        Self {
            offset: 0,
            bytes: vec![0; size],
            valid: false,
            #[cfg(test)]
            fills: 0,
        }
    }
    fn invalidate(&mut self, offset: usize, length: usize) {
        if length != 0 && offset < self.offset + self.bytes.len() && self.offset < offset + length {
            self.valid = false;
        }
    }
    fn read(
        &mut self,
        reader: &mut File,
        writer: &mut BufWriter<File>,
        region: usize,
        region_length: usize,
        mut offset: usize,
        mut output: &mut [u8],
    ) -> io::Result<()> {
        while !output.is_empty() {
            if !self.valid || offset < self.offset || offset >= self.offset + self.bytes.len() {
                writer.flush()?;
                self.valid = false;
                self.offset = offset / self.bytes.len() * self.bytes.len();
                let length = self.bytes.len().min(region_length - self.offset);
                reader.seek(SeekFrom::Start((region + self.offset) as u64))?;
                reader.read_exact(&mut self.bytes[..length])?;
                self.valid = true;
                #[cfg(test)]
                {
                    self.fills += 1;
                }
            }
            let start = offset - self.offset;
            let count = output.len().min(self.bytes.len() - start);
            output[..count].copy_from_slice(&self.bytes[start..start + count]);
            output = &mut output[count..];
            offset += count;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct Row {
    pub cols: u16,
    pub wrapped: bool,
    pub bytes: Vec<u8>,
}
#[derive(Debug)]
pub(super) struct History {
    path: PathBuf,
    data: Option<BufWriter<File>>,
    index: Option<BufWriter<File>>,
    reader: Option<File>,
    data_read: ReadBlock,
    index_read: ReadBlock,
    first: usize,
    count: usize,
    used: usize,
    write: usize,
    pub(super) serial: u64,
    pub(super) error: Option<String>,
    failed: bool,
}
impl History {
    pub(super) fn new() -> io::Result<Self> {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).map_err(|e| io::Error::other(e.to_string()))?;
        let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let path = std::env::temp_dir().join(format!("potyi-terminal-{suffix}.history"));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path)?;
        // Construct the owner first, so every failure closes handles and
        // removes the temporary file through Drop.
        let mut history = Self {
            path,
            data: Some(BufWriter::with_capacity(32 * 1024, file)),
            index: None,
            reader: None,
            data_read: ReadBlock::new(DATA_READ_BYTES),
            index_read: ReadBlock::new(INDEX_READ_BYTES),
            first: 0,
            count: 0,
            used: 0,
            write: 0,
            serial: 0,
            error: None,
            failed: false,
        };
        history
            .data
            .as_ref()
            .unwrap()
            .get_ref()
            .set_len(HISTORY_BYTES as u64)?;
        history.reader = Some(OpenOptions::new().read(true).open(&history.path)?);
        let mut index = OpenOptions::new().write(true).open(&history.path)?;
        index.seek(SeekFrom::Start(DATA_BYTES as u64))?;
        history.index = Some(BufWriter::with_capacity(8 * 1024, index));
        Ok(history)
    }
    pub(super) fn len(&self) -> usize {
        self.count
    }
    pub(super) fn clear(&mut self) -> io::Result<()> {
        self.data_read.valid = false;
        self.index_read.valid = false;
        self.data.as_mut().unwrap().seek(SeekFrom::Start(0))?;
        self.index
            .as_mut()
            .unwrap()
            .seek(SeekFrom::Start(DATA_BYTES as u64))?;
        self.first = 0;
        self.count = 0;
        self.used = 0;
        self.write = 0;
        Ok(())
    }
    pub(super) fn flush(&mut self) -> io::Result<()> {
        self.data.as_mut().unwrap().flush()?;
        self.index.as_mut().unwrap().flush()
    }
    fn metadata(&mut self, row: usize) -> io::Result<[u8; INDEX_RECORD]> {
        let mut bytes = [0u8; INDEX_RECORD];
        let slot = (self.first + row) % INDEX_ROWS;
        self.index_read.read(
            self.reader.as_mut().unwrap(),
            self.index.as_mut().unwrap(),
            DATA_BYTES,
            INDEX_ROWS * INDEX_RECORD,
            slot * INDEX_RECORD,
            &mut bytes,
        )?;
        Ok(bytes)
    }
    fn evict(&mut self) -> io::Result<()> {
        // Evict in small batches to amortize disk-index reads under sustained output.
        let count = self.count.min(256);
        let mut bytes = [0u8; 256 * INDEX_RECORD];
        let first = count.min(INDEX_ROWS - self.first);
        self.index_read.read(
            self.reader.as_mut().unwrap(),
            self.index.as_mut().unwrap(),
            DATA_BYTES,
            INDEX_ROWS * INDEX_RECORD,
            self.first * INDEX_RECORD,
            &mut bytes[..first * INDEX_RECORD],
        )?;
        if first < count {
            self.index_read.read(
                self.reader.as_mut().unwrap(),
                self.index.as_mut().unwrap(),
                DATA_BYTES,
                INDEX_ROWS * INDEX_RECORD,
                0,
                &mut bytes[first * INDEX_RECORD..count * INDEX_RECORD],
            )?;
        }
        for record in bytes[..count * INDEX_RECORD].chunks_exact(INDEX_RECORD) {
            self.used -= u32::from_le_bytes(record[4..8].try_into().unwrap()) as usize;
        }
        self.first = (self.first + count) % INDEX_ROWS;
        self.count -= count;
        Ok(())
    }
    fn append(&mut self, cols: u16, wrapped: bool, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() > MAX_ROW_BYTES {
            return Err(io::Error::other("History row exceeds its storage limit"));
        }
        while self.count == INDEX_ROWS || self.used + bytes.len() > DATA_BYTES {
            self.evict()?;
        }
        let offset = self.write;
        let first = bytes.len().min(DATA_BYTES - self.write);
        self.data_read.invalidate(self.write, first);
        self.data_read.invalidate(0, bytes.len() - first);
        let data = self.data.as_mut().unwrap();
        data.write_all(&bytes[..first])?;
        if first < bytes.len() {
            data.seek(SeekFrom::Start(0))?;
            data.write_all(&bytes[first..])?;
        }
        self.write = (self.write + bytes.len()) % DATA_BYTES;
        if self.write == 0 {
            data.seek(SeekFrom::Start(0))?;
        }
        let slot = (self.first + self.count) % INDEX_ROWS;
        self.index_read
            .invalidate(slot * INDEX_RECORD, INDEX_RECORD);
        if slot == 0 {
            self.index
                .as_mut()
                .unwrap()
                .seek(SeekFrom::Start(DATA_BYTES as u64))?;
        }
        let mut meta = [0u8; INDEX_RECORD];
        meta[..4].copy_from_slice(&(offset as u32).to_le_bytes());
        meta[4..8].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        meta[8..10].copy_from_slice(&cols.to_le_bytes());
        meta[10] = u8::from(wrapped);
        self.index.as_mut().unwrap().write_all(&meta)?;
        self.used += bytes.len();
        self.count += 1;
        self.serial = self.serial.saturating_add(1);
        Ok(())
    }
    pub(super) fn row(&mut self, row: usize) -> io::Result<Row> {
        if row >= self.count {
            return Err(io::Error::other("History row is out of range"));
        }
        let meta = self.metadata(row)?;
        let offset = u32::from_le_bytes(meta[..4].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(meta[4..8].try_into().unwrap()) as usize;
        if offset >= DATA_BYTES || size > MAX_ROW_BYTES {
            return Err(io::Error::other("Invalid history record"));
        }
        let mut bytes = vec![0; size];
        let first = size.min(DATA_BYTES - offset);
        self.data_read.read(
            self.reader.as_mut().unwrap(),
            self.data.as_mut().unwrap(),
            0,
            DATA_BYTES,
            offset,
            &mut bytes[..first],
        )?;
        if first < size {
            self.data_read.read(
                self.reader.as_mut().unwrap(),
                self.data.as_mut().unwrap(),
                0,
                DATA_BYTES,
                0,
                &mut bytes[first..],
            )?;
        }
        Ok(Row {
            cols: u16::from_le_bytes(meta[8..10].try_into().unwrap()),
            wrapped: meta[10] != 0,
            bytes,
        })
    }
    /// Update fixed-size metadata in place without changing record identity.
    pub(super) fn replace(&mut self, row: usize, bytes: &[u8]) -> io::Result<()> {
        if row >= self.count {
            return Err(io::Error::other("History row is out of range"));
        }
        self.flush()?;
        let meta = self.metadata(row)?;
        let offset = u32::from_le_bytes(meta[..4].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(meta[4..8].try_into().unwrap()) as usize;
        if offset >= DATA_BYTES || size > MAX_ROW_BYTES || size != bytes.len() {
            return Err(io::Error::other(
                "History replacement must preserve record size",
            ));
        }
        let first = size.min(DATA_BYTES - offset);
        self.data_read.invalidate(offset, first);
        self.data_read.invalidate(0, size - first);
        let data = self.data.as_mut().unwrap();
        data.seek(SeekFrom::Start(offset as u64))?;
        data.write_all(&bytes[..first])?;
        if first < size {
            data.seek(SeekFrom::Start(0))?;
            data.write_all(&bytes[first..])?;
        }
        data.seek(SeekFrom::Start(self.write as u64))?;
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn buffer_capacity(&self) -> usize {
        self.data.as_ref().unwrap().capacity()
            + self.index.as_ref().unwrap().capacity()
            + self.data_read.bytes.capacity()
            + self.index_read.bytes.capacity()
    }
}
impl vt100::ScrollbackSink for History {
    fn push(&mut self, cols: u16, wrapped: bool, formatted: &[u8]) {
        if !self.failed
            && let Err(error) = self.append(cols, wrapped, formatted)
        {
            self.failed = true;
            self.error = Some(format!("History write failed: {error}"));
        }
    }
    fn clear(&mut self) {
        if let Err(error) = History::clear(self) {
            self.failed = true;
            self.error = Some(format!("History clear failed: {error}"));
        }
    }
}
impl Drop for History {
    fn drop(&mut self) {
        // Close every handle before deletion, including on Windows.
        self.data.take();
        self.index.take();
        self.reader.take();
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vt100::ScrollbackSink;
    #[test]
    fn sequential_small_rows_amortize_reads_without_growing_the_cache() {
        let mut history = History::new().unwrap();
        let capacity = history.buffer_capacity();
        for index in 0..4000 {
            history.push(80, false, format!("cache-row-{index:04}").as_bytes());
        }
        for index in 0..4000 {
            assert_eq!(
                history.row(index).unwrap().bytes,
                format!("cache-row-{index:04}").as_bytes()
            );
        }
        assert_eq!(history.data_read.fills, 1);
        assert_eq!(history.index_read.fills, 4);
        assert_eq!(history.buffer_capacity(), capacity);
        assert_eq!(history.data_read.bytes.len(), DATA_READ_BYTES);
        assert_eq!(history.index_read.bytes.len(), INDEX_READ_BYTES);
    }
    #[test]
    fn cached_blocks_are_invalidated_by_buffered_append_replace_clear_and_wrap() {
        let mut history = History::new().unwrap();
        history.push(80, false, b"old");
        assert_eq!(history.row(0).unwrap().bytes, b"old");
        // These writes are still buffered and overlap both loaded blocks.
        history.push(90, true, b"new");
        assert_eq!(history.row(1).unwrap().bytes, b"new");
        assert!(history.row(1).unwrap().wrapped);
        history.replace(0, b"upd").unwrap();
        assert_eq!(history.row(0).unwrap().bytes, b"upd");
        history.clear().unwrap();
        history.push(100, false, b"clear");
        assert_eq!(history.row(0).unwrap().bytes, b"clear");
        // Keep reading the tail during eviction, data-ring wrap and index wrap.
        let payload = vec![b'a'; 99];
        for index in 0..80000 {
            let mut text = payload.clone();
            text[..8].copy_from_slice(format!("{index:08}").as_bytes());
            history.push(100, false, &text);
            if index % 127 == 0 {
                assert_eq!(history.row(history.len() - 1).unwrap().bytes, text);
            }
        }
        assert_eq!(
            &history.row(history.len() - 1).unwrap().bytes[..8],
            b"00079999"
        );
        let first_id = 80000 - history.len();
        assert_eq!(
            &history.row(0).unwrap().bytes[..8],
            format!("{first_id:08}").as_bytes()
        );
    }
    #[test]
    fn metadata_replacement_preserves_ring_ids_and_future_appends() {
        let mut history = History::new().unwrap();
        let payload = vec![b'x'; 60000];
        for _ in 0..200 {
            history.push(80, false, &payload);
        }
        let len = history.len();
        let serial = history.serial;
        for index in [0, len / 2, len - 1] {
            let replacement = vec![b'a' + (index % 20) as u8; 60000];
            history.replace(index, &replacement).unwrap();
            assert_eq!(history.row(index).unwrap().bytes, replacement);
        }
        assert_eq!(history.serial, serial);
        assert_eq!(history.len(), len);
        assert!(history.replace(0, b"wrong size").is_err());
        history.push(80, false, b"after");
        assert_eq!(history.row(history.len() - 1).unwrap().bytes, b"after");
    }
    #[test]
    fn file_history_wraps_data_and_disk_index_with_constant_work_buffers() {
        let mut history = History::new().unwrap();
        let capacity = history.buffer_capacity();
        for index in 0..80000 {
            history.push(80, false, format!("ROW_{index:06}").as_bytes());
        }
        assert!(history.len() <= INDEX_ROWS);
        assert_eq!(history.buffer_capacity(), capacity);
        assert_eq!(history.row(history.len() - 1).unwrap().bytes, b"ROW_079999");
        let payload = vec![b'x'; MAX_ROW_BYTES - 100];
        for _ in 0..160 {
            history.push(256, true, &payload);
        }
        assert!(history.used <= DATA_BYTES);
        assert_eq!(history.row(history.len() - 1).unwrap().bytes, payload);
        assert_eq!(
            std::fs::metadata(&history.path).unwrap().len(),
            HISTORY_BYTES as u64
        );
        let path = history.path.clone();
        drop(history);
        assert!(!path.exists());
    }
    #[test]
    fn file_history_clear_reuses_the_file_and_preserves_row_metadata() {
        let mut history = History::new().unwrap();
        history.push(80, true, "é界".as_bytes());
        let row = history.row(0).unwrap();
        assert_eq!(row.cols, 80);
        assert!(row.wrapped);
        assert_eq!(row.bytes, "é界".as_bytes());
        history.clear().unwrap();
        assert_eq!(history.len(), 0);
        history.push(120, false, b"new");
        assert_eq!(history.row(0).unwrap().bytes, b"new");
        assert_eq!(history.row(0).unwrap().cols, 120);
    }
}
