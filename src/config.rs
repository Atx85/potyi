// Pötyi - Lightweight text editor
// Copyright (C) 2026  Attila Banko
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

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use crate::embedded_config::EDITOR;

// ==========================================================================
// Editor configuration
// ==========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,

    #[serde(default)]
    pub insert_spaces: bool,

    #[serde(default)]
    pub line_numbers: LineNumberMode,

    #[serde(default = "default_font_size")]
    pub font_size: u16,

    #[serde(default)]
    pub keybinding_mode: KeybindingMode,
}

// ==========================================================================
// TOML root
// ==========================================================================

#[derive(Debug, Deserialize)]
struct EditorConfigFile {
    editor: EditorConfig,
}

// ==========================================================================
// Line numbers
// ==========================================================================

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LineNumberMode {
    Normal,
    Dynamic,
    Relative,
}

impl Default for LineNumberMode {
    fn default() -> Self {
        Self::Dynamic
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KeybindingMode {
    #[default]
    Conventional,
    Vim,
}

// ==========================================================================
// Defaults
// ==========================================================================

fn default_tab_width() -> usize {
    4
}

pub(crate) const MIN_FONT_SIZE: u16 = 8;
pub(crate) const MAX_FONT_SIZE: u16 = 72;

fn default_font_size() -> u16 {
    18
}

// ==========================================================================
// Loading
// ==========================================================================

impl EditorConfig {
    pub fn default() -> Self {
        Self::from_toml(
            EDITOR,
            Path::new("embedded editor.toml"),
        )
        .expect("embedded editor config must be valid")
    }

    pub fn load<P: AsRef<Path>>(
        path: P,
    ) -> io::Result<Self> {
        let path = path.as_ref();

        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,

            Err(error)
                if error.kind()
                    == io::ErrorKind::NotFound =>
            {
                return Ok(Self::default());
            }

            Err(error) => return Err(error),
        };

        Self::from_toml(&contents, path)
    }

    fn from_toml(
        contents: &str,
        path: &Path,
    ) -> io::Result<Self> {

        let config =
            toml::from_str::<EditorConfigFile>(
                contents
            )
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Failed to parse {}: {}",
                        path.display(),
                        error,
                    ),
                )
            })?;

        config.editor.validate(path)?;

        Ok(config.editor)
    }

    fn validate(
        &self,
        path: &Path,
    ) -> io::Result<()> {
        if !(MIN_FONT_SIZE..=MAX_FONT_SIZE)
            .contains(&self.font_size)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid font_size in {}: expected {} through {}, got {}",
                    path.display(),
                    MIN_FONT_SIZE,
                    MAX_FONT_SIZE,
                    self.font_size,
                ),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_editor(
        contents: &str,
    ) -> EditorConfig {
        toml::from_str::<EditorConfigFile>(
            contents
        )
        .unwrap()
        .editor
    }

    #[test]
    fn font_size_defaults_to_eighteen() {
        let config = parse_editor(
            "[editor]\n"
        );

        assert_eq!(config.font_size, 18);
        assert_eq!(
            config.keybinding_mode,
            KeybindingMode::Conventional,
        );
    }

    #[test]
    fn vim_keybinding_mode_parses() {
        let config = parse_editor(
            "[editor]\nkeybinding_mode = \"vim\"\n"
        );

        assert_eq!(
            config.keybinding_mode,
            KeybindingMode::Vim,
        );
    }

    #[test]
    fn all_line_number_modes_parse() {
        for (name, expected) in [
            ("normal", LineNumberMode::Normal),
            ("relative", LineNumberMode::Relative),
            ("dynamic", LineNumberMode::Dynamic),
        ] {
            let config = parse_editor(
                &format!(
                    "[editor]\nline_numbers = \"{name}\"\n"
                )
            );

            assert_eq!(config.line_numbers, expected);
        }
    }

    #[test]
    fn font_size_validation_has_safe_bounds() {
        let mut config = parse_editor(
            "[editor]\nfont_size = 8\n"
        );

        assert!(config.validate(
            Path::new("editor.toml")
        ).is_ok());

        config.font_size = 72;

        assert!(config.validate(
            Path::new("editor.toml")
        ).is_ok());

        config.font_size = 73;

        assert!(config.validate(
            Path::new("editor.toml")
        ).is_err());
    }

    #[test]
    fn missing_config_uses_embedded_editor_defaults() {
        let path = std::env::temp_dir()
            .join(format!(
                "potyi-missing-editor-{}.toml",
                std::process::id(),
            ));

        let config = EditorConfig::load(path)
            .unwrap();

        assert_eq!(config.tab_width, 2);
        assert!(!config.insert_spaces);
        assert_eq!(config.font_size, 18);
    }
}
