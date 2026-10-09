// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Legacy fallback colors and syntactic link underlines. Retained decoration
//! lives in disk-record metadata; only one bounded logical line is held in RAM.
use super::diagnostic_links::CommandContext;
use std::{ops::Range, path::Path};
const PREFIX_BYTES: usize = 192;
const LINE_BYTES: usize = 64 * 1024;
const LINE_ROWS: usize = 256;
const MARKER: &[u8] = b"\x1b]778;potyi-style;";
pub(super) const MAX_STYLE_BYTES: usize = 48;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Tone {
    Added = 1,
    Removed,
    Header,
    Hunk,
    Warning,
}
impl Tone {
    pub(super) fn from_byte(byte: u8) -> Option<Self> {
        Some(match byte {
            1 => Self::Added,
            2 => Self::Removed,
            3 => Self::Header,
            4 => Self::Hunk,
            5 => Self::Warning,
            _ => return None,
        })
    }
    pub(super) fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Added => (150, 220, 165),
            Self::Removed => (245, 145, 145),
            Self::Header => (225, 195, 120),
            Self::Hunk => (120, 195, 235),
            Self::Warning => (235, 185, 110),
        }
    }
}
pub(super) fn formatted(
    tone: Option<Tone>,
    underline: Option<Range<usize>>,
    bytes: &[u8],
) -> Vec<u8> {
    if tone.is_none() && underline.is_none() {
        return bytes.to_vec();
    }
    let range = underline.unwrap_or(0..0);
    let mut result = Vec::with_capacity(MAX_STYLE_BYTES + bytes.len());
    result.extend_from_slice(MARKER);
    result.extend_from_slice(
        format!(
            "{};{};{}",
            tone.map_or(0, |tone| tone as u8),
            range.start,
            range.end
        )
        .as_bytes(),
    );
    result.push(7);
    result.extend_from_slice(bytes);
    result
}
pub(super) fn stored_style(bytes: &[u8]) -> (Option<Tone>, Option<Range<usize>>) {
    let Some(suffix) = bytes.strip_prefix(MARKER) else {
        return (None, None);
    };
    let Some(end) = suffix
        .iter()
        .take(MAX_STYLE_BYTES)
        .position(|byte| *byte == 7)
    else {
        return (None, None);
    };
    let Ok(value) = std::str::from_utf8(&suffix[..end]) else {
        return (None, None);
    };
    let mut parts = value.split(';');
    let tone = parts
        .next()
        .and_then(|value| value.parse().ok())
        .and_then(Tone::from_byte);
    let start = parts.next().and_then(|value| value.parse::<usize>().ok());
    let end = parts.next().and_then(|value| value.parse::<usize>().ok());
    let underline = start
        .zip(end)
        .filter(|(start, end)| start < end && *end <= LINE_BYTES)
        .map(|(start, end)| start..end);
    (tone, underline)
}
/// Plain records carry only our private decoration prefix, followed by UTF-8.
pub(super) fn plain_text(bytes: &[u8]) -> std::io::Result<&str> {
    let bytes = if let Some(suffix) = bytes.strip_prefix(MARKER) {
        let end = suffix
            .iter()
            .take(MAX_STYLE_BYTES)
            .position(|byte| *byte == 7)
            .ok_or_else(|| std::io::Error::other("Invalid plain style metadata"))?;
        &suffix[end + 1..]
    } else {
        bytes
    };
    std::str::from_utf8(bytes).map_err(std::io::Error::other)
}
#[derive(Clone)]
struct Row {
    id: u64,
    start: usize,
    len: usize,
}
#[derive(Clone)]
pub(super) struct Colors {
    source: u64,
    git: bool,
    diff: bool,
    section: Option<Tone>,
    diagnostic: Option<Tone>,
    context: CommandContext,
    prefix: Vec<u8>,
    line: String,
    rows: Vec<Row>,
    bytes: usize,
    in_line: bool,
    skipped: bool,
    tone: Option<Tone>,
}
impl Default for Colors {
    fn default() -> Self {
        Self {
            source: 0,
            git: false,
            diff: false,
            section: None,
            diagnostic: None,
            context: CommandContext::new(""),
            prefix: Vec::new(),
            line: String::new(),
            rows: Vec::new(),
            bytes: 0,
            in_line: false,
            skipped: false,
            tone: None,
        }
    }
}
pub(super) struct Decoration {
    pub(super) tone: Option<Tone>,
    pub(super) underline: Option<Range<usize>>,
}
pub(super) struct Change {
    pub(super) current: Decoration,
    pub(super) previous: Vec<(u64, Decoration)>,
}
fn intersect(range: &Range<usize>, start: usize, len: usize) -> Option<Range<usize>> {
    let left = range.start.max(start);
    let right = range.end.min(start.saturating_add(len));
    (left < right).then(|| left - start..right - start)
}
impl Colors {
    pub(super) fn begin(&mut self, source: u64, command: &str) {
        *self = Self::default();
        self.source = source;
        self.context = CommandContext::new(command);
        let words = super::browser::words(command).unwrap_or_default();
        let name = words
            .first()
            .and_then(|word| Path::new(word).file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("");
        self.git = matches!(name, "git" | "git.exe");
        self.diff = self.git || matches!(name, "diff" | "diff.exe");
    }
    // Clear keeps the active command family, but forgets its old line/context.
    pub(super) fn clear(&mut self) {
        self.section = None;
        self.diagnostic = None;
        self.prefix.clear();
        self.line.clear();
        self.rows.clear();
        self.bytes = 0;
        self.in_line = false;
        self.skipped = false;
        self.tone = None;
    }
    pub(super) fn current_command_context(&self, source: u64) -> Option<CommandContext> {
        (source != 0 && source == self.source).then(|| self.context.clone())
    }
    pub(super) fn preview(&self, source: u64) -> Self {
        if source != self.source {
            let mut next = Self::default();
            next.source = source;
            return next;
        }
        self.clone()
    }
    pub(super) fn row(
        &mut self,
        source: u64,
        text: &str,
        wrapped: bool,
        id: Option<u64>,
    ) -> Change {
        if source != self.source {
            *self = Self::default();
            self.source = source;
        }
        if !self.in_line {
            self.prefix.clear();
            self.line.clear();
            self.rows.clear();
            self.bytes = 0;
            self.skipped = false;
            self.tone = None;
        }
        let start = self.bytes;
        self.bytes = self.bytes.saturating_add(text.len());
        let take = (PREFIX_BYTES - self.prefix.len()).min(text.len());
        self.prefix.extend_from_slice(&text.as_bytes()[..take]);
        if self.bytes > LINE_BYTES || self.rows.len() >= LINE_ROWS {
            self.skipped = true;
            self.line.clear();
        }
        if !self.skipped {
            self.line.push_str(text);
        }
        let prefix = String::from_utf8_lossy(&self.prefix);
        let tone = if self.diff {
            classify(&prefix, self.git, self.section)
        } else {
            diagnostic_style(&prefix, self.diagnostic)
        };
        let underline = if !wrapped && !self.skipped {
            self.context.underline(&self.line)
        } else {
            None
        };
        let previous = if tone != self.tone || underline.is_some() {
            self.rows
                .iter()
                .map(|row| {
                    (
                        row.id,
                        Decoration {
                            tone,
                            underline: underline
                                .as_ref()
                                .and_then(|range| intersect(range, row.start, row.len)),
                        },
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        self.tone = tone;
        if self.rows.len() < LINE_ROWS
            && let Some(id) = id
        {
            self.rows.push(Row {
                id,
                start,
                len: text.len(),
            });
        }
        if !wrapped {
            if !self.diff {
                if prefix.trim().is_empty() {
                    self.diagnostic = None;
                } else if let Some(tone) = diagnostic_heading(&prefix) {
                    self.diagnostic = Some(tone);
                }
            }
            if self.git {
                if prefix.starts_with("Changes to be committed:") {
                    self.section = Some(Tone::Added);
                } else if prefix.starts_with("Changes not staged for commit:") {
                    self.section = Some(Tone::Removed);
                } else if prefix.starts_with("Untracked files:") {
                    self.section = Some(Tone::Warning);
                }
            }
        }
        self.in_line = wrapped;
        Change {
            current: Decoration {
                tone,
                underline: underline
                    .as_ref()
                    .and_then(|range| intersect(range, start, text.len())),
            },
            previous,
        }
    }
    #[cfg(test)]
    fn storage(&self) -> (usize, usize, usize) {
        (self.prefix.len(), self.line.len(), self.rows.len())
    }
}

fn diagnostic_heading(line: &str) -> Option<Tone> {
    let line = line
        .trim_start()
        .strip_prefix("= ")
        .unwrap_or(line.trim_start());
    for (label, style) in [
        ("error", Tone::Removed),
        ("fatal error", Tone::Removed),
        ("warning", Tone::Warning),
        ("note", Tone::Hunk),
        ("help", Tone::Added),
    ] {
        if let Some(tail) = line.strip_prefix(label) {
            if tail.starts_with(':') || (tail.starts_with('[') && tail.contains("]:")) {
                return Some(style);
            }
        }
    }
    if line.starts_with("thread '") && line.contains(" panicked at ") {
        return Some(Tone::Removed);
    }
    None
}

fn diagnostic_style(line: &str, current: Option<Tone>) -> Option<Tone> {
    if let Some(style) = diagnostic_heading(line) {
        return Some(style);
    }
    let line = line.trim_start();
    if line.starts_with("--> ") || line.starts_with("::: ") {
        return Some(Tone::Hunk);
    }
    // Leave the code excerpt neutral; emphasize only rustc's annotation rows.
    if let Some(annotation) = line.strip_prefix('|') {
        let annotation = annotation.trim_start();
        if annotation.starts_with(['^', '-'])
            || (annotation.starts_with('_') && annotation.contains('^'))
        {
            return current;
        }
    }
    // rustc's replacement suggestions use a numbered +/- gutter.
    if current.is_some() {
        let gutter = line.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ');
        if gutter.starts_with("+ ") {
            return Some(Tone::Added);
        }
        if gutter.starts_with("- ") {
            return Some(Tone::Removed);
        }
    }
    None
}
// Git's compact log format omits the "commit" keyword. Allow graph prefixes
// as well, but require a complete hash token so fragmented reads stay stable.
fn compact_commit(line: &str) -> bool {
    let line = if line
        .trim_start_matches(' ')
        .starts_with(['|', '*', '/', '\\'])
    {
        line.trim_start_matches([' ', '|', '*', '/', '\\', '_', '.'])
    } else {
        line
    };
    let bytes = line.as_bytes();
    let hash_len = bytes.iter().take_while(|b| b.is_ascii_hexdigit()).count();
    (4..=64).contains(&hash_len) && bytes.get(hash_len) == Some(&b' ')
}

/// Native Git commands use the same prefix-limited legacy classifier as
/// ordinary command output; linked hashes are styled separately by Layout.
pub(super) fn git_line(line: &str, section: &mut Option<Tone>) -> Option<(u8, u8, u8)> {
    let prefix = String::from_utf8_lossy(&line.as_bytes()[..line.len().min(PREFIX_BYTES)]);
    let tone = classify(&prefix, true, *section);
    if prefix.starts_with("Changes to be committed:") {
        *section = Some(Tone::Added);
    } else if prefix.starts_with("Changes not staged for commit:") {
        *section = Some(Tone::Removed);
    } else if prefix.starts_with("Untracked files:") {
        *section = Some(Tone::Warning);
    }
    tone.map(Tone::rgb)
}

fn classify(line: &str, git: bool, section: Option<Tone>) -> Option<Tone> {
    use Tone::*;
    if (git && compact_commit(line))
        || line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("---")
        || line.starts_with("+++")
        || line.starts_with("commit ")
        || line.starts_with("From ")
        || line.starts_with("rename from ")
        || line.starts_with("rename to ")
    {
        return Some(Header);
    }
    if line.starts_with("@@") {
        return Some(Hunk);
    }
    if line.starts_with('+') || line.starts_with("> ") {
        return Some(Added);
    }
    if line.starts_with('-') || line.starts_with("< ") {
        return Some(Removed);
    }
    if git {
        if line.starts_with("fatal:") || line.starts_with("error:") {
            return Some(Removed);
        }
        if line.starts_with("warning:") || line.starts_with("?? ") {
            return Some(Warning);
        }
        if line.starts_with("On branch ")
            || line.starts_with("Changes ")
            || line.starts_with("Untracked files:")
        {
            return Some(Header);
        }
        if line.starts_with('\t') {
            return section;
        }
        // Git status --short / porcelain v1: index status, working-tree status.
        let b = line.as_bytes();
        let status = |v| matches!(v, b' ' | b'M' | b'A' | b'D' | b'R' | b'C' | b'U' | b'T');
        if b.len() >= 4
            && b[2] == b' '
            && status(b[0])
            && status(b[1])
            && (b[0] != b' ' || b[1] != b' ')
        {
            return Some(if b[0] == b'U' || b[1] == b'U' {
                Warning
            } else if b[1] != b' ' {
                Removed
            } else {
                Added
            });
        }
    } else {
        let s = line.trim_end();
        if s.bytes().next().is_some_and(|b| b.is_ascii_digit())
            && s.bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b',' | b'a' | b'c' | b'd'))
            && s.contains(['a', 'c', 'd'])
        {
            return Some(Hunk);
        }
    }
    None
}

#[cfg(test)]
#[path = "output_colors_tests.rs"]
mod tests;
