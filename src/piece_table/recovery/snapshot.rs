// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Clone the open original when the filesystem supports copy-on-write. A hard
//! link would not isolate outside writes; reopening the source path could copy
//! a replacement file instead of the original backing the current piece table.
use super::{copy_range, create_private};
use std::{fs::File, io, path::Path};

pub(super) fn create(input: &File, path: &Path, length: u64) -> io::Result<File> {
    create_with(input, path, length, try_clone)
}

fn create_with(
    input: &File,
    path: &Path,
    length: u64,
    clone: impl FnOnce(&File, &Path) -> io::Result<Option<File>>,
) -> io::Result<File> {
    // Untitled buffers use a null-device original: do not try to clone it.
    if length != 0 {
        if let Some(output) = clone(input, path)? {
            if output.metadata()?.len() < length {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Recovery source was shortened",
                ));
            }
            // Match the original length captured at open, including when an
            // outside writer appended text before the first edit.
            output.set_len(length)?;
            return Ok(output);
        }
    }
    let mut output = create_private(path)?;
    if cfg!(windows) && length != 0 {
        // Windows seek_read changes the file cursor, unlike Unix read_at.
        // Preserve it once for the whole copy, even if reading/writing fails.
        // Empty originals may be null devices, which cannot be sought.
        preserving_source_cursor(input, || copy_range(input, &mut output, 0, length))?;
    } else {
        copy_range(input, &mut output, 0, length)?;
    }
    Ok(output)
}

fn preserving_source_cursor(mut input: &File, copy: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    use std::io::{Seek, SeekFrom};
    let position = input.stream_position()?;
    let result = copy();
    // Attempt restoration before propagating a copy error.
    let restored = input.seek(SeekFrom::Start(position)).map(|_| ());
    result.and(restored)
}

#[cfg(target_os = "macos")]
fn try_clone(input: &File, path: &Path) -> io::Result<Option<File>> {
    use std::{
        ffi::CString,
        fs,
        os::unix::{ffi::OsStrExt, fs::PermissionsExt, io::AsRawFd},
    };
    let destination = CString::new(path.as_os_str().as_bytes())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    // CLONE_NOOWNERCOPY: keep the new file owned by this process's user.
    // SAFETY: the source descriptor and NUL-terminated destination remain valid
    // throughout the call. fclonefileat creates a new independent inode, or
    // fails without creating it (including unsupported/cross-volume cases).
    let result = unsafe {
        libc::fclonefileat(
            input.as_raw_fd(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            0x0002,
        )
    };
    if result != 0 {
        return Ok(None);
    }
    // The clone inherits source mode bits, including read-only/public modes.
    // It lives inside our private recovery directory; make the file private too.
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map(Some)
}

#[cfg(target_os = "linux")]
fn try_clone(input: &File, path: &Path) -> io::Result<Option<File>> {
    use std::{fs, os::fd::AsRawFd};
    let output = create_private(path)?;
    // SAFETY: both descriptors are live regular files; FICLONE takes the source
    // descriptor by value and does not access a userspace pointer.
    let result = unsafe { libc::ioctl(output.as_raw_fd(), libc::FICLONE, input.as_raw_fd()) };
    if result == 0 {
        return Ok(Some(output));
    }
    // Only remove the destination we just created. The caller will recreate it
    // and stream the original with the existing fixed-size buffer.
    drop(output);
    fs::remove_file(path)?;
    Ok(None)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn try_clone(_input: &File, _path: &Path) -> io::Result<Option<File>> {
    Ok(None)
}

#[cfg(test)]
mod tests;
