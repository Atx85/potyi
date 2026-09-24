// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{EntryKind, OutputEntry, SourceLocation, has_shell_operators, touch_arguments};
use std::path::{Path, PathBuf};

const MAX_PENDING: usize = 64 * 1024;

#[derive(Default)]
pub(super) struct Scanner {
    context: Option<Context>,
    pending: String,
    start: usize,
    skipped: bool,
}

struct Context {
    git: Option<std::sync::Arc<super::git::Repository>>,
    cwd: PathBuf,
    single_file: Option<PathBuf>,
    columns: bool,
    byte_columns: bool,
    line_numbers: bool,
}

impl Scanner {
    pub fn new(command: &str, cwd: &Path) -> Self {
        Self {
            context: Some(Context::new(command, cwd)),
            ..Self::default()
        }
    }

    pub fn clear_pending(&mut self) {
        self.pending.clear();
        self.start = 0;
        self.skipped = false;
    }

    pub fn trim(&mut self, bytes: usize) {
        if self.start < bytes && !self.pending.is_empty() {
            self.pending.clear();
            self.skipped = true;
        }
        self.start = self.start.saturating_sub(bytes);
    }

    /// Parse each completed logical line once, independently of reader chunks
    /// and visual wrapping. Huge unterminated lines cannot grow the parser buffer.
    pub fn append(&mut self, text: &str, offset: usize, entries: &mut Vec<OutputEntry>) {
        let Some(context) = &self.context else {
            return;
        };
        let mut position = offset;
        for fragment in text.split_inclusive('\n') {
            if self.pending.is_empty() && !self.skipped {
                self.start = position;
            }
            if !self.skipped {
                if self.pending.len() + fragment.len() <= MAX_PENDING {
                    self.pending.push_str(fragment);
                } else {
                    self.pending.clear();
                    self.skipped = true;
                }
            }
            position += fragment.len();
            if fragment.ends_with('\n') {
                if !self.skipped {
                    let line = self.pending.trim_end_matches(['\n', '\r']);
                    if let Some(repo) = &context.git {
                        if entries.len() < super::git::MAX_LINKS {
                            if let Some(range) = super::git::hash_range(line) {
                                entries.push(OutputEntry {
                                    git_status: None,
                                    commit: Some(super::git::Commit {hash:line[range.clone()].into(),repo:repo.clone()}),
                                    range:self.start + range.start..self.start + range.end,
                                    path:context.cwd.clone(), kind:EntryKind::Commit, location:None,
                                });
                            }
                        }
                    } else if let Some((path, location)) = context.parse(line) {
                        entries.push(OutputEntry {
                            range: self.start..self.start + line.len(),
                            path,
                            kind: EntryKind::Text,
                            location: Some(location),
                            commit: None,
                            git_status: None,
                        });
                    }
                }
                self.pending.clear();
                self.skipped = false;
            }
        }
    }
}

