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


// Shared by the desktop editor and the browser designer. Keep this module
// independent of SDL, filesystem access, and browser APIs.
use regex::Regex;
use serde::Deserialize;

pub const MAX_HIGHLIGHT_BYTES: usize = 16 * 1024;
pub const MAX_HIGHLIGHT_MATCHES: usize = 1024;

#[derive(Debug, Deserialize)]
pub struct SyntaxFileConfig {
    pub syntax: SyntaxDefinitionConfig,
}

#[derive(Debug, Deserialize)]
pub struct SyntaxDefinitionConfig {
    pub name: String,
    pub extensions: Vec<String>,
    pub rules: Vec<SyntaxRuleConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SyntaxRuleConfig {
    pub name: String,
    pub pattern: String,
    pub color: String,
}


#[derive(Debug, Clone, Copy)]
pub struct SyntaxMatch<C> {
    pub start: usize,
    pub end: usize,
    pub color: C,
}

pub fn matches_line<'a, C: Copy + 'a>(
    line: &str,
    rules: impl IntoIterator<Item = (&'a Regex, C)>,
) -> Vec<SyntaxMatch<C>> {
    let mut end = line.len().min(MAX_HIGHLIGHT_BYTES);
    while !line.is_char_boundary(end) { end -= 1; }
    let line = &line[..end];
    let mut matches = Vec::new();
    'rules: for (regex, color) in rules {
        for found in regex.find_iter(line) {
            if found.is_empty() { continue; }
            matches.push(SyntaxMatch { start: found.start(), end: found.end(), color });
            if matches.len() == MAX_HIGHLIGHT_MATCHES { break 'rules; }
        }
    }
    resolve_overlaps(matches)
}

pub fn resolve_overlaps<C>(mut matches: Vec<SyntaxMatch<C>>) -> Vec<SyntaxMatch<C>> {
    // Stable sorting keeps the earlier rule for identical spans.
    matches.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));
    let mut result: Vec<SyntaxMatch<C>> = Vec::with_capacity(matches.len());
    for current in matches {
        if result.last().is_some_and(|previous| previous.end > current.start) { continue; }
        result.push(current);
    }
    result
}

pub fn parse_rgb(value: &str) -> [u8; 3] {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 { return [220; 3]; }
    std::array::from_fn(|index| {
        value.get(index * 2..index * 2 + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok()).unwrap_or(220)
    })
}
