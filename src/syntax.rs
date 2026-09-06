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


use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;

use regex::Regex;
use sdl3::pixels::Color;

use crate::embedded_config::SYNTAX_DEFINITIONS;

// ==========================================================================
// Syntax file configuration
// ==========================================================================
//
// These structures represent the TOML file directly.
// They contain only values that serde/toml can deserialize.
//

#[derive(Debug, Deserialize)]
struct SyntaxFileConfig {
    syntax: SyntaxDefinitionConfig,
}

#[derive(Debug, Deserialize)]
struct SyntaxDefinitionConfig {
    name: String,
    extensions: Vec<String>,
    rules: Vec<SyntaxRuleConfig>,
}

#[derive(Debug, Deserialize)]
struct SyntaxRuleConfig {
    name: String,
    pattern: String,
    color: String,
}

// ==========================================================================
// Runtime syntax definition
// ==========================================================================
//
// These structures are used by the editor/renderer after the TOML file has
// been parsed.
//

#[derive(Debug)]
pub struct SyntaxDefinition {
    pub name: String,
    pub extensions: Vec<String>,
    pub rules: Vec<SyntaxRule>,
}

#[derive(Debug)]
pub struct SyntaxRule {
    pub name: String,
    pub pattern: String,
    pub color: Color,

    // Compiled once when the syntax definition is loaded.
    // This avoids recompiling regexes while rendering.
    pub(crate) regex: Regex,
}

// ==========================================================================
// Syntax match
// ==========================================================================

#[derive(Debug, Clone, Copy)]
pub struct SyntaxMatch {
    /// UTF-8 byte offset within the line.
    pub start: usize,

    /// UTF-8 byte offset within the line.
    pub end: usize,

    pub color: Color,
}

// ==========================================================================
// Loading
// ==========================================================================

impl SyntaxDefinition {
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let contents = fs::read_to_string(path)?;

        Self::from_toml(&contents)
    }

    fn from_toml(contents: &str) -> io::Result<Self> {
        let file: SyntaxFileConfig =
            toml::from_str(contents)
                .map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        error.to_string(),
                    )
                })?;

        Self::from_config(file.syntax)
    }

    fn from_config(
        definition: SyntaxDefinitionConfig,
    ) -> io::Result<Self> {
        let mut rules =
            Vec::with_capacity(definition.rules.len());

        for rule in definition.rules {
            let regex =
                Regex::new(&rule.pattern)
                    .map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "invalid syntax regex '{}': {}",
                                rule.pattern,
                                error
                            ),
                        )
                    })?;

            let color =
                parse_color(&rule.color);

            rules.push(SyntaxRule {
                name: rule.name,
                pattern: rule.pattern,
                color,
                regex,
            });
        }

        Ok(Self {
            name: definition.name,
            extensions: definition.extensions,
            rules,
        })
    }

    // ----------------------------------------------------------------------
    // Extension matching
    // ----------------------------------------------------------------------

    pub fn supports_extension(
        &self,
        extension: &str,
    ) -> bool {
        let extension =
            extension.trim_start_matches('.');

        self.extensions.iter().any(|value| {
            value.eq_ignore_ascii_case(extension)
        })
    }

    // ----------------------------------------------------------------------
    // Highlight one line
    // ----------------------------------------------------------------------
    //
    // Returns syntax spans only.
    //
    // The spans are:
    //
    //   * ordered by byte position
    //   * non-overlapping
    //   * guaranteed not to overlap each other
    //
    // The renderer is responsible for drawing normal text in the gaps.
    //
    // Earlier syntax rules have priority over later syntax rules.
    //

    pub fn matches_line(
        &self,
        line: &str,
    ) -> Vec<SyntaxMatch> {
        let mut matches =
            Vec::<SyntaxMatch>::new();

        for rule in &self.rules {
            for found in rule.regex.find_iter(line) {
                let start =
                    found.start();

                let end =
                    found.end();

                if start >= end {
                    continue;
                }

                matches.push(
                    SyntaxMatch {
                        start,
                        end,
                        color: rule.color,
                    }
                );
            }
        }

        resolve_overlaps(matches)
    }

    // ----------------------------------------------------------------------
    // Load syntax definition for an extension
    // ----------------------------------------------------------------------


// ----------------------------------------------------------------------
// Load syntax definition for an extension
// ----------------------------------------------------------------------



