// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use sha2::{Digest, Sha256};
use std::io::{BufWriter, Seek, SeekFrom};
const HEADER: u64 = 25;
const OVERHEAD: u64 = HEADER + 32 + 16;
const MAGIC: &[u8; 8] = b"PTYREC01";
const END: &[u8; 8] = b"PTYEND01";

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn number(input: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    input.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
fn usize_number(input: &mut impl Read) -> io::Result<usize> {
    usize::try_from(number(input)?).map_err(|_| invalid("Recovery offset exceeds this platform"))
}
fn piece(input: &mut impl Read) -> io::Result<Piece> {
    let mut flag = [0];
    input.read_exact(&mut flag)?;
    if flag[0] > 1 {
        return Err(invalid("Invalid recovery piece source"));
    }
    Ok(Piece {
        original: flag[0] == 1,
        start: usize_number(input)?,
        length: usize_number(input)?,
    })
}

pub(super) fn write_frame(log: &mut File, change: Change<'_>, add_length: u64) -> io::Result<()> {
    let (tag, payload) = match change {
        Change::Insert(_, pieces) => (
            1u8,
            16u64
                .checked_add(
                    (pieces.len() as u64)
                        .checked_mul(17)
                        .ok_or_else(|| invalid("Recovery size overflow"))?,
                )
                .ok_or_else(|| invalid("Recovery size overflow"))?,
        ),
        Change::Delete(_, _) => (2, 16),
        Change::Snapshot(pieces) => (
            3,
            8u64.checked_add(
                (pieces.len() as u64)
                    .checked_mul(17)
                    .ok_or_else(|| invalid("Recovery size overflow"))?,
            )
            .ok_or_else(|| invalid("Recovery size overflow"))?,
        ),
        Change::Saved => (4, 0),
    };
    let start = log.seek(SeekFrom::End(0))?;
    let result = (|| {
        let mut output = BufWriter::with_capacity(8192, &mut *log);
        let mut hash = Sha256::new();
        let mut write = |bytes: &[u8]| -> io::Result<()> {
            output.write_all(bytes)?;
            hash.update(bytes);
            Ok(())
        };
        write(MAGIC)?;
        write(&payload.to_le_bytes())?;
        write(&add_length.to_le_bytes())?;
        write(&[tag])?;
        match change {
            Change::Insert(position, pieces) => {
                write(&(position as u64).to_le_bytes())?;
                write(&(pieces.len() as u64).to_le_bytes())?;
                for piece in pieces {
                    write(&[u8::from(piece.original)])?;
                    write(&(piece.start as u64).to_le_bytes())?;
                    write(&(piece.length as u64).to_le_bytes())?;
                }
            }
            Change::Delete(position, length) => {
                write(&(position as u64).to_le_bytes())?;
                write(&(length as u64).to_le_bytes())?;
            }
            Change::Snapshot(pieces) => {
                write(&(pieces.len() as u64).to_le_bytes())?;
                for piece in pieces {
                    write(&[u8::from(piece.original)])?;
                    write(&(piece.start as u64).to_le_bytes())?;
                    write(&(piece.length as u64).to_le_bytes())?;
                }
            }
            Change::Saved => (),
        }
        output.write_all(&hash.finalize())?;
        output.write_all(&(payload + OVERHEAD).to_le_bytes())?;
        output.write_all(END)?;
        output.flush()
    })();
    if result.is_err() {
        let _ = log.set_len(start);
        let _ = log.seek(SeekFrom::End(0));
    }
    result
}

// Validate a whole frame before mutating the replay table. The checksum pass
// streams through 8 KiB; large snapshot payloads are never buffered in RAM.
fn frame(log: &mut File, start: u64, file_length: u64) -> io::Result<Option<(u8, u64, u64, u64)>> {
    if file_length.saturating_sub(start) < OVERHEAD {
        return Ok(None);
    }
    log.seek(SeekFrom::Start(start))?;
    let mut magic = [0; 8];
    log.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Ok(None);
    }
    let payload = number(log)?;
    let add = number(log)?;
    let mut tag = [0];
    log.read_exact(&mut tag)?;
    let Some(end) = start
        .checked_add(OVERHEAD)
        .and_then(|p| p.checked_add(payload))
        .filter(|e| *e <= file_length)
    else {
        return Ok(None);
    };
    log.seek(SeekFrom::Start(start))?;
    let mut hash = Sha256::new();
    let mut remaining = HEADER + payload;
    let mut buffer = [0u8; 8192];
    while remaining > 0 {
        let size = remaining.min(buffer.len() as u64) as usize;
        log.read_exact(&mut buffer[..size])?;
        hash.update(&buffer[..size]);
        remaining -= size as u64;
    }
    let mut digest = [0u8; 32];
    log.read_exact(&mut digest)?;
    let length = number(log)?;
    log.read_exact(&mut magic)?;
    if hash.finalize()[..] != digest || length != OVERHEAD + payload || &magic != END {
        return Ok(None);
    }
    Ok(Some((tag[0], payload, add, end)))
}

