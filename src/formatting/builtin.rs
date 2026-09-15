// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conservative indentation, with file-backed input/output and bounded scratch
//! space. This is not a parser or a replacement for a language formatter.

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;

use super::{MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, Scratch, invalid};
use crate::piece_table::PieceTable;

const MAX_LINE: usize = 64 * 1024;
const MAX_DEPTH: usize = 256;

pub(super) fn supported(path: Option<&Path>) -> bool {
    path.is_none_or(|path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "c" | "h"
                        | "cc"
                        | "cpp"
                        | "cxx"
                        | "hh"
                        | "hpp"
                        | "hxx"
                        | "m"
                        | "mm"
                        | "java"
                        | "cs"
                        | "proto"
                        | "json"
                        | "jsonc"
                )
            })
    })
}

pub(crate) fn run(
    path: Option<&Path>,
    document: &PieceTable,
    scratch: &Scratch,
    tab_width: usize,
    insert_spaces: bool,
) -> io::Result<File> {
    if !supported(path) {
        return Err(invalid(
            "Built-in indentation supports C/C++, C#, Java, Objective-C, Protobuf, JSON and untitled code. For this file type, use :formatters to choose a language formatter.",
        ));
    }
    if document.len() > MAX_INPUT_BYTES {
        return Err(invalid("Formatting is limited to documents up to 2 MiB"));
    }
    let mut input = scratch.file("builtin-input")?;
    document.visit_chunks(|bytes| input.write_all(bytes))?;
    input.rewind()?;
    let mut output = scratch.file("builtin-output")?;
    let json = path
        .and_then(Path::extension)
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("jsonc"));
    indent(
        BufReader::new(input),
        &mut output,
        tab_width,
        insert_spaces,
        json,
    )?;
    output.rewind()?;
    Ok(output)
}

#[derive(Default, PartialEq)]
enum Lex {
    #[default]
    Code,
    Comment,
    Quote(u8),
    Verbatim,
    Raw(usize),
    CppRaw(Vec<u8>),
}

#[derive(Default)]
struct Scanner {
    lex: Lex,
    stack: Vec<u8>,
    directive: bool,
    json: bool,
}

