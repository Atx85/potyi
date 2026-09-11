// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
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

use super::TerminalEvent;
use sdl3::event::EventSender;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, sync_channel},
};
use std::thread;

use super::{DirectoryListing, LsOptions, append_ls_entry, parse_ls_arguments, resolve_ls_path};

const CHUNK_BYTES: usize = 16 * 1024;
const CHUNK_ENTRIES: usize = 64;
const QUEUED_CHUNKS: usize = 8;

pub(super) enum Message {
    Started(Option<PathBuf>),
    Chunk(DirectoryListing),
    Finished(Result<(), String>),
}

pub(super) struct Job {
    pub receiver: Receiver<Message>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

pub(super) fn start(
    arguments: String,
    cwd: PathBuf,
    enter: Option<PathBuf>,
    events: Option<EventSender>,
) -> io::Result<Job> {
    let (sender, receiver) = sync_channel(QUEUED_CHUNKS);
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = cancelled.clone();
    thread::Builder::new()
        .name("potyi-listing".into())
        .spawn(move || {
            let mut writer = Writer {
                sender,
                events,
                cancelled: worker_cancelled,
                chunk: DirectoryListing::default(),
            };
            let result = (|| {
                let cwd = if let Some(path) = enter {
                    let target = path.canonicalize()?;
                    if !target.is_dir() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Not a directory",
                        ));
                    }
                    writer.send(Message::Started(Some(target.clone())))?;
                    writer
                        .chunk
                        .text
                        .push_str(&format!("{}:\n", target.display()));
                    target
                } else {
                    writer.send(Message::Started(None))?;
                    cwd
                };
                let (options, paths) = parse_ls_arguments(&arguments)?;
                for (index, value) in paths.iter().enumerate() {
                    writer.check_cancelled()?;
                    if index > 0 {
                        writer.chunk.text.push('\n');
                    }
                    writer.path(
                        &resolve_ls_path(value, &cwd),
                        value,
                        &options,
                        paths.len() > 1 || options.recursive,
                    )?;
                }
                writer.chunk.text.push('\n');
                writer.flush()
            })();
            // Preserve any entries produced before a filesystem error.
            if !writer.chunk.text.is_empty() {
                let _ = writer.flush();
            }
            let _ = writer.send(Message::Finished(
                result.map_err(|error: io::Error| error.to_string()),
            ));
        })?;
    Ok(Job {
        receiver,
        cancelled,
    })
}

struct Writer {
    sender: SyncSender<Message>,
    events: Option<EventSender>,
    cancelled: Arc<AtomicBool>,
    chunk: DirectoryListing,
}

impl Writer {
    fn check_cancelled(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Listing cancelled",
            ))
        } else {
            Ok(())
        }
    }

    fn send(&self, message: Message) -> io::Result<()> {
        self.check_cancelled()?;
        // The bounded queue applies backpressure; dropping Job unblocks send.
        self.sender
            .send(message)
            .map_err(|_| io::Error::new(io::ErrorKind::Interrupted, "Listing closed"))?;
        if let Some(events) = &self.events {
            let _ = events.push_custom_event(TerminalEvent::ListingReady);
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        let chunk = std::mem::take(&mut self.chunk);
        if !chunk.text.is_empty() {
            self.send(Message::Chunk(chunk))?;
        }
        Ok(())
    }

    fn entry(&mut self, path: &Path, name: &str, options: &LsOptions) -> io::Result<()> {
        self.check_cancelled()?;
        append_ls_entry(&mut self.chunk, path, name, options)?;
        if self.chunk.text.len() >= CHUNK_BYTES || self.chunk.entries.len() >= CHUNK_ENTRIES {
            self.flush()?;
        }
        Ok(())
    }

    fn path(
        &mut self,
        path: &Path,
        name: &str,
        options: &LsOptions,
        header: bool,
    ) -> io::Result<()> {
        self.check_cancelled()?;
        if !std::fs::metadata(path)?.is_dir() {
            return self.entry(path, name, options);
        }
        if header {
            self.chunk.text.push_str(&format!("{name}:\n"));
        }
        if let Some(parent) = path.canonicalize()?.parent() {
            self.entry(parent, "..", options)?;
        }
        // Show an initial batch immediately; large directories do not need to
        // be read or sorted in full before the first result reaches the UI.
        self.flush()?;
        for entry in std::fs::read_dir(path)? {
            self.check_cancelled()?;
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !options.all && name.starts_with('.') {
                continue;
            }
            self.entry(&entry.path(), &name, options)?;
            if options.recursive && entry.file_type()?.is_dir() {
                self.chunk.text.push('\n');
                self.path(
                    &entry.path(),
                    &entry.path().display().to_string(),
                    options,
                    true,
                )?;
            }
        }
        self.flush()
    }
}
