// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

//! Explicit, synchronous document formatting. Configuration is loaded only
//! when formatting or showing formatter choices; no worker or watcher is used.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::piece_table::PieceTable;
use serde::Deserialize;

pub(crate) mod process;
pub(crate) mod builtin;

pub(crate) const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_ERROR_BYTES: usize = 64 * 1024;
const CHUNK_BYTES: usize = 16 * 1024;
const TIMEOUT: Duration = Duration::from_secs(3);

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Provider {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    filenames: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Formatters {
    #[serde(default)]
    providers: Vec<Provider>,
}

pub(crate) struct FormatterChoice {
    pub name: String,
    pub description: String,
}

impl Formatters {
    pub fn load(path: &Path) -> io::Result<Self> {
        let mut contents = String::new();
        match File::open(path) {
            Ok(file) => {
                file.take(MAX_CONFIG_BYTES + 1)
                    .read_to_string(&mut contents)?;
                if contents.len() as u64 > MAX_CONFIG_BYTES {
                    return Err(invalid("Formatter configuration exceeds 64 KiB"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                contents.push_str(crate::embedded_config::FORMATTERS);
            }
            Err(error) => return Err(error),
        }
        Self::parse(&contents)
    }

    fn parse(contents: &str) -> io::Result<Self> {
        let config: Self = toml::from_str(contents)
            .map_err(|error| invalid(format!("Invalid formatters.toml: {error}")))?;
        if config.providers.len() > 32 {
            return Err(invalid("At most 32 formatter providers are supported"));
        }
        for (index, provider) in config.providers.iter().enumerate() {
            if provider.name == "builtin" {
                return Err(invalid("The formatter name 'builtin' is reserved for built-in indentation"));
            }
            if provider.name.is_empty()
                || provider.name.len() > 40
                || !provider
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || provider.command.trim().is_empty()
                || provider.command.contains('\0')
                || provider.args.len() > 32
                || provider.args.iter().any(|arg| arg.contains('\0'))
            {
                return Err(invalid("Invalid formatter name, command, or arguments"));
            }
            if config.providers[..index]
                .iter()
                .any(|p| p.name == provider.name)
            {
                return Err(invalid(format!("Duplicate formatter: {}", provider.name)));
            }
        }
        Ok(config)
    }

    /// Config order is the preference order. Availability checks inspect the
    /// filesystem only; they never launch --version probes or package managers.
    pub fn select(
        &self,
        path: Option<&Path>,
        name: Option<&str>,
    ) -> io::Result<(&Provider, PathBuf)> {
        if let Some(name) = name {
            let provider = self
                .providers
                .iter()
                .find(|p| p.name == name)
                .ok_or_else(|| invalid(format!("Unknown formatter '{name}'; use :formatters")))?;
            return provider
                .executable()
                .map(|executable| (provider, executable));
        }
        let path = path.ok_or_else(|| {
            invalid("Save the document to choose its language, or use :format <provider>")
        })?;
        let mut matches = self.providers.iter().filter(|p| p.matches(path)).peekable();
        if matches.peek().is_none() {
            return Err(invalid(
                "No matching formatter; use :formatters or configure config/formatters.toml",
            ));
        }
        for provider in matches {
            if let Ok(executable) = provider.executable() {
                return Ok((provider, executable));
            }
        }
        Err(invalid(
            "No matching formatter is installed; use :formatters",
        ))
    }

    pub fn choices(&self, path: Option<&Path>) -> Vec<FormatterChoice> {
        std::iter::once(FormatterChoice {
            name: "builtin".into(),
            description: if builtin::supported(path) {
                "Basic C-style/JSON indentation — no installation needed"
            } else {
                "Not supported for this file type; choose a language formatter"
            }.into(),
        }).chain(self.providers
            .iter()
            .filter(|provider| path.is_none_or(|path| provider.matches(path)))
            .map(|provider| {
                let files = provider.extensions.iter().map(|ext| format!(".{ext}"))
                    .chain(provider.filenames.iter().cloned()).collect::<Vec<_>>().join(", ");
                let availability = if provider.executable().is_ok() { "Installed" }
                    else { "Not installed; install tool or set its path" };
                FormatterChoice {
                    name: provider.name.clone(),
                    description: format!("{availability} — {files}"),
                }
            }))
            .collect()
    }
}

impl Provider {
    fn matches(&self, path: &Path) -> bool {
        path.extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| {
                self.extensions
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            })
            || path
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| self.filenames.iter().any(|candidate| candidate == name))
    }

    fn executable(&self) -> io::Result<PathBuf> {
        let command = Path::new(&self.command);
        if command.components().count() > 1 || command.is_absolute() {
            let absolute = std::path::absolute(command)?;
            if executable_file(&absolute) {
                return Ok(absolute);
            }
        } else if let Some(path) = std::env::var_os("PATH") {
            for directory in std::env::split_paths(&path) {
                let candidate = std::path::absolute(directory.join(command))?;
                if executable_file(&candidate) {
                    return Ok(candidate);
                }
                #[cfg(windows)]
                if command.extension().is_none() {
                    let candidate = candidate.with_extension("exe");
                    if executable_file(&candidate) {
                        return Ok(candidate);
                    }
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Formatter '{}' is unavailable; install {} or set its executable path",
                self.name, self.command
            ),
        ))
    }

    fn arguments(&self, filepath: &Path) -> Vec<OsString> {
        self.args
            .iter()
            .map(|arg| {
                let mut value = OsString::new();
                let mut parts = arg.split("{filepath}");
                value.push(parts.next().unwrap_or_default());
                for part in parts {
                    value.push(filepath);
                    value.push(part);
                }
                value
            })
            .collect()
    }
}

fn executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        // Batch scripts require a shell; only directly executable programs
        // belong in the foreground formatter pipeline.
        path.extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    }
}

