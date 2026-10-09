// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded, command-aware search and compiler diagnostic semantics.
use super::super::browser;
use std::{ops::Range, path::Path};

#[derive(Clone, Default)]
pub(super) struct Context {
    single_file: Option<String>,
    search: bool,
    line_numbers: bool,
    columns: bool,
    byte_columns: bool,
}
pub(super) struct Location {
    pub(super) range: Range<usize>,
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) column: Option<usize>,
    pub(super) byte_column: bool,
    pub(super) compiler: bool,
}
impl Context {
    pub(super) fn new(command: &str) -> Self {
        let mut context = Self::default();
        if command.len() > 64 * 1024 {
            return context;
        }
        let Ok(words) = browser::words(command) else {
            return context;
        };
        let Some(executable) = words
            .first()
            .and_then(|word| Path::new(word).file_name())
            .and_then(|name| name.to_str())
        else {
            return context;
        };
        let rg = matches!(executable, "rg" | "rg.exe");
        if !rg && !matches!(executable, "grep" | "egrep" | "fgrep" | "grep.exe") {
            return context;
        }
        context.search = true;
        context.byte_columns = rg;
        context.columns = rg
            && words
                .iter()
                .any(|word| matches!(word.as_str(), "--column" | "--vimgrep"));
        context.line_numbers = context.columns
            || words.iter().skip(1).any(|word| {
                word == "--line-number"
                    || (word.starts_with('-') && !word.starts_with("--") && word[1..].contains('n'))
            });
        if browser::shell_operators(command) {
            return context;
        }
        let mut pattern = false;
        let mut options = true;
        let mut file = None;
        let mut multiple_files = false;
        let mut index = 1;
        while index < words.len() {
            let word = &words[index];
            index += 1;
            if options && word == "--" {
                options = false;
                continue;
            }
            if options && word.starts_with("--") {
                let (name, attached) = word
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
                    | "--vimgrep"
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
                    | "--no-messages"
                    | "--color"
                    | "--colour" => false,
                    _ => return context,
                };
                if takes_value && !attached {
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
                if file.is_some() {
                    multiple_files = true;
                } else {
                    file = Some(word);
                }
            }
        }
        if let Some(file) = file
            && !multiple_files
            && file != "-"
            && !file.contains(['*', '?', '[', '$', '`', '~'])
        {
            context.single_file = Some(file.clone());
        }
        context
    }
    pub(super) fn parse(&self, text: &str) -> Option<Location> {
        if text.len() > 64 * 1024 {
            return None;
        }
        if let Some(location) = rust_location(text) {
            return Some(location);
        }
        if !self.search || !self.line_numbers {
            return None;
        }
        if let Some((line, tail)) = number_field(text) {
            let path = self.single_file.clone()?;
            return Some(Location {
                range: 0..text.len(),
                path,
                line,
                column: self.columns.then(|| column_field(tail)).flatten(),
                byte_column: self.byte_columns,
                compiler: false,
            });
        }
        for (index, _) in text.match_indices(':') {
            let path = &text[..index];
            if path.is_empty() || path.trim() != path {
                continue;
            }
            if let Some((line, tail)) = number_field(&text[index + 1..]) {
                return Some(Location {
                    range: 0..text.len(),
                    path: path.into(),
                    line,
                    column: self.columns.then(|| column_field(tail)).flatten(),
                    byte_column: self.byte_columns,
                    compiler: false,
                });
            }
        }
        None
    }
    pub(super) fn underline(&self, text: &str) -> Option<Range<usize>> {
        if text.len() > 64 * 1024 {
            return None;
        }
        if let Some(location) = self.parse(text) {
            return Some(location.range);
        }
        if self.search || rust_marker(text) {
            return None;
        }
        // :term's default command context classifies unprefixed numeric
        // locations syntactically. Activation still applies stricter path,
        // click-range and filesystem checks in the bounded lookup worker.
        text.match_indices(':').find_map(|(index, _)| {
            let path = &text[..index];
            (!path.is_empty() && path.trim() == path)
                .then(|| number_field(&text[index + 1..]))
                .flatten()
                .map(|_| 0..text.len())
        })
    }
}
pub(super) fn rust_marker(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with("--> ")
        || text.starts_with("::: ")
        || (text.starts_with("thread '") && text.contains(" panicked at "))
}
fn positive(text: &str) -> Option<usize> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|number| *number > 0)
}
fn number_field(text: &str) -> Option<(usize, &str)> {
    let (number, rest) = text.split_once(':')?;
    Some((positive(number)?, rest))
}
fn column_field(text: &str) -> Option<usize> {
    number_field(text)
        .map(|(column, _)| column)
        .or_else(|| positive(text))
}
fn rust_location(text: &str) -> Option<Location> {
    let trimmed = text.trim_start();
    let location = trimmed
        .strip_prefix("--> ")
        .or_else(|| trimmed.strip_prefix("::: "))
        .or_else(|| {
            trimmed
                .starts_with("thread '")
                .then(|| trimmed.split_once(" panicked at "))
                .flatten()
                .map(|(_, tail)| tail)
        })?
        .trim_start();
    let start = text.len() - location.len();
    let location = location.trim_end().trim_end_matches(':');
    let (path_line, column) = location.rsplit_once(':')?;
    let column = positive(column)?;
    let (path, line) = path_line.rsplit_once(':')?;
    let line = positive(line)?;
    if path.is_empty() || path.starts_with('<') {
        return None;
    }
    Some(Location {
        range: start..start + location.len(),
        path: path.into(),
        line,
        column: Some(column),
        byte_column: false,
        compiler: true,
    })
}
