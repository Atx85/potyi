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

use std::ops::Range;
use std::path::{Component, Path, PathBuf};

use super::{EntryKind, OutputEntry, expand_home, home_directory, split_command};

const MAX_ENTRIES: usize = 4096;
const MAX_PATH_BYTES: usize = 256 * 1024;

#[derive(Default)]
pub(super) struct Cache {
    entries: Vec<(PathBuf, bool)>,
    bytes: usize,
    cycle: Option<Cycle>,
}

struct Cycle {
    before: String,
    after: String,
    candidates: Vec<String>,
    selected: usize,
}

impl Cache {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn reset_cycle(&mut self) {
        self.cycle = None;
    }

    pub fn extend(&mut self, entries: &[OutputEntry]) {
        self.reset_cycle();
        for entry in entries {
            let bytes = entry.path.as_os_str().len();
            if self.entries.len() >= MAX_ENTRIES || self.bytes + bytes > MAX_PATH_BYTES {
                break;
            }
            self.entries.push((
                entry.path.components().collect(),
                entry.kind == EntryKind::Directory,
            ));
            self.bytes += bytes;
        }
    }

    pub fn complete(
        &mut self,
        input: &mut String,
        cursor: &mut usize,
        cwd: &Path,
        backwards: bool,
    ) -> Option<String> {
        if let Some(cycle) = &mut self.cycle {
            cycle.selected = if backwards {
                cycle
                    .selected
                    .checked_sub(1)
                    .unwrap_or(cycle.candidates.len() - 1)
            } else {
                (cycle.selected + 1) % cycle.candidates.len()
            };
        } else {
            let (range, prefix) = word_at(input, *cursor)?;
            // Complete filenames, not command names or command separators.
            let before = input[..range.start].trim_end();
            if ((range.start == 0 && range.end < input.len()) || before.ends_with(['|', '&', ';']))
                && !prefix.contains('/')
                && !prefix.contains('\\')
            {
                return None;
            }
            if self.entries.is_empty() {
                return Some("Run ls to load filename suggestions".into());
            }
            let (command, _) = split_command(input);
            let built_in =
                !super::has_shell_operators(input) && matches!(command, "edit" | "view" | "cd");
            let mut candidates = Vec::new();
            for (path, directory) in &self.entries {
                if command == "cd" && !directory {
                    continue;
                }
                let Some(mut name) = candidate_path(path, cwd, &prefix) else {
                    continue;
                };
                if name.starts_with('-') {
                    name.insert_str(0, "./");
                }
                if *directory && !name.ends_with('/') {
                    name.push('/');
                }
                if !name.starts_with(&prefix) || name.chars().any(char::is_control) {
                    continue;
                }
                // Quoted ~ would be literal in a shell, so emit its full path.
                if name.starts_with("~/") {
                    name = expand_home(&name).to_string_lossy().into_owned();
                }
                if let Some(quoted) = quote_path(&name, built_in) {
                    candidates.push(quoted);
                }
            }
            candidates.sort();
            candidates.dedup();
            if candidates.is_empty() {
                return Some("No matching names in the latest listing".into());
            }
            let selected = if backwards { candidates.len() - 1 } else { 0 };
            self.cycle = Some(Cycle {
                before: input[..range.start].to_string(),
                after: input[range.end..].to_string(),
                candidates,
                selected,
            });
        }
        let cycle = self.cycle.as_ref().unwrap();
        input.clear();
        input.push_str(&cycle.before);
        input.push_str(&cycle.candidates[cycle.selected]);
        *cursor = input.len();
        input.push_str(&cycle.after);
        Some(format!(
            "Match {} of {} · Tab / Shift+Tab cycles",
            cycle.selected + 1,
            cycle.candidates.len()
        ))
    }
}

