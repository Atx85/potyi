// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileLink {
    pub(crate) path: PathBuf,
    pub(crate) line: Option<usize>,
    pub(crate) column: Option<usize>,
    pub(crate) byte_column: bool,
    pub(crate) read_only: bool,
    pub(crate) other_pane: bool,
}
pub(crate) fn resolve(value: &str, cwd: &Path) -> Option<FileLink> {
    let value = value.trim_matches(['\'', '"', '(', ')', '[', ']', ',', ';']);
    let full = cwd.join(value);
    let (path, line, column) = if full.is_file() || full.is_dir() {
        (full, None, None)
    } else {
        let regex = regex::Regex::new(r"^(.+?):([0-9]+)(?::([0-9]+))?(?::|$)").ok()?;
        let matched = regex.captures(value)?;
        let path = cwd.join(matched.get(1)?.as_str());
        if !path.is_file() {
            return None;
        }
        (
            path,
            matched.get(2).and_then(|v| v.as_str().parse().ok()),
            matched.get(3).and_then(|v| v.as_str().parse().ok()),
        )
    };
    Some(FileLink {
        path,
        line,
        column,
        byte_column: false,
        read_only: false,
        other_pane: false,
    })
}
pub(crate) fn at(
    line: &str,
    column: usize,
    cwd: &Path,
) -> Option<(std::ops::Range<usize>, FileLink)> {
    // Prefer a whole filename (including spaces), then quoted filenames, then tokens.
    let mut ranges = vec![0..line.len()];
    let mut quote = None;
    for (i, ch) in line.char_indices() {
        if matches!(ch, '\'' | '"') {
            if let Some((start, opening)) = quote {
                if opening == ch {
                    ranges.push(start..i + ch.len_utf8());
                    quote = None;
                }
            } else {
                quote = Some((i, ch));
            }
        }
    }
    let mut start = 0;
    for (i, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if start < i {
                ranges.push(start..i);
            }
            start = i + ch.len_utf8();
        }
    }
    if start < line.len() {
        ranges.push(start..line.len());
    }
    let byte = line
        .char_indices()
        .nth(column)
        .map_or(line.len(), |(i, _)| i);
    ranges.into_iter().find_map(|range| {
        if !range.contains(&byte) {
            return None;
        }
        resolve(&line[range.clone()], cwd).map(|link| (range, link))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_links_keep_spaces_unicode_and_diagnostic_locations() {
        let root = std::env::temp_dir().join(format!("potyi-link-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("é notes.rs"), "test").unwrap();
        let (_, link) = at("error: 'é notes.rs:12:3' failed", 10, &root).unwrap();
        assert_eq!(link.line, Some(12));
        assert_eq!(link.column, Some(3));
        assert!(resolve("missing.rs:12", &root).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
