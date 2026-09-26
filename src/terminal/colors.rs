// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded, streaming highlighting for diagnostics, Git and diff. Output bytes stay unchanged.
use std::{collections::VecDeque, ops::Range, path::Path};

const MAX_SPANS: usize = 8192;
const MAX_PREFIX: usize = 192;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Style {
    Added,
    Removed,
    Header,
    Hunk,
    Warning,
}
impl Style {
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Added => (150, 220, 165),
            Self::Removed => (245, 145, 145),
            Self::Header => (225, 195, 120),
            Self::Hunk => (120, 195, 235),
            Self::Warning => (235, 185, 110),
        }
    }
}
#[derive(Default)]
pub(super) struct Colors {
    spans: VecDeque<(Range<usize>, Style)>,
    enabled: bool,
    git: bool,
    diff: bool,
    section: Option<Style>,
    diagnostic: Option<Style>,
    prefix: Vec<u8>,
    start: usize,
    pending: bool,
}
impl Colors {
    pub fn begin(&mut self, command: &str) {
        self.end();
        let words = super::touch_arguments(command).unwrap_or_default();
        let name = words
            .first()
            .and_then(|s| Path::new(s).file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("");
        self.git = matches!(name, "git" | "git.exe");
        self.diff = self.git || matches!(name, "diff" | "diff.exe");
        self.enabled = true;
    }
    pub fn end(&mut self) {
        self.enabled = false;
        self.section = None;
        self.diagnostic = None;
        self.prefix.clear();
        self.pending = false;
    }
    pub fn append(&mut self, text: &str, offset: usize) {
        if !self.enabled {
            return;
        }
        let mut position = offset;
        for fragment in text.split_inclusive('\n') {
            if !self.pending {
                self.start = position;
                self.pending = true;
            }
            let take = (MAX_PREFIX - self.prefix.len()).min(fragment.len());
            self.prefix.extend_from_slice(&fragment.as_bytes()[..take]);
            position += fragment.len();
            let line = String::from_utf8_lossy(&self.prefix);
            let style = if self.diff {
                classify(&line, self.git, self.section)
            } else {
                diagnostic_style(&line, self.diagnostic)
            };
            // Reclassify a partial prefix (e.g. '+' becoming '+++') without duplicates.
            if self
                .spans
                .back()
                .is_some_and(|(r, _)| r.start == self.start)
            {
                self.spans.pop_back();
            }
            if let Some(style) = style {
                if self.spans.len() == MAX_SPANS {
                    self.spans.pop_front();
                }
                self.spans.push_back((self.start..position, style));
            }
            if fragment.ends_with('\n') {
                // Commit context only at a logical newline, so arbitrary reader
                // fragmentation cannot change how the next line is highlighted.
                if !self.diff {
                    if line.trim().is_empty() {
                        self.diagnostic = None;
                    } else if let Some(style) = diagnostic_heading(&line) {
                        self.diagnostic = Some(style);
                    }
                }
                if self.git {
                    if line.starts_with("Changes to be committed:") {
                        self.section = Some(Style::Added);
                    } else if line.starts_with("Changes not staged for commit:") {
                        self.section = Some(Style::Removed);
                    } else if line.starts_with("Untracked files:") {
                        self.section = Some(Style::Warning);
                    }
                }
                self.prefix.clear();
                self.pending = false;
            }
        }
    }
    pub fn at(&self, byte: usize) -> Option<(u8, u8, u8)> {
        let index = self.spans.partition_point(|(r, _)| r.end <= byte);
        self.spans
            .get(index)
            .filter(|(r, _)| r.contains(&byte))
            .map(|(_, s)| s.rgb())
    }
    pub fn trim(&mut self, bytes: usize) {
        while self.spans.front().is_some_and(|(r, _)| r.end <= bytes) {
            self.spans.pop_front();
        }
        for (r, _) in &mut self.spans {
            r.start = r.start.saturating_sub(bytes);
            r.end -= bytes;
        }
        self.start = self.start.saturating_sub(bytes);
    }
}

fn diagnostic_heading(line: &str) -> Option<Style> {
    let line = line.trim_start().strip_prefix("= ").unwrap_or(line.trim_start());
    for (label, style) in [
        ("error", Style::Removed),
        ("fatal error", Style::Removed),
        ("warning", Style::Warning),
        ("note", Style::Hunk),
        ("help", Style::Added),
    ] {
        if let Some(tail) = line.strip_prefix(label) {
            if tail.starts_with(':') || (tail.starts_with('[') && tail.contains("]:")) {
                return Some(style);
            }
        }
    }
    if line.starts_with("thread '") && line.contains(" panicked at ") {
        return Some(Style::Removed);
    }
    None
}

fn diagnostic_style(line: &str, current: Option<Style>) -> Option<Style> {
    if let Some(style) = diagnostic_heading(line) {
        return Some(style);
    }
    let line = line.trim_start();
    if line.starts_with("--> ") || line.starts_with("::: ") {
        return Some(Style::Hunk);
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
            return Some(Style::Added);
        }
        if gutter.starts_with("- ") {
            return Some(Style::Removed);
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

fn classify(line: &str, git: bool, section: Option<Style>) -> Option<Style> {
    use Style::*;
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
mod tests {
    use super::*;
    #[test]
    fn rust_diagnostics_are_fragment_independent_and_keep_source_neutral() {
        let text = "error[E0308]: mismatched types\n  --> src/東京 file.rs:3:18\n   |\n3  | let value: u8 = \"no\";\n   |                 ^^^^ expected `u8`\n   = note: expected type `u8`\nhelp: replace the value\n3  - let value: u8 = \"no\";\n3  + let value: u8 = 1;\n\nwarning: unused variable\n  --> src/main.rs:5:9\n  |\n5 | let other = 1;\n  |     ^^^^^ unused\n  = help: prefix it with an underscore\n\nordinary output\n";
        let mut whole = Colors::default();
        whole.begin("cargo check");
        whole.append(text, 0);
        let mut split = Colors::default();
        split.begin("env RUSTFLAGS='' cargo +stable check");
        for (offset, ch) in text.char_indices() {
            split.append(&ch.to_string(), offset);
        }
        assert_eq!(whole.spans, split.spans);
        for (needle, style) in [
            ("error[E0308]", Style::Removed),
            ("--> src/東京", Style::Hunk),
            ("^^^^ expected", Style::Removed),
            ("= note:", Style::Hunk),
            ("help: replace", Style::Added),
            ("3  -", Style::Removed),
            ("3  +", Style::Added),
            ("warning:", Style::Warning),
            ("^^^^^ unused", Style::Warning),
            ("= help:", Style::Added),
        ] {
            assert_eq!(split.at(text.find(needle).unwrap()), Some(style.rgb()), "{needle}");
        }
        for needle in ["3  | let", "5 | let", "ordinary output"] {
            assert_eq!(split.at(text.find(needle).unwrap()), None, "{needle}");
        }
        let cut = text.find("warning:").unwrap();
        split.trim(cut);
        assert_eq!(split.at(0), Some(Style::Warning.rgb()));
        split.end();
        split.append("error: not process output\n", text.len() - cut);
        assert_eq!(split.at(text.len() - cut), None);
        split.begin("cargo test");
        split.append("  | ^^^ unrelated\n", text.len() - cut);
        assert_eq!(split.at(text.len() - cut), None);
    }

    #[test]
    fn diagnostic_metadata_is_bounded_and_ordinary_output_stays_plain() {
        let mut colors = Colors::default();
        colors.begin("cargo check");
        let text = format!("error: {}\n", "x".repeat(1_000_000));
        colors.append(&text, 0);
        colors.append(&"warning: problem\n".repeat(MAX_SPANS + 10), text.len());
        assert_eq!(colors.spans.len(), MAX_SPANS);
        assert!(colors.prefix.capacity() <= MAX_PREFIX);
        for line in ["errors: 0", "+ordinary", "---", "warning_count: 0", "let error = 2;"] {
            assert_eq!(diagnostic_style(line, None), None, "{line}");
        }
    }

    #[test]
    fn fragmented_diff_matches_whole_output_and_keeps_history() {
        let text = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+🦀new\n context\n";
        let mut whole = Colors::default();
        whole.begin("git diff");
        whole.append(text, 0);
        let mut split = Colors::default();
        split.begin("git diff");
        for (i, c) in text.char_indices() {
            split.append(&c.to_string(), i);
        }
        assert_eq!(whole.spans, split.spans);
        split.end();
        split.append("+plain\n", text.len());
        assert_eq!(split.at(text.len()), None);
        assert_eq!(
            split.at(text.find("+🦀").unwrap()),
            Some(Style::Added.rgb())
        );
        let cut = text.find("@@").unwrap();
        split.trim(cut);
        assert_eq!(split.at(0), Some(Style::Hunk.rgb()));
    }
    #[test]
    fn oneline_logs_and_graphs_keep_color_across_stream_chunks_and_wrapping() {
        let text = "290a677 lsp updates, multi cursor and other fixes\n* 4e3f6ad (HEAD -> main) version\n| * de14776 Add formatting 🦀 and a long commit message that wraps in a narrow terminal\n|/\n";
        let mut whole = Colors::default();
        whole.begin("git log --oneline --graph");
        whole.append(text, 0);
        let mut split = Colors::default();
        split.begin("git log --oneline --graph");
        for (offset, ch) in text.char_indices() {
            split.append(&ch.to_string(), offset);
        }
        assert_eq!(whole.spans, split.spans);
        for word in ["290a677", "4e3f6ad", "de14776", "narrow terminal"] {
            assert_eq!(
                split.at(text.find(word).unwrap()),
                Some(Style::Header.rgb())
            );
        }
        assert_eq!(split.at(text.find("|/\n").unwrap()), None);
        let cut = text.find("4e3f6ad").unwrap();
        split.trim(cut);
        assert_eq!(split.at(0), Some(Style::Header.rgb()));
        split.end();
        split.append("abcd ordinary output\n", text.len() - cut);
        assert_eq!(split.at(text.len() - cut), None);
        assert_eq!(classify("abcd ordinary diff context", false, None), None);
        for line in [
            "abc too short",
            "abcxyz invalid",
            "abcdef",
            "| * not a commit",
        ] {
            assert!(!compact_commit(line), "{line}");
        }
    }

    #[test]
    fn normal_diff_and_git_status_colors() {
        assert_eq!(classify("1c1\n", false, None), Some(Style::Hunk));
        assert_eq!(classify("< old\n", false, None), Some(Style::Removed));
        assert_eq!(classify("> new\n", false, None), Some(Style::Added));
        let mut c = Colors::default();
        c.begin("git -C project status");
        let text = "Changes to be committed:\n\tmodified: a\nChanges not staged for commit:\n\tmodified: b\nUntracked files:\n\tc\n";
        c.append(text, 0);
        assert_eq!(
            c.at(text.find("\tmodified: a").unwrap()),
            Some(Style::Added.rgb())
        );
        assert_eq!(
            c.at(text.find("\tmodified: b").unwrap()),
            Some(Style::Removed.rgb())
        );
        assert_eq!(c.at(text.find("\tc").unwrap()), Some(Style::Warning.rgb()));
        assert_eq!(classify("M  a", true, None), Some(Style::Added));
        assert_eq!(classify(" M a", true, None), Some(Style::Removed));
    }
    #[test]
    fn huge_lines_and_history_have_fixed_bounds() {
        let mut c = Colors::default();
        c.begin("diff a b");
        c.append(&format!("+{}", "x".repeat(1_000_000)), 0);
        assert_eq!(c.prefix.len(), MAX_PREFIX);
        assert_eq!(c.spans.len(), 1);
        c.append(&"\n+a".repeat(MAX_SPANS + 10), 1_000_001);
        assert!(c.spans.len() <= MAX_SPANS);
        c.begin("printf +ordinary");
        c.append("+ordinary\n", 2_000_000);
        assert_eq!(c.at(2_000_000), None);
    }
}