impl Context {
    fn new(command: &str, cwd: &Path) -> Self {
        let mut context = Self {
            git: super::git::Repository::from_log(command, cwd),
            cwd: cwd.into(),
            single_file: None,
            columns: true,
            byte_columns: false,
            line_numbers: true,
        };
        let Ok(words) = touch_arguments(command) else {
            return context;
        };
        let Some(executable) = words
            .first()
            .and_then(|word| Path::new(word).file_name())
            .and_then(|name| name.to_str())
        else {
            return context;
        };
        let grep = matches!(executable, "grep" | "egrep" | "fgrep" | "grep.exe");
        let rg = matches!(executable, "rg" | "rg.exe");
        if !grep && !rg {
            return context;
        }
        context.columns = rg
            && words
                .iter()
                .any(|word| word == "--column" || word == "--vimgrep");
        context.byte_columns = rg;
        context.line_numbers = context.columns
            || words.iter().skip(1).any(|word| {
                word == "--line-number"
                    || (word.starts_with('-') && !word.starts_with("--") && word[1..].contains('n'))
            });
        // Shell expansions/pipelines can change the input. Only infer an omitted
        // filename for an unambiguous standalone search of one literal file.
        if has_shell_operators(command) {
            return context;
        }
        let mut pattern = false;
        let mut options = true;
        let mut files = Vec::new();
        let mut index = 1;
        while index < words.len() {
            let word = &words[index];
            index += 1;
            if options && word == "--" {
                options = false;
                continue;
            }
            if options && word.starts_with("--") {
                let (name, value) = word
                    .split_once('=')
                    .map_or((word.as_str(), false), |(name, _)| (name, true));
                let takes_value = match name {
                    "--regexp" | "--file" => {
                        pattern = true;
                        true
                    }
                    "--after-context" | "--before-context" | "--context" | "--max-count"
                    | "--include" | "--exclude" | "--exclude-dir" | "--exclude-from"
                    | "--binary-files" | "--devices" | "--directories" | "--label" | "--glob"
                    | "--iglob" | "--type" | "--type-not" | "--encoding" => true,
                    "--line-number"
                    | "--with-filename"
                    | "--no-filename"
                    | "--column"
                    | "--no-heading"
                    | "--heading"
                    | "--hidden"
                    | "--ignore-case"
                    | "--fixed-strings"
                    | "--word-regexp"
                    | "--line-regexp"
                    | "--invert-match"
                    | "--recursive"
                    | "--dereference-recursive"
                    | "--text"
                    | "--no-messages" => false,
                    _ if name == "--color" || name == "--colour" => false,
                    _ => return context,
                };
                if takes_value && !value {
                    index += 1;
                }
            } else if options && word.starts_with('-') && word != "-" {
                for (offset, flag) in word[1..].char_indices() {
                    if matches!(flag, 'e' | 'f') {
                        pattern = true;
                    }
                    if matches!(
                        flag,
                        'e' | 'f' | 'A' | 'B' | 'C' | 'm' | 'g' | 't' | 'T' | 'd' | 'D'
                    ) {
                        if offset + 2 == word.len() {
                            index += 1;
                        }
                        break;
                    }
                    if !"nHhriIvFwxsEaUuRbclLqozZPS".contains(flag) {
                        return context;
                    }
                }
            } else if !pattern {
                pattern = true;
            } else {
                files.push(word);
            }
        }
        if let [file] = files.as_slice() {
            // Expansion cannot safely identify a single target without the shell.
            if *file != "-"
                && !file.contains(['*', '?', '[', '$', '`', '~'])
                && !cwd.join(file).is_dir()
            {
                context.single_file = Some(cwd.join(file));
            }
        }
        context
    }

    fn parse(&self, text: &str) -> Option<(PathBuf, SourceLocation)> {
        if !self.line_numbers {
            return None;
        }
        // A leading line number is grep/rg's single-file output format.
        if let Some((line, tail)) = number_field(text) {
            if let Some(path) = &self.single_file {
                let column = if self.columns {
                    column_field(tail)
                } else {
                    None
                };
                return Some((
                    path.clone(),
                    SourceLocation {
                        line,
                        column,
                        byte_column: self.byte_columns,
                    },
                ));
            }
            return None;
        }
        // Scan separators from the left: filenames can contain spaces and drive
        // letters, and the matching source text may itself contain colons.
        for (index, _) in text.match_indices(':') {
            let path = &text[..index];
            if path.is_empty() || path.trim() != path {
                continue;
            }
            if let Some((line, tail)) = number_field(&text[index + 1..]) {
                let column = if self.columns {
                    column_field(tail)
                } else {
                    None
                };
                return Some((
                    self.cwd.join(path),
                    SourceLocation {
                        line,
                        column,
                        byte_column: self.byte_columns,
                    },
                ));
            }
        }
        None
    }
}

fn column_field(text: &str) -> Option<usize> {
    number_field(text).map(|(column, _)| column).or_else(|| {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        text.parse::<usize>().ok().filter(|column| *column > 0)
    })
}

