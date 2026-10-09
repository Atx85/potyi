// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Small owned command input; no attempt to mirror a shell's line editor.
use std::{
    collections::VecDeque,
    ops::Range,
    path::{Component, Path, PathBuf},
};
const MAX_COMMAND: usize = 64 * 1024;
const MAX_HISTORY: usize = 100;
const MAX_HISTORY_BYTES: usize = 256 * 1024;
const MAX_MATCHES: usize = 4096;
const MAX_MATCH_BYTES: usize = 256 * 1024;
#[derive(Default)]
pub(crate) struct Prompt {
    text: String,
    cursor: usize,
    history: VecDeque<String>,
    history_bytes: usize,
    history_position: Option<usize>,
    draft: String,
    cycle: Option<Cycle>,
}
struct Cycle {
    before: String,
    after: String,
    matches: Vec<String>,
    selected: usize,
}
impl Prompt {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }
    pub(crate) fn set_cursor(&mut self, byte: usize) {
        let mut byte = byte.min(self.text.len());
        while !self.text.is_char_boundary(byte) {
            byte -= 1;
        }
        self.cursor = byte;
        self.reset_completion();
    }
    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.draft.clear();
        self.history_position = None;
        self.reset_completion();
    }
    pub(crate) fn reset_completion(&mut self) {
        self.cycle = None;
    }
    fn reset_history_navigation(&mut self) {
        self.history_position = None;
        self.draft.clear();
    }
    pub(crate) fn insert(&mut self, value: &str) {
        if value.is_empty() {
            return;
        }
        self.reset_completion();
        let remaining = MAX_COMMAND - self.text.len();
        let mut clean = String::with_capacity(remaining.min(value.len()));
        for c in value
            .chars()
            .take(remaining)
            .map(|c| if matches!(c, '\r' | '\n') { ' ' } else { c })
            .filter(|c| !c.is_control() || *c == '\t')
        {
            if clean.len() + c.len_utf8() > remaining {
                break;
            }
            clean.push(c);
        }
        self.text.insert_str(self.cursor, &clean);
        self.cursor += clean.len();
        if !clean.is_empty() {
            self.reset_history_navigation();
        }
    }
    pub(crate) fn backspace(&mut self) {
        self.reset_completion();
        if let Some((previous, _)) = self.text[..self.cursor].char_indices().next_back() {
            self.text.drain(previous..self.cursor);
            self.cursor = previous;
            self.reset_history_navigation();
        }
    }
    pub(crate) fn delete(&mut self) {
        self.reset_completion();
        if let Some(c) = self.text[self.cursor..].chars().next() {
            self.text.drain(self.cursor..self.cursor + c.len_utf8());
            self.reset_history_navigation();
        }
    }
    pub(crate) fn move_left(&mut self) {
        self.reset_completion();
        self.cursor = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i);
    }
    pub(crate) fn move_right(&mut self) {
        self.reset_completion();
        if let Some(c) = self.text[self.cursor..].chars().next() {
            self.cursor += c.len_utf8();
        }
    }
    pub(crate) fn home(&mut self) {
        self.reset_completion();
        self.cursor = 0;
    }
    pub(crate) fn end(&mut self) {
        self.reset_completion();
        self.cursor = self.text.len();
    }
    pub(crate) fn history_previous(&mut self) {
        self.reset_completion();
        if self.history.is_empty() {
            return;
        }
        let index = match self.history_position {
            None => {
                self.draft.clone_from(&self.text);
                self.history.len() - 1
            }
            Some(i) => i.saturating_sub(1),
        };
        self.history_position = Some(index);
        self.text.clone_from(&self.history[index]);
        self.cursor = self.text.len();
    }
    pub(crate) fn history_next(&mut self) {
        self.reset_completion();
        let Some(index) = self.history_position else {
            return;
        };
        if index + 1 < self.history.len() {
            self.history_position = Some(index + 1);
            self.text.clone_from(&self.history[index + 1]);
        } else {
            self.history_position = None;
            self.text = std::mem::take(&mut self.draft);
        }
        self.cursor = self.text.len();
    }
    pub(crate) fn take_command(&mut self) -> Option<String> {
        self.reset_completion();
        self.history_position = None;
        self.draft.clear();
        self.cursor = 0;
        let command = std::mem::take(&mut self.text).trim().to_owned();
        if command.is_empty() {
            return None;
        }
        self.remember_command(&command);
        Some(command)
    }
    /// Remember commands dispatched through the command bar or Again as well
    /// as commands consumed from this input. Never retain a truncated command.
    pub(crate) fn remember_command(&mut self, command: &str) {
        if command.len() > MAX_COMMAND {
            return;
        }
        let command = command.trim();
        if command.is_empty() || self.history.back().map(String::as_str) == Some(command) {
            return;
        }
        while self.history.len() >= MAX_HISTORY
            || self.history_bytes + command.len() > MAX_HISTORY_BYTES
        {
            let Some(old) = self.history.pop_front() else {
                break;
            };
            self.history_bytes -= old.len();
        }
        self.history_bytes += command.len();
        self.history.push_back(command.to_owned());
    }
    pub(crate) fn complete(
        &mut self,
        backwards: bool,
        cwd: &Path,
        candidates: &[(PathBuf, bool)],
    ) -> Option<String> {
        self.reset_history_navigation();
        if let Some(cycle) = &mut self.cycle {
            cycle.selected = if backwards {
                cycle
                    .selected
                    .checked_sub(1)
                    .unwrap_or(cycle.matches.len() - 1)
            } else {
                (cycle.selected + 1) % cycle.matches.len()
            };
        } else {
            let (range, prefix) = word_at(&self.text, self.cursor)?;
            let before = self.text[..range.start].trim_end();
            if ((range.start == 0 && range.end < self.text.len())
                || before.ends_with(['|', '&', ';']))
                && !prefix.contains(['/', '\\'])
            {
                return None;
            }
            let command = self.text.split_whitespace().next().unwrap_or("");
            let built_in =
                !has_shell_operators(&self.text) && matches!(command, "edit" | "view" | "cd");
            let mut matches = Vec::new();
            let mut bytes = 0;
            for (path, directory) in candidates.iter().take(MAX_MATCHES) {
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
                if name.starts_with("~/") {
                    name = expand_home(&name).to_string_lossy().into_owned();
                }
                let Some(quoted) = quote_path(&name, built_in) else {
                    continue;
                };
                if self.text.len() - range.len() + quoted.len() > MAX_COMMAND
                    || bytes + quoted.len() > MAX_MATCH_BYTES
                {
                    continue;
                }
                bytes += quoted.len();
                matches.push(quoted);
            }
            matches.sort();
            matches.dedup();
            if matches.is_empty() {
                return Some("No matching names in the latest listing".into());
            }
            let selected = if backwards { matches.len() - 1 } else { 0 };
            self.cycle = Some(Cycle {
                before: self.text[..range.start].into(),
                after: self.text[range.end..].into(),
                matches,
                selected,
            });
        }
        let cycle = self.cycle.as_ref().unwrap();
        self.text.clear();
        self.text.push_str(&cycle.before);
        self.text.push_str(&cycle.matches[cycle.selected]);
        self.cursor = self.text.len();
        self.text.push_str(&cycle.after);
        Some(format!(
            "Match {} of {} · Tab / Shift+Tab cycles",
            cycle.selected + 1,
            cycle.matches.len()
        ))
    }
}
fn has_shell_operators(command: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;

    for character in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        // cmd.exe uses caret escapes and only double quotes; Unix shells
        // also support single quotes and backslash escapes.
        if (cfg!(windows) && character == '^' && quote.is_none())
            || (!cfg!(windows) && character == '\\' && quote != Some('\''))
        {
            escaped = true;
            continue;
        }

        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
            continue;
        }

        match character {
            '"' => quote = Some(character),
            '\'' if !cfg!(windows) => quote = Some(character),
            '|' | '&' | '<' | '>' | '\n' | '(' | ')' => return true,
            ';' if !cfg!(windows) => return true,
            _ => {}
        }
    }

    false
}
fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
fn expand_home(name: &str) -> PathBuf {
    if name == "~" || name.starts_with("~/") || name.starts_with("~\\") {
        home_directory()
            .unwrap_or_default()
            .join(name.get(2..).unwrap_or(""))
    } else {
        name.into()
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
    #[test]
    fn owned_prompt_edits_unicode_and_restores_history_draft() {
        let mut prompt = Prompt::default();
        prompt.insert("é界x");
        prompt.move_left();
        prompt.backspace();
        assert_eq!(prompt.text(), "éx");
        prompt.home();
        prompt.delete();
        assert_eq!(prompt.text(), "x");
        prompt.end();
        assert_eq!(prompt.take_command().as_deref(), Some("x"));
        prompt.insert("draft");
        prompt.history_previous();
        assert_eq!(prompt.text(), "x");
        prompt.history_next();
        assert_eq!(prompt.text(), "draft");
    }
    #[test]
    fn owned_prompt_completion_preserves_tail_and_cycles_both_directions() {
        let cwd = Path::new("/tmp");
        let candidates = vec![
            (cwd.join("é notes.txt"), false),
            (cwd.join("é second.txt"), false),
        ];
        let mut prompt = Prompt::default();
        prompt.insert("edit é tail");
        prompt.home();
        for _ in 0..6 {
            prompt.move_right();
        }
        prompt.complete(false, cwd, &candidates);
        assert!(prompt.text().contains("notes.txt"));
        assert!(prompt.text().ends_with(" tail"));
        prompt.complete(false, cwd, &candidates);
        assert!(prompt.text().contains("second.txt"));
        prompt.complete(true, cwd, &candidates);
        assert!(prompt.text().contains("notes.txt"));
    }
    #[cfg(unix)]
    #[test]
    fn owned_prompt_external_and_executable_paths_never_gain_shell_expansion() {
        let cwd = Path::new("/tmp");
        for (input, name, expected) in [
            ("cat cost", "cost$PRICE.txt", "cat 'cost$PRICE.txt'"),
            ("./run", "run$PRICE.sh", "'./run$PRICE.sh'"),
            ("cat tick", "tick`whoami`.txt", "cat 'tick`whoami`.txt'"),
        ] {
            let mut prompt = Prompt::default();
            prompt.insert(input);
            prompt.complete(false, cwd, &[(cwd.join(name), false)]);
            assert_eq!(prompt.text(), expected);
        }
    }
    #[test]
    fn owned_prompt_clear_preserves_command_history() {
        let mut prompt = Prompt::default();
        prompt.insert("first");
        prompt.take_command();
        prompt.insert("draft");
        prompt.history_previous();
        prompt.clear();
        assert!(prompt.text().is_empty());
        prompt.history_previous();
        assert_eq!(prompt.text(), "first");
        prompt.history_next();
        assert!(prompt.text().is_empty());
    }
    #[test]
    fn owned_prompt_dispatch_history_is_shared_bounded_and_deduplicates_again() {
        let mut prompt = Prompt::default();
        prompt.remember_command("  from-command-bar é  ");
        prompt.insert("typed-command");
        let command = prompt.take_command().unwrap();
        prompt.remember_command(&command);
        prompt.remember_command(&command);
        assert_eq!(prompt.history.len(), 2);
        prompt.history_previous();
        assert_eq!(prompt.text(), "typed-command");
        prompt.history_previous();
        assert_eq!(prompt.text(), "from-command-bar é");
        prompt.remember_command("");
        prompt.remember_command(&"x".repeat(MAX_COMMAND + 1));
        assert_eq!(
            prompt.history.len(),
            2,
            "oversize commands are rejected without truncation"
        );
        for index in 0..120 {
            prompt.remember_command(&format!("{index}-{}", "x".repeat(MAX_COMMAND - 4)));
        }
        assert!(prompt.history.len() <= MAX_HISTORY);
        assert!(prompt.history_bytes <= MAX_HISTORY_BYTES);
        assert!(prompt.history.back().unwrap().starts_with("119-"));
    }
    #[test]
    fn owned_prompt_editing_and_completion_leave_history_navigation() {
        let mut prompt = Prompt::default();
        prompt.remember_command("edit café");
        prompt.insert("draft");
        prompt.history_previous();
        prompt.insert("x");
        prompt.history_next();
        assert_eq!(prompt.text(), "edit caféx");
        prompt.history_previous();
        prompt.backspace();
        prompt.history_next();
        assert_eq!(prompt.text(), "edit caf");
        prompt.history_previous();
        prompt.home();
        prompt.delete();
        prompt.history_next();
        assert_eq!(prompt.text(), "dit café");
        prompt.history_previous();
        let directory = Path::new("/tmp");
        prompt.complete(false, directory, &[(directory.join("café.rs"), false)]);
        let completed = prompt.text().to_owned();
        prompt.history_next();
        assert_eq!(prompt.text(), completed);
    }
    #[test]
    fn owned_prompt_paste_normalizes_newlines_inside_the_bounded_buffer() {
        let mut prompt = Prompt::default();
        prompt.insert("first\r\n第二\nthird\rfourth");
        assert_eq!(prompt.text(), "first  第二 third fourth");
        prompt.clear();
        prompt.insert(&"\r\n".repeat(MAX_COMMAND));
        assert_eq!(prompt.text().len(), MAX_COMMAND);
        assert!(prompt.text().bytes().all(|byte| byte == b' '));
    }
    #[test]
    fn owned_prompt_storage_has_byte_limits_and_keeps_utf8_boundaries() {
        let mut prompt = Prompt::default();
        for index in 0..120 {
            prompt.insert(&format!("{index}-{}", "界".repeat(MAX_COMMAND)));
            prompt.take_command();
        }
        assert!(prompt.history.len() <= MAX_HISTORY);
        assert!(prompt.history_bytes <= MAX_HISTORY_BYTES);
        assert_eq!(prompt.history.len(), 4);
        assert!(prompt.history.back().unwrap().starts_with("119-"));
        let mut small = Prompt::default();
        for index in 0..120 {
            small.insert(&format!("command{index}"));
            small.take_command();
        }
        assert_eq!(small.history.len(), 100);
        assert_eq!(small.history.front().unwrap(), "command20");
        prompt.insert(&"é".repeat(MAX_COMMAND));
        assert!(prompt.text().len() <= MAX_COMMAND);
        assert!(prompt.text().is_char_boundary(prompt.cursor()));
    }
}