fn candidate_path(path: &Path, cwd: &Path, prefix: &str) -> Option<String> {
    if prefix.starts_with("~/") || prefix.starts_with("~\\") {
        return Some(format!(
            "~/{}",
            path.strip_prefix(home_directory()?).ok()?.to_str()?
        ));
    }
    if Path::new(prefix).is_absolute() {
        return Some(path.to_str()?.to_string());
    }
    let from: Vec<_> = cwd.components().collect();
    let to: Vec<_> = path.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return Some(path.to_str()?.to_string());
    }
    let mut relative = PathBuf::new();
    for component in &from[common..] {
        if matches!(component, Component::Normal(_)) {
            relative.push("..");
        }
    }
    for component in &to[common..] {
        relative.push(component.as_os_str());
    }
    let name = relative.to_str()?;
    let name = if name.is_empty() { "." } else { name };
    // Use the separator already typed by the user on Windows.
    let name = if cfg!(windows) && !prefix.contains('\\') {
        name.replace('\\', "/")
    } else {
        name.to_string()
    };
    Some(if prefix.starts_with("./") {
        format!("./{name}")
    } else {
        name
    })
}

fn quote_path(path: &str, built_in: bool) -> Option<String> {
    if path
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '/' | '\\' | '.' | '_' | '-' | ':' | '+'))
        && (cfg!(windows) || !path.contains('\\'))
    {
        return Some(path.to_string());
    }
    if cfg!(windows) {
        // cmd.exe expands these even inside quotes. Avoid producing a command
        // that refers to a different file; standalone editor built-ins are safe.
        if path.contains('"') || (!built_in && path.contains(['%', '!'])) {
            return None;
        }
        return Some(format!("\"{path}\""));
    }
    if built_in && !path.contains('"') {
        return Some(format!("\"{path}\""));
    }
    Some(format!("'{}'", path.replace('\'', "'\\''")))
}

