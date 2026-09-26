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

use regex::Regex;
use sdl3::pixels::Color;

use crate::embedded_config::SYNTAX_DEFINITIONS;

use crate::syntax_core::{self, SyntaxDefinitionConfig, SyntaxFileConfig};
#[cfg(test)]
use crate::syntax_core::{MAX_HIGHLIGHT_BYTES, MAX_HIGHLIGHT_MATCHES, resolve_overlaps};

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

pub type SyntaxMatch = syntax_core::SyntaxMatch<Color>;

// ==========================================================================
// Loading
// ==========================================================================

impl SyntaxDefinition {
    fn load_for_extension(
        path: &Path,
        extension: &str,
    ) -> io::Result<Option<Self>> {
        let contents = fs::read_to_string(path)?;

        Self::from_toml_for_extension(&contents, extension)
    }

    #[cfg(test)]
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

    fn from_toml_for_extension(
        contents: &str,
        extension: &str,
    ) -> io::Result<Option<Self>> {
        let file: SyntaxFileConfig = toml::from_str(contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if !file.syntax.extensions.iter().any(|value| {
            value.eq_ignore_ascii_case(extension.trim_start_matches('.'))
        }) {
            return Ok(None);
        }

        // Selecting a language must not compile other languages' regexes.
        Self::from_config(file.syntax).map(Some)
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
    // Earlier positions win, followed by longer matches at the same position.
    // Rule order breaks ties between identical spans.
    //

    pub fn matches_line(
        &self,
        line: &str,
    ) -> Vec<SyntaxMatch> {
        syntax_core::matches_line(line, self.rules.iter().map(|rule| (&rule.regex, rule.color)))
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

                let syntax = match Self::load_for_extension(&path, &extension) {
                    Ok(Some(syntax)) => syntax,
                    Ok(None) => continue,
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
        let syntax = Self::from_toml_for_extension(contents, &extension)
            .unwrap_or_else(|error| {
                panic!(
                    "embedded syntax definition {name} is invalid: {error}"
                )
            });

        if let Some(syntax) = syntax {
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

fn parse_color(value: &str) -> Color {
    let [r, g, b] = syntax_core::parse_rgb(value);
    Color::RGB(r, g, b)
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
    fn browser_download_loads_in_desktop_with_custom_colors() {
        // This fixture is an actual download from the designer, not handwritten TOML.
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tools/syntax-designer/fixtures/browser-export-python.toml");
        let syntax = SyntaxDefinition::load_for_extension(&path, "py").unwrap().unwrap();
        assert_eq!(syntax.name, "Python");
        assert_eq!(syntax.rules.len(), 7);
        assert!(syntax.supports_extension("pyi"));
        assert!(SyntaxDefinition::load_for_extension(&path, "cs").unwrap().is_none());
        let comment = "# café 🐈";
        let spans = syntax.matches_line(comment);
        assert_eq!(spans.len(), 1);
        assert_eq!((spans[0].start, spans[0].end), (0, comment.len()));
        assert_eq!(spans[0].color, Color::RGB(0xe8, 0x79, 0xf9));
        assert_token(&syntax, r#"name = "Ada""#, r#""Ada""#, "#CE9178");
    }

    fn embedded_for_extension(extension: &str) -> SyntaxDefinition {
        SYNTAX_DEFINITIONS.iter().find_map(|(_, contents)| {
            SyntaxDefinition::from_toml_for_extension(contents, extension).unwrap()
        }).unwrap_or_else(|| panic!("missing syntax for {extension}"))
    }

    fn assert_token(syntax: &SyntaxDefinition, line: &str, token: &str, color: &str) {
        let start = line.find(token).unwrap();
        let end = start + token.len();
        assert!(syntax.matches_line(line).iter().any(|span| {
            span.start <= start && span.end >= end && span.color == parse_color(color)
        }), "{}: {token:?} in {line:?}", syntax.name);
    }

    #[test]
    fn bundled_languages_highlight_representative_source() {
        for (extension, line, token, color) in [
            ("c", "int main(void) { return 42; }", "return", "#569CD6"),
            ("cpp", "template <typename T> class Box {};", "template", "#569CD6"),
            ("cs", "public record Person(string Name);", "record", "#569CD6"),
            ("py", "async def greet(name: str):", "async", "#569CD6"),
            ("php", "<?php ECHO $name;", "$name", "#9CDCFE"),
            ("ts", "interface Person { name: string }", "interface", "#569CD6"),
            ("go", "package main", "package", "#569CD6"),
            ("java", "public class Main {}", "class", "#569CD6"),
            ("sh", "if [ -n \"$HOME\" ]; then", "then", "#569CD6"),
            ("lua", "local value = nil", "local", "#569CD6"),
            ("json", "{\"enabled\": true}", "true", "#4FC1FF"),
            ("toml", "enabled = true", "enabled", "#9CDCFE"),
            ("yaml", "enabled: true", "enabled", "#9CDCFE"),
            ("html", "<div class=\"example\">Hello</div>", "<div", "#569CD6"),
            ("css", "body { color: #ffffff; }", "color:", "#9CDCFE"),
            ("sql", "SELECT name FROM people;", "SELECT", "#569CD6"),
        ] {
            let syntax = embedded_for_extension(extension);
            assert_token(&syntax, line, token, color);
        }
    }

    #[test]
    fn rust_labels_and_lifetimes_preserve_following_highlights() {
        let syntax = embedded_for_extension("rs");
        for (line, token) in [
            ("'running: loop {}", "'running"),
            ("'outer: loop { break 'outer; }", "'outer"),
            ("break 'running; let done = true;", "'running"),
            ("continue 'running; let done = true;", "'running"),
            ("fn borrow<'a>(value: &'a str) -> &'a str {}", "'a"),
            ("let value: &'static str = \"text\";", "'static"),
            ("let value: Thing<'_> = make();", "'_"),
        ] {
            let start = line.find(token).unwrap();
            assert!(syntax.matches_line(line).iter().any(|span|
                span.start == start && span.end == start + token.len()
                    && span.color == parse_color("#D7BA7D")), "{line}");
        }
        for (line, token, color) in [
            ("'running: loop {}", "loop", "#569CD6"),
            ("'running: loop { break 'running; }", "break", "#569CD6"),
            ("break 'running; let count = 42;", "let", "#569CD6"),
            ("break 'running; let count = 42;", "42", "#B5CEA8"),
            ("fn borrow<'a>(value: &'a str) -> &'a str {}", "str", "#4EC9B0"),
        ] {
            assert_token(&syntax, line, token, color);
        }
    }

    #[test]
    fn rust_character_literals_end_before_following_code() {
        let syntax = embedded_for_extension("rs");
        for literal in ["'a'", "'é'", "'🦀'", r"'\n'", r"'\\'", r"'\''",
            r"'\x7f'", r"'\u{1F980}'", r"'\u{1_F9_80}'", "b'a'", r"b'\xFF'"] {
            let line = format!("let character = {literal}; let count = 42;");
            let start = line.find(literal).unwrap();
            assert!(syntax.matches_line(&line).iter().any(|span|
                span.start == start && span.end == start + literal.len()
                    && span.color == parse_color("#CE9178")), "{line}");
            assert_token(&syntax, &line, "42", "#B5CEA8");
        }
        assert_token(&syntax, "'running: loop { let ch = 'x'; }", "loop", "#569CD6");
        assert_token(&syntax, "// 'running: loop {}", "// 'running: loop {}", "#6A9955");
        assert_token(&syntax, r#"let text = "'running: loop {}";"#,
            r#""'running: loop {}""#, "#CE9178");
    }

    #[test]
    fn common_languages_keep_strings_and_comments_distinct() {
        for (extension, line, token) in [
            ("cs", "var url = \"https://example.test\";", "\"https://example.test\""),
            ("py", "value = \"# not a comment\"", "\"# not a comment\""),
            ("php", "$value = \"# not a comment\";", "\"# not a comment\""),
            ("c", "char *url = \"https://example.test\";", "\"https://example.test\""),
        ] {
            assert_token(&embedded_for_extension(extension), line, token, "#CE9178");
        }
        for (extension, line) in [
            ("cs", "// return 42;"), ("py", "# return 42"),
            ("php", "// return 42;"), ("c", "/* return 42; */"),
        ] {
            assert_token(&embedded_for_extension(extension), line, line, "#6A9955");
        }
        assert_token(&embedded_for_extension("cs"), r#"var path = @"C:\Users\Name";"#,
            r#"@"C:\Users\Name""#, "#CE9178");
        assert_token(&embedded_for_extension("py"), "value = f\"Hello {name}\"",
            "f\"Hello {name}\"", "#CE9178");
    }

    #[test]
    fn embedded_extensions_are_unique_and_case_insensitive() {
        let mut extensions = std::collections::HashSet::new();
        for (_, contents) in SYNTAX_DEFINITIONS {
            let file: SyntaxFileConfig = toml::from_str(contents).unwrap();
            for extension in &file.syntax.extensions {
                assert!(extensions.insert(extension.to_ascii_lowercase()), "{extension}");
                let syntax = SyntaxDefinition::from_toml_for_extension(
                    contents, &format!(".{}", extension.to_ascii_uppercase()),
                ).unwrap().unwrap();
                assert!(syntax.supports_extension(extension));
            }
        }
        assert_eq!(embedded_for_extension("pyi").name, "Python");
        assert_eq!(embedded_for_extension("csx").name, "C#");
        assert_eq!(embedded_for_extension("mjs").name, "JavaScript");
        assert_eq!(embedded_for_extension("tsx").name, "TypeScript");
    }

    #[test]
    fn unrelated_language_regexes_are_not_compiled() {
        let contents = r##"
            [syntax]
            name = "Unrelated"
            extensions = ["other"]
            [[syntax.rules]]
            name = "invalid"
            pattern = "("
            color = "#ffffff"
        "##;
        assert!(SyntaxDefinition::from_toml_for_extension(contents, "py").unwrap().is_none());
        assert!(SyntaxDefinition::from_toml_for_extension(contents, "other").is_err());
    }

    #[test]
    fn highlighting_caps_long_lines_at_utf8_boundaries() {
        let syntax = embedded_for_extension("py");
        let line = format!("#{}é trailing", "a".repeat(MAX_HIGHLIGHT_BYTES - 2));
        let spans = syntax.matches_line(&line);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].end, MAX_HIGHLIGHT_BYTES - 1);
        assert!(line.is_char_boundary(spans[0].end));
    }

    #[test]
    fn highlighting_caps_dense_match_output() {
        let syntax = SyntaxDefinition::from_toml(r##"
            [syntax]
            name = "Dense"
            extensions = ["dense"]
            [[syntax.rules]]
            name = "character"
            pattern = "."
            color = "#ffffff"
        "##).unwrap();
        let line = "x".repeat(MAX_HIGHLIGHT_BYTES * 2);
        let spans = syntax.matches_line(&line);
        assert_eq!(spans.len(), MAX_HIGHLIGHT_MATCHES);
        assert_eq!(spans.last().unwrap().end, MAX_HIGHLIGHT_MATCHES);
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