impl Scanner {
    fn scan(&mut self, line: &[u8]) -> io::Result<()> {
        let mut i = 0;
        while i < line.len() {
            let rest = &line[i..];
            let byte = line[i];
            match &self.lex {
                Lex::Comment => {
                    if rest.starts_with(b"*/") {
                        self.lex = Lex::Code;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                Lex::Quote(quote) => {
                    if byte == b'\\' {
                        i += 2;
                    } else if byte == *quote {
                        self.lex = Lex::Code;
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Lex::Verbatim => {
                    if rest.starts_with(b"\"\"") {
                        i += 2;
                    } else if byte == b'"' {
                        self.lex = Lex::Code;
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Lex::Raw(count) => {
                    let quotes = rest.iter().take_while(|&&b| b == b'"').count();
                    if quotes >= *count {
                        i += *count;
                        self.lex = Lex::Code;
                    } else {
                        i += quotes.max(1);
                    }
                }
                Lex::CppRaw(end) => {
                    if rest.starts_with(end) {
                        i += end.len();
                        self.lex = Lex::Code;
                    } else {
                        i += 1;
                    }
                }
                Lex::Code => {
                    if !self.json
                        && (rest.starts_with(b"$\"")
                            || rest.starts_with(b"$@\"")
                            || rest.starts_with(b"@$\""))
                    {
                        return Err(invalid(
                            "Built-in indentation cannot safely handle interpolated strings yet; use :formatters. Document unchanged.",
                        ));
                    } else if rest.starts_with(b"//") {
                        // A backslash continues a C/C++ comment onto the next
                        // physical line. Refuse it instead of treating its text as code.
                        if line.ends_with(b"\\") {
                            return Err(invalid(
                                "Continued line comments need a language formatter; use :formatters. Document unchanged.",
                            ));
                        }
                        break;
                    } else if rest.starts_with(b"/*") {
                        self.lex = Lex::Comment;
                        i += 2;
                    } else if !self.json && rest.starts_with(b"@\"") {
                        self.lex = Lex::Verbatim;
                        i += 2;
                    } else if !self.json && rest.starts_with(b"R\"") {
                        let Some(end) = rest[2..]
                            .iter()
                            .position(|&b| b == b'(')
                            .filter(|&n| n <= 16)
                        else {
                            return Err(invalid(
                                "Unrecognized raw string; use :formatters. Document unchanged.",
                            ));
                        };
                        let mut closing = vec![b')'];
                        closing.extend_from_slice(&rest[2..2 + end]);
                        closing.push(b'"');
                        self.lex = Lex::CppRaw(closing);
                        i += end + 3;
                    } else if byte == b'"' {
                        let count = rest.iter().take_while(|&&b| b == b'"').count();
                        if !self.json && count >= 3 {
                            self.lex = Lex::Raw(count);
                            i += count;
                        } else {
                            self.lex = Lex::Quote(byte);
                            i += 1;
                        }
                    } else if !self.json && byte == b'\'' {
                        // C++ numeric separators are not character literals.
                        if i > 0
                            && line[i - 1].is_ascii_hexdigit()
                            && rest.get(1).is_some_and(u8::is_ascii_hexdigit)
                        {
                            i += 1;
                        } else {
                            self.lex = Lex::Quote(byte);
                            i += 1;
                        }
                    } else if b"{[(".contains(&byte) {
                        if self.stack.len() == MAX_DEPTH {
                            return Err(invalid(
                                "Code is nested too deeply for built-in indentation",
                            ));
                        }
                        self.stack.push(byte);
                        i += 1;
                    } else if let Some(index) = b"}])".iter().position(|&b| b == byte) {
                        if self.stack.pop() != Some(b"{[("[index]) {
                            return Err(invalid(
                                "Unmatched closing bracket; fix it or use :formatters. Document unchanged.",
                            ));
                        }
                        i += 1;
                    } else if byte == b'\\' {
                        return Err(invalid(
                            "Escapes outside strings need a language formatter; use :formatters. Document unchanged.",
                        ));
                    } else {
                        i += 1;
                    }
                }
            }
        }
        Ok(())
    }
}

fn indent(
    mut input: impl BufRead,
    output: impl Write,
    width: usize,
    spaces: bool,
    json: bool,
) -> io::Result<()> {
    if !(1..=32).contains(&width) {
        return Err(invalid(
            "Built-in indentation needs tab_width between 1 and 32",
        ));
    }
    let mut output = BufWriter::new(output);
    let mut scanner = Scanner {
        json,
        ..Scanner::default()
    };
    let mut line = Vec::with_capacity(MAX_LINE + 1);
    let mut written = 0usize;
    let mut first = true;
    loop {
        line.clear();
        let size = input
            .by_ref()
            .take((MAX_LINE + 1) as u64)
            .read_until(b'\n', &mut line)?;
        if size == 0 {
            break;
        }
        if size > MAX_LINE {
            return Err(invalid(
                "Built-in indentation supports lines up to 64 KiB; document unchanged",
            ));
        }
        let bom = if first && line.starts_with(b"\xef\xbb\xbf") {
            3
        } else {
            0
        };
        first = false;
        let mut end = line.len();
        if line.get(end.wrapping_sub(1)) == Some(&b'\n') {
            end -= 1;
        }
        if line.get(end.wrapping_sub(1)) == Some(&b'\r') {
            end -= 1;
        }
        let leading = line[bom..end]
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count()
            + bom;
        let code = &line[leading..end];
        let directive =
            !json && scanner.lex == Lex::Code && (scanner.directive || code.starts_with(b"#"));
        let preserve = scanner.lex != Lex::Code || directive || code.is_empty();
        let mut depth = scanner.stack.len();
        if !preserve {
            for &byte in code {
                if b"}])".contains(&byte) {
                    depth = depth.saturating_sub(1);
                } else if byte != b' ' && byte != b'\t' {
                    break;
                }
            }
        }
        let indent_len = if spaces { depth * width } else { depth };
        let length = if preserve {
            line.len()
        } else {
            bom + indent_len + line.len() - leading
        };
        written = written.saturating_add(length);
        if written > MAX_OUTPUT_BYTES {
            return Err(invalid("Indented output exceeds 4 MiB; document unchanged"));
        }
        if preserve {
            output.write_all(&line)?;
        } else {
            output.write_all(&line[..bom])?;
            let padding = [if spaces { b' ' } else { b'\t' }; 256];
            let mut remaining = indent_len;
            while remaining > 0 {
                let count = remaining.min(padding.len());
                output.write_all(&padding[..count])?;
                remaining -= count;
            }
            output.write_all(&line[leading..])?;
        }
        if directive {
            scanner.directive = code.ends_with(b"\\");
        } else {
            scanner.scan(&line[bom..end])?;
        }
    }
    if scanner.lex != Lex::Code {
        return Err(invalid(
            "Unclosed string or comment; document unchanged. Finish it or use :formatters.",
        ));
    }
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formatted(text: &str, spaces: bool, json: bool) -> io::Result<String> {
        let mut output = Vec::new();
        indent(io::Cursor::new(text), &mut output, 2, spaces, json)?;
        Ok(String::from_utf8(output).unwrap())
    }

    #[test]
    fn nested_code_tabs_spaces_crlf_and_bom() {
        let source = "\u{feff}class Example {\r\nvoid OnEnable() {\r\nCall(\r\n1,\r\n2);\r\n}\r\n}";
        let expected = "\u{feff}class Example {\r\n  void OnEnable() {\r\n    Call(\r\n      1,\r\n      2);\r\n  }\r\n}";
        assert_eq!(formatted(source, true, false).unwrap(), expected);
        assert_eq!(formatted(expected, true, false).unwrap(), expected);
        assert_eq!(
            formatted("{\nx();\n}\n", false, false).unwrap(),
            "{\n\tx();\n}\n"
        );
    }

    #[test]
    fn strings_comments_directives_and_raw_contents_are_preserved() {
        let source = "{\nconst char *s = \"}\\\"{\"; // }\n/* {\n   comment }\n */\n#define BRACE { \\\n  {\nvar s = @\"hi\n  } doubled \"\" quotes\nbye\";\nvar raw = \"\"\"\n  { raw\n  \"\"\";\nauto cpp = R\"tag(\n } \" {\n)tag\";\nx();\n}\n";
        let result = formatted(source, true, false).unwrap();
        assert!(result.contains("\n  const char *s"));
        assert!(result.contains("\n   comment }\n */\n#define BRACE { \\\n  {\n"));
        assert!(result.contains("@\"hi\n  } doubled \"\" quotes\nbye\";"));
        assert!(result.contains("\"\"\"\n  { raw\n  \"\"\";"));
        assert!(result.contains("R\"tag(\n } \" {\n)tag\";"));
        assert!(result.ends_with("\n  x();\n}\n"));
    }

    #[test]
    fn json_quotes_blank_lines_and_final_newline_are_preserved() {
        let source = "{\n\"a\": [\n\"{}[]\",\n\"\",\n1\n]\n   \n}";
        assert_eq!(
            formatted(source, true, true).unwrap(),
            "{\n  \"a\": [\n    \"{}[]\",\n    \"\",\n    1\n  ]\n   \n}"
        );
        assert_eq!(formatted("", true, false).unwrap(), "");
    }

    #[test]
    fn unsupported_and_ambiguous_input_is_rejected() {
        assert!(!supported(Some(Path::new("script.py"))));
        assert!(!supported(Some(Path::new("Makefile"))));
        assert!(supported(Some(Path::new("Example.CS"))));
        assert!(supported(None));
        for input in [
            "}",
            "{]",
            "\"unclosed",
            "/* unclosed",
            "$\"{x}\"",
            "// comment\\\n{",
            "x \\\n{",
        ] {
            assert!(formatted(input, true, false).is_err(), "{input}");
        }
        assert!(formatted(&"x".repeat(MAX_LINE + 1), true, false).is_err());
        assert!(formatted(&"{".repeat(MAX_DEPTH + 1), true, false).is_err());
    }
}
