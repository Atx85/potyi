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

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

pub(crate) const KEYBINDINGS: &str =
    include_str!("../config/keybindings.toml");

pub(crate) const EDITOR: &str =
    include_str!("../config/editor.toml");

pub(crate) const SYNTAX_DEFINITIONS: &[(&str, &str)] = &[
    (
        "syntax/syntax_rs.toml",
        include_str!("../config/syntax/syntax_rs.toml"),
    ),
    (
        "syntax/syntax_javascript.toml",
        include_str!("../config/syntax/syntax_javascript.toml"),
    ),
];

pub(crate) struct ExtractionSummary {
    pub created: usize,
    pub existing: usize,
}

/// Write the embedded, editable defaults beneath `config_dir`.
///
/// Existing files are deliberately preserved so extracting defaults can
/// never erase a user's custom configuration.
pub(crate) fn extract_defaults(
    config_dir: &Path,
) -> io::Result<ExtractionSummary> {
    let mut summary = ExtractionSummary {
        created: 0,
        existing: 0,
    };

    let files = [
        ("editor.toml", EDITOR),
        ("keybindings.toml", KEYBINDINGS),
    ]
    .into_iter()
    .chain(SYNTAX_DEFINITIONS.iter().copied());

    for (relative_path, contents) in files {
        let path = config_dir.join(relative_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error)
                if error.kind()
                    == io::ErrorKind::AlreadyExists =>
            {
                summary.existing += 1;
                continue;
            }
            Err(error) => return Err(error),
        };

        if let Err(error) = file.write_all(contents.as_bytes()) {
            // The file was created by this function, so removing a partial
            // write cannot affect pre-existing user data.
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(error);
        }

        summary.created += 1;
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn extraction_creates_defaults_without_overwriting() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "potyi-defaults-{}-{unique}",
            std::process::id(),
        ));

        let first = extract_defaults(&directory).unwrap();
        assert_eq!(first.created, 4);
        assert_eq!(first.existing, 0);

        let keybindings = directory.join("keybindings.toml");
        fs::write(&keybindings, "custom").unwrap();

        let second = extract_defaults(&directory).unwrap();
        assert_eq!(second.created, 0);
        assert_eq!(second.existing, 4);
        assert_eq!(fs::read_to_string(keybindings).unwrap(), "custom");

        fs::remove_dir_all(directory).unwrap();
    }
}