pub(super) fn last_is_saved(path: &Path) -> io::Result<bool> {
    let mut log = File::open(path)?;
    let size = log.metadata()?.len();
    if size < OVERHEAD {
        return Ok(false);
    }
    log.seek(SeekFrom::End(-16))?;
    let length = number(&mut log)?;
    let mut end = [0; 8];
    log.read_exact(&mut end)?;
    if &end != END || length > size || length != OVERHEAD {
        return Ok(false);
    }
    Ok(frame(&mut log, size - length, size)?
        .is_some_and(|(tag, payload, _, _)| tag == 4 && payload == 0))
}

pub(super) fn replay(directory: &Path, meta: &Metadata) -> io::Result<(PieceTable, bool)> {
    let original = directory.join("original");
    if fs::metadata(&original)?.len() != meta.original_length {
        return Err(invalid("Original recovery snapshot is incomplete"));
    }
    let mut table = PieceTable::open(
        original
            .to_str()
            .ok_or_else(|| invalid("Recovery path is not UTF-8"))?,
    )?;
    let add = File::open(directory.join("add"))?;
    let add_length = add.metadata()?.len();
    table.add = super::super::EditStore {
        file: Some(add),
        path: directory.join("add"),
        length: usize::try_from(add_length)
            .map_err(|_| invalid("Edit store exceeds this platform"))?,
        remove_on_drop: false,
    };
    let mut log = File::open(directory.join("journal"))?;
    let file_length = log.metadata()?.len();
    let mut offset = 0;
    let mut frames = 0;
    while offset < file_length {
        let Some((tag, payload, add, end)) = frame(&mut log, offset, file_length)? else {
            break;
        };
        if add > add_length {
            break;
        }
        log.seek(SeekFrom::Start(offset + HEADER))?;
        match tag {
            1 | 3 => {
                let mut position = if tag == 1 { usize_number(&mut log)? } else { 0 };
                let count = number(&mut log)?;
                if count
                    .checked_mul(17)
                    .and_then(|n| n.checked_add(if tag == 1 { 16 } else { 8 }))
                    != Some(payload)
                {
                    return Err(invalid("Invalid recovery piece count"));
                }
                // First validate all references/ranges. This also bounds allocation
                // by real source bytes instead of trusting a declared piece count.
                let data_start = log.stream_position()?;
                let mut added = 0usize;
                for _ in 0..count {
                    let p = piece(&mut log)?;
                    let limit = if p.original {
                        meta.original_length
                    } else {
                        add
                    };
                    if p.length == 0
                        || (p.start as u64)
                            .checked_add(p.length as u64)
                            .is_none_or(|e| e > limit)
                    {
                        return Err(invalid("Recovery piece exceeds its backing file"));
                    }
                    added = added
                        .checked_add(p.length)
                        .ok_or_else(|| invalid("Recovery document length overflow"))?;
                }
                if tag == 1 && (position > table.len() || table.len().checked_add(added).is_none())
                {
                    return Err(invalid("Invalid recovery insertion position"));
                }
                log.seek(SeekFrom::Start(data_start))?;
                if tag == 3 {
                    table.pieces.clear();
                    table.length = 0;
                }
                for _ in 0..count {
                    let p = piece(&mut log)?;
                    if tag == 3 {
                        table.pieces.push(p);
                        table.length += p.length;
                    } else {
                        table.insert_pieces(position, std::slice::from_ref(&p))?;
                        position += p.length;
                    }
                }
            }
            2 if payload == 16 => {
                let position = usize_number(&mut log)?;
                let length = usize_number(&mut log)?;
                table.delete(position, length)?;
            }
            4 if payload == 0 => (),
            _ => return Err(invalid("Unknown recovery operation")),
        }
        offset = end;
        frames += 1;
    }
    if frames == 0 {
        return Err(invalid(
            "No complete recovery checkpoint; the interrupted session has been kept",
        ));
    }
    Ok((table, offset < file_length))
}