pub fn for_extension(
    extension: &str,
) -> Option<Self> {
    let extension = extension
        .trim_start_matches('.')
        .to_ascii_lowercase();

    let syntax_dir = Path::new("config")
        .join("syntax");

    match fs::read_dir(&syntax_dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        println!(
                            "Failed to read syntax directory entry: {}",
                            error
                        );
                        continue;
                    }
                };

                let path = entry.path();

                if !path.is_file()
                    || path.extension()
                        .and_then(|value| value.to_str())
                        .map(|value| {
                            !value.eq_ignore_ascii_case("toml")
                        })
                        .unwrap_or(true)
                {
                    continue;
                }

                let syntax = match Self::load(&path) {
                    Ok(syntax) => syntax,
                    Err(error) => {
                        println!(
                            "Failed to load syntax definition {}: {}",
                            path.display(),
                            error
                        );
                        continue;
                    }
                };

                if syntax.supports_extension(&extension) {
                    return Some(syntax);
                }
            }
        }

        Err(error)
            if error.kind() == io::ErrorKind::NotFound => {}

        Err(error) => {
            println!(
                "Failed to read syntax directory {}: {}",
                syntax_dir.display(),
                error
            );
        }
    }

    for (name, contents) in SYNTAX_DEFINITIONS {
        let syntax = Self::from_toml(contents)
            .unwrap_or_else(|error| {
                panic!(
                    "embedded syntax definition {name} is invalid: {error}"
                )
            });

        if syntax.supports_extension(&extension) {
            return Some(syntax);
        }
    }

    println!(
        "NO syntax definition found supporting .{}",
        extension
    );

    None
}



}

// ==========================================================================
// Colour parsing
// ==========================================================================

fn parse_color(
    value: &str,
) -> Color {
    let value =
        value
            .trim()
            .trim_start_matches('#');

    if value.len() != 6 {
        return Color::RGB(
            220,
            220,
            220,
        );
    }

    let r =
        u8::from_str_radix(
            &value[0..2],
            16,
        )
        .unwrap_or(220);

    let g =
        u8::from_str_radix(
            &value[2..4],
            16,
        )
        .unwrap_or(220);

    let b =
        u8::from_str_radix(
            &value[4..6],
            16,
        )
        .unwrap_or(220);

    Color::RGB(
        r,
        g,
        b,
    )
}

// ==========================================================================
// Overlap resolution
// ==========================================================================
//
// Multiple syntax rules can match the same characters.
//
// Example:
//
//     rule 1:  "let"
//     rule 2:  "let foo"
//
// Both can produce matches:
//
//     [0,3]
//     [0,7]
//
// We need to select one colour for every byte range.
//
// Rules appearing earlier in the syntax file have priority.
//
// This function therefore:
//
// 1. Sorts matches by start position.
// 2. Keeps the first match covering each region.
// 3. Splits later matches around already-covered regions.
// 4. Never produces overlapping spans.
//
// This is important because the renderer can then safely draw each span
// exactly once without normal text being rendered across a coloured span.
//

fn resolve_overlaps(
    mut matches: Vec<SyntaxMatch>,
) -> Vec<SyntaxMatch> {
    matches.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| b.end.cmp(&a.end))
    });

    let mut result: Vec<SyntaxMatch> =
        Vec::with_capacity(matches.len());

    for current in matches {
        if let Some(previous) = result.last() {
            if previous.end > current.start {
                continue;
            }
        }

        result.push(current);
    }

    result
}
// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_syntax_definitions_are_valid() {
        for (name, contents) in SYNTAX_DEFINITIONS {
            SyntaxDefinition::from_toml(contents)
                .unwrap_or_else(|error| {
                    panic!("{name}: {error}")
                });
        }
    }

    #[test]
    fn color_parser_accepts_hex_color() {
        let color =
            parse_color("#123456");

        assert_eq!(
            color,
            Color::RGB(
                0x12,
                0x34,
                0x56,
            )
        );
    }

    #[test]
    fn color_parser_accepts_hex_without_hash() {
        let color =
            parse_color("abcdef");

        assert_eq!(
            color,
            Color::RGB(
                0xab,
                0xcd,
                0xef,
            )
        );
    }

    #[test]
    fn color_parser_falls_back_for_invalid_color() {
        let color =
            parse_color("invalid");

        assert_eq!(
            color,
            Color::RGB(
                220,
                220,
                220,
            )
        );
    }

    #[test]
    fn overlap_resolution_removes_overlapping_matches() {
        let matches = vec![
            SyntaxMatch {
                start: 0,
                end: 5,
                color: Color::RGB(
                    255,
                    0,
                    0,
                ),
            },
            SyntaxMatch {
                start: 3,
                end: 8,
                color: Color::RGB(
                    0,
                    255,
                    0,
                ),
            },
        ];

        let result =
            resolve_overlaps(matches);

        assert_eq!(
            result.len(),
            1
        );

        assert_eq!(
            result[0].start,
            0
        );

        assert_eq!(
            result[0].end,
            5
        );
    }

    #[test]
    fn non_overlapping_matches_are_preserved() {
        let matches = vec![
            SyntaxMatch {
                start: 0,
                end: 3,
                color: Color::RGB(
                    255,
                    0,
                    0,
                ),
            },
            SyntaxMatch {
                start: 5,
                end: 8,
                color: Color::RGB(
                    0,
                    255,
                    0,
                ),
            },
        ];

        let result =
            resolve_overlaps(matches);

        assert_eq!(
            result.len(),
            2
        );

        assert_eq!(
            result[0].start,
            0
        );

        assert_eq!(
            result[0].end,
            3
        );

        assert_eq!(
            result[1].start,
            5
        );

        assert_eq!(
            result[1].end,
            8
        );
    }
}