fn number_field(text: &str) -> Option<(usize, &str)> {
    let (number, rest) = text.split_once(':')?;
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number = number.parse::<usize>().ok()?;
    (number > 0).then_some((number, rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Terminal, TerminalAction};

    fn scan(command: &str, output: &str) -> Vec<OutputEntry> {
        let mut scanner = Scanner::new(command, Path::new("/project"));
        let mut entries = Vec::new();
        // Small fragments exercise line and Unicode boundaries in cleaned output.
        for (offset, ch) in output.char_indices() {
            scanner.append(&ch.to_string(), offset, &mut entries);
        }
        entries
    }

    #[test]
    fn single_file_grep_remembers_quoted_filename_and_does_not_invent_columns() {
        for command in [
            "grep -n 'fn main' 'src/my main.rs'",
            "grep -in -e 'fn main' -- 'src/my main.rs'",
            "grep --line-number --regexp='fn main' 'src/my main.rs'",
            "grep -n -f patterns 'src/my main.rs'",
        ] {
            let links = scan(command, "42:17:fn main() {\n");
            assert_eq!(links.len(), 1, "{command}");
            assert_eq!(links[0].path, Path::new("/project/src/my main.rs"));
            assert_eq!(
                links[0].location.unwrap(),
                SourceLocation {
                    line: 42,
                    column: None,
                    byte_column: false
                }
            );
        }
    }

    #[test]
    fn filename_results_support_spaces_colons_unicode_and_column_output() {
        let links = scan(
            "grep -rn main src",
            "src/my 東京.rs:42:fn main() {\nsrc/other.rs:9:32:literal text\n",
        );
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].path, Path::new("/project/src/my 東京.rs"));
        assert_eq!(links[1].location.unwrap().column, None);
        let links = scan(
            "rg --column main src",
            "src/my 東京.rs:42:7:  fn main() {\n",
        );
        assert_eq!(
            links[0].location.unwrap(),
            SourceLocation {
                line: 42,
                column: Some(7),
                byte_column: true
            }
        );
        let links = scan(
            "rg --vimgrep main src",
            "src/my 東京.rs:42:7:  fn main() {\n",
        );
        assert_eq!(links[0].location.unwrap().column, Some(7));
        let links = scan("custom-search", "C:\\work\\main.rs:42:7:result\n");
        assert!(
            links[0]
                .path
                .to_string_lossy()
                .ends_with("C:\\work\\main.rs")
        );
        assert_eq!(links[0].location.unwrap().column, Some(7));
    }

    #[test]
    fn does_not_guess_files_for_stdin_pipelines_or_ambiguous_arguments() {
        assert!(scan("grep pattern file", "12:numeric source text\n").is_empty());
        assert!(scan("grep -H pattern file", "file:12:numeric source text\n").is_empty());
        for command in [
            "grep -n pattern",
            "grep -n pattern -",
            "grep -n pattern *.rs",
            "grep -n pattern a b",
            "cat a | grep -n pattern",
            "cd src && grep -n main main.rs",
            "grep --unknown value pattern file",
        ] {
            assert!(scan(command, "12:match\n").is_empty(), "{command}");
        }
        assert!(scan("grep -n main file", "0:match\nno matches\n--\n").is_empty());
    }

    #[test]
    fn scanner_bounds_unfinished_lines_and_recovers_at_next_newline() {
        let mut scanner = Scanner::new("grep -n main file", Path::new("/project"));
        let mut entries = Vec::new();
        scanner.append(&"x".repeat(MAX_PENDING + 1), 0, &mut entries);
        assert!(scanner.pending.len() <= MAX_PENDING);
        scanner.append("\n3:match\n", MAX_PENDING + 1, &mut entries);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].range.start, MAX_PENDING + 2);
    }

    #[test]
    fn links_remain_clickable_after_cd_wrapping_and_trimming() {
        use crate::terminal_layout::{TerminalLayout, WrapMetrics};
        let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        terminal.location_scanner = Scanner::new("grep -n main 'my file.rs'", &terminal.cwd);
        terminal
            .append_text("42:fn main() { /* 東京: extra long matching source text */ }\n")
            .unwrap();
        let expected = terminal.entries[0].action();
        terminal.cwd = terminal.cwd.join("elsewhere");
        let mut layout = TerminalLayout::default();
        layout
            .update(
                &terminal.output,
                terminal.output_generation(),
                WrapMetrics {
                    width: 8,
                    cell_width: 1,
                    tab_width: 4,
                    font_size: 18,
                },
                |_| 1,
            )
            .unwrap();
        assert!(layout.len() > 3);
        for row in 0..layout.len() - 1 {
            let range = layout.row_range(row, &terminal.output).unwrap().unwrap();
            assert_eq!(
                terminal.action_at_output_offset(range.start).unwrap(),
                expected
            );
        }
        terminal
            .begin_output_drag(0, 0, 0, expected.clone(), false)
            .unwrap();
        terminal.drag_output_to(3, 10, 0).unwrap();
        assert_eq!(terminal.finish_output_drag(), None);
        assert_eq!(terminal.output_selection(), 0..3);
        terminal.location_scanner = Scanner::default();
        terminal
            .append_text(&"x\n".repeat(super::super::MAX_OUTPUT_BYTES / 2))
            .unwrap();
        assert!(terminal.entries.is_empty());
        assert!(matches!(expected, Some(TerminalAction::Location(_, _))));
    }
}