// Locate the whole argument under the cursor, but match only its typed prefix.
// Incomplete quotes are accepted, and text after the argument is preserved.
fn word_at(input: &str, cursor: usize) -> Option<(Range<usize>, String)> {
    if !input.is_char_boundary(cursor) {
        return None;
    }
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut prefix = String::new();
    for (index, character) in input.char_indices() {
        if escaped {
            if index < cursor {
                prefix.push(character);
            }
            escaped = false;
            continue;
        }
        let next = input[index + character.len_utf8()..].chars().next();
        if character == '\\'
            && !cfg!(windows)
            && quote != Some('\'')
            && (quote.is_none() || next.is_some_and(|c| matches!(c, '"' | '\\' | '$' | '`')))
        {
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            } else if index < cursor {
                prefix.push(character);
            }
        } else if matches!(character, '"' | '\'') {
            quote = Some(character);
        } else if character.is_whitespace()
            || matches!(character, '|' | '&' | '<' | '>')
            || (character == ';' && !cfg!(windows))
        {
            if index >= cursor {
                return Some((start..index, prefix));
            }
            start = index + character.len_utf8();
            prefix.clear();
        } else if index < cursor {
            prefix.push(character);
        }
    }
    Some((start..input.len(), prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache(cwd: &Path, names: &[(&str, bool)]) -> Cache {
        let mut cache = Cache::default();
        cache.extend(
            &names
                .iter()
                .map(|(name, directory)| OutputEntry {
                    range: 0..0,
                    location: None,
                    commit: None,
                    path: cwd.join(name),
                    kind: if *directory {
                        EntryKind::Directory
                    } else {
                        EntryKind::Text
                    },
                })
                .collect::<Vec<_>>(),
        );
        cache
    }

    #[test]
    fn cycles_both_directions_using_cached_paths_without_filesystem_access() {
        let cwd = std::env::temp_dir().join("potyi-nonexistent-completion-directory");
        let mut cache = cache(&cwd, &[("notes-b.txt", false), ("notes-a.txt", false)]);
        let mut input = "edit no".to_string();
        let mut cursor = input.len();
        cache.complete(&mut input, &mut cursor, &cwd, false);
        assert_eq!(input, "edit notes-a.txt");
        cache.complete(&mut input, &mut cursor, &cwd, false);
        assert_eq!(input, "edit notes-b.txt");
        cache.complete(&mut input, &mut cursor, &cwd, false);
        assert_eq!(input, "edit notes-a.txt");
        cache.complete(&mut input, &mut cursor, &cwd, true);
        assert_eq!(input, "edit notes-b.txt");
    }

    #[test]
    fn completes_quoted_unicode_argument_at_cursor_and_preserves_other_arguments() {
        let cwd = std::env::temp_dir();
        let mut cache = cache(&cwd, &[("é notes.txt", false)]);
        let mut input = "edit \"é n\" trailing".to_string();
        let mut cursor = "edit \"é n".len();
        cache.complete(&mut input, &mut cursor, &cwd, false);
        assert_eq!(input, "edit \"é notes.txt\" trailing");
        assert_eq!(cursor, "edit \"é notes.txt\"".len());
        cache.reset_cycle();
        input = "edit \"é n".into();
        cursor = input.len();
        cache.complete(&mut input, &mut cursor, &cwd, false);
        assert_eq!(input, "edit \"é notes.txt\"");
    }

    #[test]
    fn directory_and_nested_path_completion_preserve_the_typed_path_style() {
        let cwd = std::env::temp_dir();
        for (initial, expected) in [
            ("cd sr", "cd src/"),
            ("edit ./src/ma", "edit ./src/main.rs"),
        ] {
            let mut cache = cache(
                &cwd,
                &[("src", true), ("src.txt", false), ("src/main.rs", false)],
            );
            let mut input = initial.to_string();
            let mut cursor = input.len();
            cache.complete(&mut input, &mut cursor, &cwd, false);
            assert_eq!(input, expected);
        }
    }

    #[test]
    fn no_match_and_command_names_in_pipelines_are_left_unchanged() {
        let cwd = std::env::temp_dir();
        let mut cache = cache(&cwd, &[("cat", false)]);
        for initial in ["cat | ca", "edit missing"] {
            let mut input = initial.to_string();
            let mut cursor = input.len();
            cache.complete(&mut input, &mut cursor, &cwd, false);
            assert_eq!(input, initial);
        }
    }

    #[test]
    fn completes_a_bare_filename_for_direct_open() {
        let cwd = std::env::temp_dir();
        let mut cache = cache(&cwd, &[("Cargo.lock", false)]);
        let mut input = "Car".to_string();
        let mut cursor = input.len();
        cache.complete(&mut input, &mut cursor, &cwd, false);
        assert_eq!(input, "Cargo.lock");
    }

    #[test]
    fn cache_is_bounded_by_both_entry_count_and_path_bytes() {
        let cwd = std::env::temp_dir();
        let mut cache = Cache::default();
        for i in 0..MAX_ENTRIES + 100 {
            cache.extend(&[OutputEntry {
                range: 0..0,
                location: None,
                commit: None,
                path: cwd.join(format!("{i}.txt")),
                kind: EntryKind::Text,
            }]);
        }
        assert!(cache.entries.len() <= MAX_ENTRIES);
        assert!(cache.bytes <= MAX_PATH_BYTES);
        cache.clear();
        for _ in 0..1000 {
            cache.extend(&[OutputEntry {
                range: 0..0,
                location: None,
                commit: None,
                path: cwd.join("x".repeat(1000)),
                kind: EntryKind::Text,
            }]);
        }
        assert!(cache.bytes <= MAX_PATH_BYTES);
        assert!(cache.entries.len() < 1000);
    }

    #[test]
    #[cfg(not(windows))]
    fn shell_quotes_keep_metacharacters_and_quotes_literal() {
        for name in [
            "a file.txt",
            "a; echo bad",
            "$(echo bad)",
            "a\"b'c",
            "a\\b",
            "`echo bad`",
        ] {
            let quoted = quote_path(name, false).unwrap();
            assert_eq!(super::super::touch_arguments(&quoted).unwrap(), [name]);
            assert!(!super::super::has_shell_operators(&format!("cat {quoted}")));
            let quoted = quote_path(name, true).unwrap();
            assert_eq!(super::super::unquote_argument(&quoted).unwrap(), name);
        }
    }
}