pub(crate) struct Scratch {
    directory: PathBuf,
}

impl Scratch {
    pub fn create() -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..1024 {
            let directory = std::env::temp_dir().join(format!(
                "potyi-format-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&directory) {
                Ok(()) => return Ok(Self { directory }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(invalid("Could not create formatter temporary directory"))
    }

    pub fn file(&self, name: &str) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(self.directory.join(name))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

pub(crate) fn run(
    provider: &Provider,
    executable: &Path,
    source_path: Option<&Path>,
    document: &PieceTable,
    scratch: &Scratch,
) -> io::Result<File> {
    run_with_timeout(
        provider,
        executable,
        source_path,
        document,
        scratch,
        TIMEOUT,
    )
}

fn run_with_timeout(
    provider: &Provider,
    executable: &Path,
    source_path: Option<&Path>,
    document: &PieceTable,
    scratch: &Scratch,
    timeout: Duration,
) -> io::Result<File> {
    if document.len() > MAX_INPUT_BYTES {
        return Err(invalid("Formatting is limited to documents up to 2 MiB"));
    }
    let untitled = PathBuf::from(format!(
        "untitled.{}",
        provider
            .extensions
            .first()
            .map(String::as_str)
            .unwrap_or("txt")
    ));
    let filepath = std::path::absolute(source_path.unwrap_or(&untitled))?;
    let mut input = scratch.file("input")?;
    document.visit_chunks(|bytes| input.write_all(bytes))?;
    input.rewind()?;
    let mut output = scratch.file("output")?;
    let mut command = Command::new(executable);
    command
        .args(provider.arguments(&filepath))
        .current_dir(
            filepath
                .parent()
                .ok_or_else(|| invalid("Source path has no parent directory"))?,
        )
        .env("RAYON_NUM_THREADS", "1")
        .env("GOMAXPROCS", "1")
        .stdin(Stdio::from(input))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut running = process::Running::spawn(&mut command).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Could not start {}: {error}", provider.name),
        )
    })?;
    let mut stdout = running.child.stdout.take().expect("piped stdout");
    let mut stderr = running.child.stderr.take().expect("piped stderr");
    process::prepare_pipe(&stdout)?;
    process::prepare_pipe(&stderr)?;

    let started = Instant::now();
    let mut buffer = [0; CHUNK_BYTES];
    let mut output_length = 0;
    let mut error_length = 0;
    let mut diagnostic = Vec::with_capacity(1024);
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    let mut status = None;
    loop {
        if started.elapsed() >= timeout {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{} exceeded the formatting time limit; document preserved",
                    provider.name
                ),
            ));
        }
        let mut progress = false;
        if !stdout_closed && let Some(read) = process::read_pipe(&mut stdout, &mut buffer)? {
            stdout_closed = read == 0;
            progress |= read > 0;
            output_length += read;
            if output_length > MAX_OUTPUT_BYTES {
                return Err(invalid(
                    "Formatter output exceeds 4 MiB; document preserved",
                ));
            }
            output.write_all(&buffer[..read])?;
        }
        if !stderr_closed && let Some(read) = process::read_pipe(&mut stderr, &mut buffer)? {
            stderr_closed = read == 0;
            progress |= read > 0;
            error_length += read;
            if error_length > MAX_ERROR_BYTES {
                return Err(invalid(
                    "Formatter diagnostics exceed 64 KiB; document preserved",
                ));
            }
            let keep = read.min(1024 - diagnostic.len());
            diagnostic.extend_from_slice(&buffer[..keep]);
        }
        if status.is_none() {
            status = running.child.try_wait()?;
        }
        if stdout_closed
            && stderr_closed
            && let Some(status) = status
        {
            if !status.success() {
                let message: String = String::from_utf8_lossy(&diagnostic)
                    .chars()
                    .map(|ch| if ch.is_control() { ' ' } else { ch })
                    .collect();
                return Err(io::Error::other(format!(
                    "{} failed ({status}): {}",
                    provider.name,
                    message.trim()
                )));
            }
            break;
        }
        if !progress {
            // Only the explicitly requested command waits here. Sleeping
            // avoids burning CPU while the formatter computes its output.
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    output.rewind()?;
    Ok(output)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Position {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

/// Map two cursor endpoints by line/column with a fixed-size buffer. Unlike
/// line_count/move_cursor, this does not build a cache of every preceding line.
pub(crate) fn map_positions(
    reader: &mut impl Read,
    targets: [(usize, usize); 2],
) -> io::Result<[Position; 2]> {
    let mut buffer = [0; CHUNK_BYTES];
    let mut current = Position::default();
    let mut mapped = [None; 2];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for &byte in &buffer[..read] {
            if byte == 0 {
                return Err(invalid(
                    "Formatter returned binary output; document preserved",
                ));
            }
            if byte & 0xc0 != 0x80 {
                for (index, &(line, column)) in targets.iter().enumerate() {
                    if mapped[index].is_none()
                        && current.line == line
                        && (current.column >= column || byte == b'\n' || byte == b'\r')
                    {
                        mapped[index] = Some(current);
                    }
                }
                if byte == b'\n' {
                    current.line += 1;
                    current.column = 0;
                } else {
                    current.column += 1;
                }
            }
            current.byte += 1;
            if current.byte > MAX_OUTPUT_BYTES {
                return Err(invalid("Formatter output exceeds the size limit"));
            }
        }
    }
    Ok(mapped.map(|position| position.unwrap_or(current)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_providers_and_custom_configuration() {
        let config = Formatters::parse(crate::embedded_config::FORMATTERS).unwrap();
        assert_eq!(config.providers.len(), 8);
        for (filename, name) in [
            ("file.rs", "rustfmt"),
            ("file.PY", "ruff"),
            ("file.go", "gofmt"),
            ("file.cpp", "clang-format"),
            (".bashrc", "shfmt"),
            ("file.lua", "stylua"),
            ("Cargo.toml", "taplo"),
            ("file.tsx", "biome"),
        ] {
            assert!(
                config
                    .providers
                    .iter()
                    .any(|p| p.name == name && p.matches(Path::new(filename)))
            );
        }
        let custom =
            "[[providers]]\nname = 'custom'\ncommand = 'my-format'\nargs = ['{filepath}']\n";
        assert!(Formatters::parse(custom).is_ok());
        assert!(Formatters::parse(&format!("{custom}{custom}")).is_err());
        assert!(Formatters::parse("[[providers]]\nname = 'bad name'\ncommand = 'x'").is_err());
        assert!(Formatters::parse("[[providers]]\nname = 'ok'\ncommand = ''").is_err());
        assert!(Formatters::parse("provider = []").is_err());
        assert!(config.select(None, None).is_err());
        assert!(
            config
                .select(Some(Path::new("file.unknown")), None)
                .is_err()
        );
        assert!(config.select(None, Some("unknown")).is_err());
    }

    #[test]
    fn filepath_substitution_is_one_literal_argument() {
        let provider = &Formatters::parse(crate::embedded_config::FORMATTERS)
            .unwrap()
            .providers[3];
        let arguments = provider.arguments(Path::new("folder with spaces/$(touch nope); test.cpp"));
        assert_eq!(
            arguments,
            [OsString::from(
                "--assume-filename=folder with spaces/$(touch nope); test.cpp"
            )]
        );
    }

    #[test]
    fn cursor_mapping_handles_unicode_crlf_and_shorter_documents() {
        let mut text = io::Cursor::new("éé\r\nlast\n".as_bytes());
        let positions = map_positions(&mut text, [(0, 99), (1, 2)]).unwrap();
        assert_eq!(
            positions[0],
            Position {
                byte: 4,
                line: 0,
                column: 2
            }
        );
        assert_eq!(
            positions[1],
            Position {
                byte: 8,
                line: 1,
                column: 2
            }
        );
        text.rewind().unwrap();
        assert_eq!(
            map_positions(&mut text, [(10, 0); 2]).unwrap()[0],
            Position {
                byte: 11,
                line: 2,
                column: 0
            }
        );
    }

    #[test]
    fn cursor_mapping_does_not_depend_on_chunk_boundaries() {
        let text = format!("{}é🙂\nend", "a".repeat(CHUNK_BYTES - 1));
        let positions = map_positions(
            &mut io::Cursor::new(text.as_bytes()),
            [(0, CHUNK_BYTES + 1), (1, 2)],
        )
        .unwrap();
        assert_eq!(positions[0].byte, CHUNK_BYTES + 5);
        assert_eq!(positions[1].byte, text.len() - 1);
    }

    #[test]
    fn temporary_files_are_removed_on_drop() {
        let scratch = Scratch::create().unwrap();
        let path = scratch.directory.clone();
        drop(scratch.file("test").unwrap());
        drop(scratch);
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "Manual resource probe: run alone under /usr/bin/time -l"]
    fn resource_probe() {
        let size: usize = std::env::var("POTYI_FORMAT_PROBE_BYTES")
            .unwrap_or_else(|_| MAX_INPUT_BYTES.to_string())
            .parse()
            .unwrap();
        assert!(size <= MAX_INPUT_BYTES);
        let scratch = Scratch::create().unwrap();
        let path = scratch.directory.join("source");
        let mut source = scratch.file("source").unwrap();
        let block = [b'a'; 4096];
        let mut remaining = size;
        while remaining > 0 {
            let amount = remaining.min(block.len());
            source.write_all(&block[..amount]).unwrap();
            remaining -= amount;
        }
        drop(source);
        let mut editor = crate::Editor::new(crate::config::EditorConfig::default()).unwrap();
        editor.document = PieceTable::open(path.to_str().unwrap()).unwrap();
        let mut output = run(
            &shell_provider("tr a b"),
            Path::new("/bin/sh"),
            None,
            &editor.document,
            &scratch,
        )
        .unwrap();
        assert!(editor.apply_formatted(&mut output).unwrap());
        assert_eq!(editor.document.len(), size);
        assert!(editor.document.cached_line_count() <= 1); // Lazy first-line placeholder only.
        assert_eq!(editor.document.byte_at(size - 1).unwrap(), Some(b'b'));
        editor.undo().unwrap();
        assert_eq!(editor.document.byte_at(size - 1).unwrap(), Some(b'a'));
    }

    #[test]
    #[ignore = "Requires a locally installed rustfmt"]
    fn rustfmt_smoke_test_formats_buffer_without_saving() {
        let config = Formatters::parse(crate::embedded_config::FORMATTERS).unwrap();
        let (provider, executable) = config.select(None, Some("rustfmt")).unwrap();
        let scratch = Scratch::create().unwrap();
        let filepath = scratch.directory.join("fixture.rs");
        fs::write(&filepath, "// saved content\n").unwrap();
        let mut editor = crate::Editor::new(crate::config::EditorConfig::default()).unwrap();
        editor
            .document
            .insert(0, "fn main(){println!(\"hello\");}")
            .unwrap();
        let mut output = run(
            provider,
            &executable,
            Some(&filepath),
            &editor.document,
            &scratch,
        )
        .unwrap();
        assert!(editor.apply_formatted(&mut output).unwrap());
        assert_eq!(
            editor.document.text().unwrap(),
            "fn main() {\n    println!(\"hello\");\n}\n"
        );
        assert_eq!(fs::read_to_string(&filepath).unwrap(), "// saved content\n");
        editor.undo().unwrap();
        assert_eq!(
            editor.document.text().unwrap(),
            "fn main(){println!(\"hello\");}"
        );
    }

    #[cfg(unix)]
    fn shell_provider(script: &str) -> Provider {
        Provider {
            name: "fixture".into(),
            command: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            extensions: vec!["txt".into()],
            filenames: vec![],
        }
    }

    #[cfg(unix)]
    #[test]
    fn selection_uses_config_order_and_only_skips_unavailable_executables() {
        let mut missing = shell_provider("");
        missing.name = "missing".into();
        missing.command = "/no/such/formatter".into();
        let config = Formatters {
            providers: vec![missing, shell_provider("cat")],
        };
        assert_eq!(
            config
                .select(Some(Path::new("file.txt")), None)
                .unwrap()
                .0
                .name,
            "fixture"
        );
        assert!(config.select(None, Some("missing")).is_err());
        assert_eq!(
            config.select(None, Some("fixture")).unwrap().0.name,
            "fixture"
        );
    }

    #[cfg(unix)]
    #[test]
    fn process_streams_unsaved_input_and_drains_both_pipes() {
        let mut document = PieceTable::empty().unwrap();
        let text = "éabc\n".repeat(25_000);
        document.insert(0, &text).unwrap();
        let scratch = Scratch::create().unwrap();
        // Diagnostics precede reading stdin, a common pipe deadlock scenario.
        let provider = shell_provider("head -c 32768 /dev/zero >&2; cat");
        let mut output = run(&provider, Path::new("/bin/sh"), None, &document, &scratch).unwrap();
        let mut actual = String::new();
        output.read_to_string(&mut actual).unwrap();
        assert_eq!(actual, text);
        assert_eq!(document.text().unwrap(), text);
    }

    #[cfg(unix)]
    #[test]
    fn process_failure_timeout_and_output_limits_preserve_document() {
        let mut document = PieceTable::empty().unwrap();
        document.insert(0, "original\n").unwrap();
        for (script, expected, timeout) in [
            (
                "printf 'partial'; printf 'syntax problem' >&2; exit 7".to_string(),
                "syntax problem",
                TIMEOUT,
            ),
            (
                "sleep 10".to_string(),
                "time limit",
                Duration::from_millis(100),
            ),
            (
                format!("head -c {} /dev/zero", MAX_OUTPUT_BYTES + 1),
                "4 MiB",
                TIMEOUT,
            ),
            (
                format!("head -c {} /dev/zero >&2", MAX_ERROR_BYTES + 1),
                "64 KiB",
                TIMEOUT,
            ),
        ] {
            let scratch = Scratch::create().unwrap();
            let started = Instant::now();
            let error = run_with_timeout(
                &shell_provider(&script),
                Path::new("/bin/sh"),
                None,
                &document,
                &scratch,
                timeout,
            )
            .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
            assert!(started.elapsed() < TIMEOUT + Duration::from_secs(1));
            assert_eq!(document.text().unwrap(), "original\n");
        }
    }
}
