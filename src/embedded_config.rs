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

pub(crate) const LSP: &str = include_str!("../config/lsp.toml");

pub(crate) const FORMATTERS: &str =
    include_str!("../config/formatters.toml");

pub(crate) const SYNTAX_DEFINITIONS: &[(&str, &str)] = &[
    (
        "syntax/syntax_rs.toml",
        include_str!("../config/syntax/syntax_rs.toml"),
    ),
    (
        "syntax/syntax_javascript.toml",
        include_str!("../config/syntax/syntax_javascript.toml"),
    ),
    (
        "syntax/syntax_c.toml",
        include_str!("../config/syntax/syntax_c.toml"),
    ),
    (
        "syntax/syntax_cpp.toml",
        include_str!("../config/syntax/syntax_cpp.toml"),
    ),
    (
        "syntax/syntax_csharp.toml",
        include_str!("../config/syntax/syntax_csharp.toml"),
    ),
    (
        "syntax/syntax_python.toml",
        include_str!("../config/syntax/syntax_python.toml"),
    ),
    (
        "syntax/syntax_php.toml",
        include_str!("../config/syntax/syntax_php.toml"),
    ),
    (
        "syntax/syntax_typescript.toml",
        include_str!("../config/syntax/syntax_typescript.toml"),
    ),
    (
        "syntax/syntax_go.toml",
        include_str!("../config/syntax/syntax_go.toml"),
    ),
    (
        "syntax/syntax_java.toml",
        include_str!("../config/syntax/syntax_java.toml"),
    ),
    (
        "syntax/syntax_shell.toml",
        include_str!("../config/syntax/syntax_shell.toml"),
    ),
    (
        "syntax/syntax_lua.toml",
        include_str!("../config/syntax/syntax_lua.toml"),
    ),
    (
        "syntax/syntax_json.toml",
        include_str!("../config/syntax/syntax_json.toml"),
    ),
    (
        "syntax/syntax_toml.toml",
        include_str!("../config/syntax/syntax_toml.toml"),
    ),
    (
        "syntax/syntax_yaml.toml",
        include_str!("../config/syntax/syntax_yaml.toml"),
    ),
    (
        "syntax/syntax_html.toml",
        include_str!("../config/syntax/syntax_html.toml"),
    ),
    (
        "syntax/syntax_css.toml",
        include_str!("../config/syntax/syntax_css.toml"),
    ),
    (
        "syntax/syntax_sql.toml",
        include_str!("../config/syntax/syntax_sql.toml"),
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
        ("formatters.toml", FORMATTERS),
        ("lsp.toml", LSP),
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
        assert_eq!(first.created, 4 + SYNTAX_DEFINITIONS.len());
        assert_eq!(first.existing, 0);

        let keybindings = directory.join("keybindings.toml");
        fs::write(&keybindings, "custom").unwrap();

        let second = extract_defaults(&directory).unwrap();
        assert_eq!(second.created, 0);
        assert_eq!(second.existing, 4 + SYNTAX_DEFINITIONS.len());
        assert_eq!(fs::read_to_string(keybindings).unwrap(), "custom");

        fs::remove_dir_all(directory).unwrap();
    }
}
