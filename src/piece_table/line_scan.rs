// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Retain unread bytes between lazy line discoveries, bounded to one block.
use super::{Document as PieceTable, PIECE_TABLE_CHUNK_SIZE};
use std::io;

pub(super) struct LineScanBuffer {
    start: usize,
    bytes: Vec<u8>,
    block_size: usize,
}

impl Default for LineScanBuffer {
    fn default() -> Self {
        Self {
            start: 0,
            bytes: Vec::new(),
            block_size: PIECE_TABLE_CHUNK_SIZE,
        }
    }
}

impl LineScanBuffer {
    pub(super) fn read<'a>(
        &'a mut self,
        table: &PieceTable,
        position: usize,
    ) -> io::Result<&'a [u8]> {
        if position < self.start || position - self.start >= self.bytes.len() {
            if !self.bytes.is_empty() {
                self.block_size = (self.block_size * 2).min(PIECE_TABLE_CHUNK_SIZE);
            }
            self.bytes
                .resize((table.len() - position).min(self.block_size), 0);
            self.start = position;
            match table.read_range_into(position, &mut self.bytes) {
                Ok(read) => self.bytes.truncate(read),
                Err(error) => {
                    self.bytes.clear();
                    return Err(error);
                }
            }
        }
        Ok(&self.bytes[position - self.start..])
    }

    pub(super) fn clear(&mut self) {
        self.bytes.clear();
        // Editing usually needs one short line. Grow geometrically only when
        // navigation or a long line consumes the smaller refill.
        self.block_size = 256;
    }

    #[cfg(test)]
    pub(super) fn allocated_bytes(&self) -> usize {
        self.bytes.capacity()
    }
}
