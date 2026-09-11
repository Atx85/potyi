//! Browser-only bridge to the same syntax schema and matcher used by Pötyi.
#[path = "../../../src/syntax_core.rs"]
mod syntax_core;

use regex::{Regex, RegexBuilder};
use std::cell::RefCell;
use std::fmt::Write;
use syntax_core::{SyntaxDefinitionConfig, SyntaxFileConfig};

const MAX_INPUT: usize = 64 * 1024;
struct Compiled {
    definition: SyntaxDefinitionConfig,
    rules: Vec<(Regex, [u8; 3])>,
}
thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
    static ACTIVE: RefCell<Option<Compiled>> = const { RefCell::new(None) };
}

fn quote(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\"' => result.push_str("\\\""), '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"), '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            ch if ch < ' ' => { write!(result, "\\u{:04x}", ch as u32).unwrap(); }
            ch => result.push(ch),
        }
    }
    result.push('"');
    result
}

fn configure(source: &str) -> Result<Compiled, String> {
    if source.len() > MAX_INPUT { return Err("Keep the definition under 64 KiB.".into()); }
    let file: SyntaxFileConfig = toml::from_str(source).map_err(|error| format!("Invalid TOML: {error}"))?;
    let definition = file.syntax;
    if definition.name.trim().is_empty() || definition.name.len() > 80 {
        return Err("Give the language a name of 1–80 bytes.".into());
    }
    if definition.extensions.is_empty() || definition.extensions.len() > 32 || definition.extensions.iter().any(|s| {
        s.is_empty() || s.len() > 24 || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b"_+-".contains(&b))
    }) {
        return Err("Use 1–32 extensions, without dots, spaces, or filenames (for example: cs, csx).".into());
    }
    if definition.rules.is_empty() || definition.rules.len() > 32 {
        return Err("Use between 1 and 32 highlighting rules.".into());
    }
    let mut rules = Vec::new();
    for (index, rule) in definition.rules.iter().enumerate() {
        let prefix = format!("Rule {} ({})", index + 1, rule.name);
        if rule.name.trim().is_empty() || rule.name.len() > 80 {
            return Err(format!("{prefix}: give the rule a name of 1–80 bytes."));
        }
        if rule.pattern.is_empty() || rule.pattern.len() > 4096 {
            return Err(format!("{prefix}: use a pattern of 1–4,096 bytes."));
        }
        if rule.color.len() != 7 || !rule.color.starts_with('#') || !rule.color[1..].bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("{prefix}: use a six-digit color such as #83d6a3."));
        }
        let regex = RegexBuilder::new(&rule.pattern).size_limit(1024 * 1024).dfa_size_limit(128 * 1024)
            .build().map_err(|error| format!("{prefix}: {error}"))?;
        rules.push((regex, syntax_core::parse_rgb(&rule.color)));
    }
    Ok(Compiled { definition, rules })
}

fn definition_json(definition: &SyntaxDefinitionConfig) -> String {
    let extensions = definition.extensions.iter().map(|s| quote(s)).collect::<Vec<_>>().join(",");
    let rules = definition.rules.iter().map(|rule| format!("{{\"name\":{},\"pattern\":{},\"color\":{}}}",
        quote(&rule.name), quote(&rule.pattern), quote(&rule.color))).collect::<Vec<_>>().join(",");
    format!("{{\"name\":{},\"extensions\":[{extensions}],\"rules\":[{rules}]}}", quote(&definition.name))
}

fn respond(result: Result<String, String>) {
    OUTPUT.with(|output| *output.borrow_mut() = match result {
        Ok(value) => value,
        Err(error) => format!("{{\"error\":{}}}", quote(&error)),
    });
}

// A single bounded input buffer avoids allocation ownership crossing the JS ABI.
#[unsafe(no_mangle)]
pub extern "C" fn input_buffer(length: usize) -> *mut u8 {
    if length > MAX_INPUT { return std::ptr::null_mut(); }
    INPUT.with(|input| { let mut input = input.borrow_mut(); input.resize(length, 0); input.as_mut_ptr() })
}
#[unsafe(no_mangle)]
pub extern "C" fn output_ptr() -> *const u8 { OUTPUT.with(|output| output.borrow().as_ptr()) }
#[unsafe(no_mangle)]
pub extern "C" fn output_len() -> usize { OUTPUT.with(|output| output.borrow().len()) }

#[unsafe(no_mangle)]
pub extern "C" fn configure_input() {
    ACTIVE.with(|active| *active.borrow_mut() = None);
    respond(INPUT.with(|input| {
        let input = input.borrow();
        let source = std::str::from_utf8(&input).map_err(|_| "The definition must be UTF-8.".to_string())?;
        let compiled = configure(source)?;
        let json = definition_json(&compiled.definition);
        ACTIVE.with(|active| *active.borrow_mut() = Some(compiled));
        Ok(json)
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn highlight_input() {
    respond(INPUT.with(|input| {
        let input = input.borrow();
        let source = std::str::from_utf8(&input).map_err(|_| "Sample text must be UTF-8.".to_string())?;
        if source.len() > 16 * 1024 { return Err("Keep the preview sample under 16 KiB.".into()); }
        ACTIVE.with(|active| {
            let active = active.borrow();
            let compiled = active.as_ref().ok_or("Fix the definition before previewing.".to_string())?;
            let mut output = String::from("{\"lines\":[");
            for (line_index, line) in source.split('\n').take(200).enumerate() {
                if line_index > 0 { output.push(','); }
                output.push('[');
                for (index, span) in syntax_core::matches_line(line, compiled.rules.iter().map(|(r, c)| (r, *c))).iter().enumerate() {
                    if index > 0 { output.push(','); }
                    write!(output, "[{},{},\"#{:02x}{:02x}{:02x}\"]", span.start, span.end, span.color[0], span.color[1], span.color[2]).unwrap();
                }
                output.push(']');
            }
            output.push_str("]}");
            Ok(output)
        })
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_bundled_definition_is_accepted() {
        for entry in std::fs::read_dir("../../config/syntax").unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                configure(&std::fs::read_to_string(&path).unwrap()).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            }
        }
    }
    #[test]
    fn json_strings_escape_controls_and_preserve_unicode() {
        assert_eq!(quote("é\n\"\\\0"), "\"é\\n\\\"\\\\\\u0000\"");
    }
}
