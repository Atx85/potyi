// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

const MAX_OUTPUT: usize = 1024 * 1024;
const MAX_PATHS: usize = 8192;
const MAX_PATH_BYTES: usize = 1024 * 1024;

// Ordered by precedence when several children decorate the same directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Status {
    Ignored,
    Untracked,
    Added,
    Modified,
    Deleted,
    Conflict,
}

impl Status {
    pub(crate) fn color(self) -> (u8, u8, u8) {
        match self {
            Self::Ignored => (140, 145, 140),
            Self::Untracked => (120, 205, 220),
            Self::Added => (150, 220, 165),
            Self::Modified => (225, 185, 115),
            Self::Deleted => (230, 140, 140),
            Self::Conflict => (225, 140, 215),
        }
    }

    fn parse(xy: &[u8]) -> Option<Self> {
        match xy {
            b"!!" => Some(Self::Ignored),
            b"??" => Some(Self::Untracked),
            b"DD" | b"AU" | b"UD" | b"UA" | b"DU" | b"AA" | b"UU" => Some(Self::Conflict),
            _ if xy.contains(&b'D') => Some(Self::Deleted),
            _ if xy.contains(&b'M')
                || xy.contains(&b'R')
                || xy.contains(&b'C')
                || xy.contains(&b'T') =>
            {
                Some(Self::Modified)
            }
            _ if xy.contains(&b'A') => Some(Self::Added),
            _ => None,
        }
    }
}

pub(super) struct Snapshot {
    pub scope: PathBuf,
    paths: HashMap<PathBuf, Decoration>,
}

#[derive(Clone, Copy)]
struct Decoration {
    status: Status,
    descendants: Option<Status>,
}

impl Snapshot {
    pub fn status(&self, path: &Path) -> Option<Status> {
        if !path.starts_with(&self.scope) {
            return None;
        }
        if let Some(status) = self.paths.get(path) {
            return Some(status.status);
        }
        // Git collapses untracked/ignored directories. Inherit those states
        // for recursive listings without asking Git to enumerate their contents.
        for parent in path.ancestors().skip(1) {
            if !parent.starts_with(&self.scope) {
                break;
            }
            if let Some(status) = self.paths.get(parent).and_then(|value| value.descendants) {
                return Some(status);
            }
        }
        None
    }
}

pub(super) fn load(
    scope: &Path,
    cancelled: &Arc<AtomicBool>,
    deadline: Instant,
) -> Option<Snapshot> {
    let mut command = git(scope);
    command.args(["rev-parse", "--show-toplevel"]);
    let root = output(command, cancelled, deadline, 32 * 1024)?;
    let root = root.strip_suffix(b"\n").unwrap_or(&root);
    #[cfg(windows)]
    let root = root.strip_suffix(b"\r").unwrap_or(root);
    let root = path_from_bytes(root)?.canonicalize().ok()?;
    let mut command = git(&root);
    command.args([
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=normal",
        "--ignored=matching",
        "--ignore-submodules=all",
        "--no-renames",
        "--",
    ]);
    let relative = scope.strip_prefix(&root).ok()?;
    command.arg(if relative.as_os_str().is_empty() {
        Path::new(".")
    } else {
        relative
    });
    let bytes = output(command, cancelled, deadline, MAX_OUTPUT)?;
    parse(&bytes, &root, scope)
}

fn git(cwd: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(cwd)
        .args(["--no-optional-locks", "-c", "core.fsmonitor=false"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

// A capped pipe reader prevents an enormous status output from filling memory.
// Only this listing worker waits. Cancellation/timeout kills and reaps Git;
// neither shutdown nor the UI waits for it.
fn output(
    mut command: Command,
    cancelled: &Arc<AtomicBool>,
    deadline: Instant,
    limit: usize,
) -> Option<Vec<u8>> {
    if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
        return None;
    }
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let reader = std::thread::Builder::new()
        .name("potyi-git-status".into())
        .spawn(move || {
            let mut bytes = Vec::new();
            let result = stdout
                .take(limit as u64 + 1)
                .read_to_end(&mut bytes)
                .ok()
                .filter(|_| bytes.len() <= limit)
                .map(|_| bytes);
            let _ = sender.send(result);
        });
    if reader.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }
    let mut bytes = None;
    let result = loop {
        if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
            break None;
        }
        if bytes.is_none() {
            match receiver.try_recv() {
                Ok(Some(value)) => bytes = Some(value),
                Ok(None) | Err(mpsc::TryRecvError::Disconnected) => break None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => break None,
            Ok(Some(_)) if bytes.is_some() => break bytes,
            Err(_) => break None,
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if result.is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

fn parse(bytes: &[u8], root: &Path, scope: &Path) -> Option<Snapshot> {
    let mut snapshot = Snapshot {
        scope: scope.into(),
        paths: HashMap::new(),
    };
    let mut path_bytes = 0;
    let mut records = bytes.split(|&b| b == 0);
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        if record.len() < 4 || record[2] != b' ' {
            return None;
        }
        let status = Status::parse(&record[..2])?;
        let relative = path_from_bytes(&record[3..])?;
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return None;
        }
        let path = root.join(relative);
        let descendants = matches!(status, Status::Ignored | Status::Untracked)
            .then_some(status)
            .filter(|_| record.ends_with(b"/"));
        // Git can report a collapsed directory above a requested subdirectory.
        let path = if descendants.is_some() && scope.starts_with(&path) {
            scope.to_path_buf()
        } else {
            path
        };
        let mut current = path.as_path();
        while current.starts_with(scope) {
            if let Some(old) = snapshot.paths.get_mut(current) {
                old.status = old.status.max(status);
                if current == path {
                    old.descendants = descendants;
                }
            } else {
                path_bytes += current.as_os_str().len();
                if snapshot.paths.len() >= MAX_PATHS || path_bytes > MAX_PATH_BYTES {
                    return None;
                }
                snapshot.paths.insert(
                    current.into(),
                    Decoration {
                        status,
                        descendants: if current == path { descendants } else { None },
                    },
                );
            }
            // Ignored entries do not make their tracked parent look ignored.
            if status == Status::Ignored || current == scope {
                break;
            }
            let Some(parent) = current.parent() else {
                break;
            };
            current = parent;
        }
        // Accept rename records too, although collection disables rename detection.
        if record[..2].contains(&b'R') || record[..2].contains(&b'C') {
            records.next()?;
        }
    }
    Some(snapshot)
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> Option<PathBuf> {
    std::str::from_utf8(bytes).ok().map(PathBuf::from)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::terminal::tests::Fixture;

    #[test]
    fn git_status_parses_conflicts_renames_and_collapsed_directories() {
        let root = std::env::temp_dir().join("repo");
        let snapshot = parse(b" M src/changed.rs\0?? src/new.rs\0!! cache/\0?? new/\0R  renamed file\0old file\0UU conflict\0 D src/deleted\0", &root, &root).unwrap();
        for (name, expected) in [
            ("src/changed.rs", Some(Status::Modified)),
            ("src/new.rs", Some(Status::Untracked)),
            ("src/clean.rs", None),
            ("clean.rs", None),
            ("src", Some(Status::Deleted)),
            ("cache/deep/file", Some(Status::Ignored)),
            ("new/deep/file", Some(Status::Untracked)),
            ("renamed file", Some(Status::Modified)),
            ("old file", None),
            ("conflict", Some(Status::Conflict)),
        ] {
            assert_eq!(snapshot.status(&root.join(name)), expected, "{name}");
        }
        assert_eq!(snapshot.status(root.parent().unwrap()), None);
        let nested = root.join("cache/deep");
        let snapshot = parse(b"!! cache/\0", &root, &nested).unwrap();
        assert_eq!(snapshot.status(&nested.join("file")), Some(Status::Ignored));
    }

    #[test]
    fn git_status_bounds_metadata_and_preserves_filename_bytes() {
        let root = std::env::temp_dir();
        let snapshot = parse(b" M space \t newline\nname\0", &root, &root).unwrap();
        assert_eq!(
            snapshot.status(&root.join("space \t newline\nname")),
            Some(Status::Modified)
        );
        #[cfg(unix)]
        {
            let snapshot = parse(b" M invalid-\xff\0", &root, &root).unwrap();
            assert_eq!(
                snapshot.status(&root.join(path_from_bytes(b"invalid-\xff").unwrap())),
                Some(Status::Modified)
            );
        }
        let oversized: Vec<u8> = (0..MAX_PATHS)
            .flat_map(|i| format!("?? file-{i}\0").into_bytes())
            .collect();
        assert!(parse(&oversized, &root, &root).is_none());
        assert!(parse(b" M ../outside\0", &root, &root).is_none());
        assert!(parse(b"bad\0", &root, &root).is_none());
    }

    pub(in crate::terminal) fn repo_command(root: &Path, args: &[&str]) {
        let result = Command::new("git")
            .current_dir(root)
            .args([
                "-c",
                "user.name=Potyi Test",
                "-c",
                "user.email=potyi@example.invalid",
                "-c",
                "commit.gpgSign=false",
                "-c",
                "core.hooksPath=.no-hooks",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn git_status_reads_real_repository_and_subdirectory_without_locking_index() {
        let root = Fixture::new();
        repo_command(&root.0, &["init", "-q"]);
        std::fs::create_dir(root.0.join("src")).unwrap();
        for name in ["clean.txt", "src/changed é.txt", "src/deleted.txt"] {
            std::fs::write(root.0.join(name), "original\n").unwrap();
        }
        std::fs::write(root.0.join(".gitignore"), "ignored/\n").unwrap();
        repo_command(&root.0, &["add", "."]);
        repo_command(&root.0, &["commit", "-qm", "initial"]);
        std::fs::write(root.0.join("src/changed é.txt"), "changed\n").unwrap();
        std::fs::remove_file(root.0.join("src/deleted.txt")).unwrap();
        std::fs::write(root.0.join("added.txt"), "added\n").unwrap();
        repo_command(&root.0, &["add", "added.txt"]);
        std::fs::write(root.0.join(".new"), "new\n").unwrap();
        std::fs::create_dir(root.0.join("ignored")).unwrap();
        std::fs::write(root.0.join("ignored/cache"), "cache\n").unwrap();
        let index = std::fs::read(root.0.join(".git/index")).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let snapshot = load(&root.0, &cancelled, Instant::now() + Duration::from_secs(5)).unwrap();
        for (name, expected) in [
            ("clean.txt", None),
            ("src/changed é.txt", Some(Status::Modified)),
            ("src", Some(Status::Deleted)),
            ("added.txt", Some(Status::Added)),
            (".new", Some(Status::Untracked)),
            ("ignored", Some(Status::Ignored)),
            ("ignored/cache", Some(Status::Ignored)),
        ] {
            assert_eq!(snapshot.status(&root.0.join(name)), expected, "{name}");
        }
        assert_eq!(std::fs::read(root.0.join(".git/index")).unwrap(), index);
        assert!(!root.0.join(".git/index.lock").exists());
        let scope = root.0.join("src");
        let snapshot = load(&scope, &cancelled, Instant::now() + Duration::from_secs(5)).unwrap();
        assert_eq!(
            snapshot.status(&scope.join("changed é.txt")),
            Some(Status::Modified)
        );
        assert_eq!(snapshot.status(&root.0.join("added.txt")), None);
        assert!(load(&root.0, &cancelled, Instant::now()).is_none());
        cancelled.store(true, Ordering::Relaxed);
        assert!(load(&root.0, &cancelled, Instant::now() + Duration::from_secs(5)).is_none());
    }

    #[test]
    fn git_status_output_is_bounded_and_slow_children_are_reaped() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut command = Command::new("git");
        command.arg("--version").stdout(Stdio::piped());
        assert!(
            output(
                command,
                &cancelled,
                Instant::now() + Duration::from_secs(5),
                1
            )
            .is_none()
        );
        #[cfg(unix)]
        {
            let mut command = Command::new("sh");
            command.args(["-c", "exec sleep 10"]).stdout(Stdio::piped());
            let started = Instant::now();
            assert!(
                output(
                    command,
                    &cancelled,
                    started + Duration::from_millis(40),
                    MAX_OUTPUT
                )
                .is_none()
            );
            assert!(started.elapsed() < Duration::from_secs(2));
        }
    }
}
