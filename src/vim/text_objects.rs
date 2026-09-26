// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::{PieceTable, WordClass, word_class};
use sdl3::keyboard::{Keycode, Mod};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Word(bool),
    Pair(u8, u8),
    Quote(u8),
    Sentence,
    Paragraph,
    Tag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Object {
    pub kind: Kind,
    pub around: bool,
}

pub(super) fn from_key(key: Keycode, modifiers: Mod, around: bool) -> Option<Object> {
    let shifted = super::shift_pressed(modifiers);
    let kind = match key {
        Keycode::W => Kind::Word(shifted),
        Keycode::B => {
            if shifted {
                Kind::Pair(b'{', b'}')
            } else {
                Kind::Pair(b'(', b')')
            }
        }
        Keycode::S if !shifted => Kind::Sentence,
        Keycode::P if !shifted => Kind::Paragraph,
        Keycode::T if !shifted => Kind::Tag,
        Keycode::LeftParen | Keycode::RightParen => Kind::Pair(b'(', b')'),
        Keycode::_9 | Keycode::_0 if shifted => Kind::Pair(b'(', b')'),
        Keycode::LeftBrace | Keycode::RightBrace => Kind::Pair(b'{', b'}'),
        Keycode::LeftBracket | Keycode::RightBracket => {
            if shifted {
                Kind::Pair(b'{', b'}')
            } else {
                Kind::Pair(b'[', b']')
            }
        }
        Keycode::Less | Keycode::Greater => Kind::Pair(b'<', b'>'),
        Keycode::Comma | Keycode::Period if shifted => Kind::Pair(b'<', b'>'),
        Keycode::DblApostrophe => Kind::Quote(b'"'),
        Keycode::Apostrophe => Kind::Quote(if shifted { b'"' } else { b'\'' }),
        Keycode::Grave if !shifted => Kind::Quote(b'`'),
        _ => return None,
    };
    Some(Object { kind, around })
}

pub(super) type Range = (usize, usize, bool);

pub(super) fn range(
    table: &PieceTable,
    position: usize,
    object: Object,
    count: usize,
) -> Result<Option<Range>, String> {
    let mut scan = Scan::new(table);
    let count = count.max(1);
    let position = position.min(table.len());
    match object.kind {
        Kind::Word(big) => word(&mut scan, position, big, object.around, count),
        Kind::Pair(open, close) => pair(&mut scan, position, open, close, object.around, count),
        Kind::Quote(quote) => quoted(&mut scan, position, quote, object.around, count),
        Kind::Sentence => sentence(&mut scan, position, object.around, count),
        Kind::Paragraph => paragraph(&mut scan, position, object.around, count),
        Kind::Tag => tag(&mut scan, position, object.around, count),
    }
}

// One fixed-size page, reused in either direction. No whole-file/line copies,
// persistent parse trees, or work while the editor is idle.
struct Scan<'a> {
    table: &'a PieceTable,
    page: Vec<u8>,
    start: usize,
}
impl<'a> Scan<'a> {
    fn new(table: &'a PieceTable) -> Self {
        Self {
            table,
            page: Vec::new(),
            start: 0,
        }
    }
    fn len(&self) -> usize {
        self.table.len()
    }
    fn byte(&mut self, at: usize) -> Result<Option<u8>, String> {
        if at >= self.len() {
            return Ok(None);
        }
        if at < self.start || at >= self.start + self.page.len() {
            self.start = at / 8192 * 8192;
            self.page = self
                .table
                .read_range(self.start, (self.len() - self.start).min(8192))
                .map_err(|e| e.to_string())?;
        }
        Ok(Some(self.page[at - self.start]))
    }
    fn prev(&mut self, mut at: usize) -> Result<usize, String> {
        if at == 0 {
            return Ok(0);
        }
        at -= 1;
        while at > 0 && self.byte(at)?.is_some_and(|b| b & 0xc0 == 0x80) {
            at -= 1;
        }
        Ok(at)
    }
    fn character(&mut self, at: usize) -> Result<Option<char>, String> {
        let Some(first) = self.byte(at)? else {
            return Ok(None);
        };
        let len = if first < 128 {
            1
        } else if first < 224 {
            2
        } else if first < 240 {
            3
        } else {
            4
        };
        let mut bytes = [0; 4];
        for (i, byte) in bytes[..len].iter_mut().enumerate() {
            *byte = self.byte(at + i)?.ok_or("Incomplete UTF-8 character")?;
        }
        Ok(std::str::from_utf8(&bytes[..len])
            .map_err(|e| e.to_string())?
            .chars()
            .next())
    }
    fn next(&mut self, at: usize) -> Result<usize, String> {
        Ok(at + self.character(at)?.map_or(0, char::len_utf8))
    }
    fn space(&mut self, at: usize) -> Result<bool, String> {
        Ok(self.character(at)?.is_some_and(char::is_whitespace))
    }
    fn line_start(&mut self, mut at: usize) -> Result<usize, String> {
        while at > 0 && self.byte(at - 1)? != Some(b'\n') {
            at -= 1;
        }
        Ok(at)
    }
    fn line_end(&mut self, mut at: usize) -> Result<usize, String> {
        while at < self.len() && self.byte(at)? != Some(b'\n') {
            at += 1;
        }
        Ok(at)
    }
    fn in_quotes(&mut self, at: usize) -> Result<bool, String> {
        let mut quoted = false;
        for position in self.line_start(at)?..at {
            if self.byte(position)? == Some(b'"') && !self.escaped(position)? {
                quoted = !quoted;
            }
        }
        Ok(quoted)
    }
    fn escaped(&mut self, mut at: usize) -> Result<bool, String> {
        let mut odd = false;
        while at > 0 && self.byte(at - 1)? == Some(b'\\') {
            odd = !odd;
            at -= 1;
        }
        Ok(odd)
    }
}

fn class(scan: &mut Scan<'_>, at: usize, big: bool) -> Result<Option<WordClass>, String> {
    Ok(scan.character(at)?.map(|c| {
        if big && !c.is_whitespace() {
            WordClass::Keyword
        } else {
            word_class(c)
        }
    }))
}

fn run_end(scan: &mut Scan<'_>, mut at: usize, big: bool) -> Result<usize, String> {
    let wanted = class(scan, at, big)?;
    while at < scan.len() && class(scan, at, big)? == wanted {
        at = scan.next(at)?;
    }
    Ok(at)
}

fn word(
    scan: &mut Scan<'_>,
    position: usize,
    big: bool,
    around: bool,
    count: usize,
) -> Result<Option<Range>, String> {
    if position >= scan.len() {
        return Ok(None);
    }
    let wanted = class(scan, position, big)?;
    let mut start = position;
    while start > 0 {
        let previous = scan.prev(start)?;
        // Never take indentation from the previous line as leading whitespace.
        if scan.byte(previous)? == Some(b'\n') || class(scan, previous, big)? != wanted {
            break;
        }
        start = previous;
    }
    let mut end = run_end(scan, position, big)?;
    if !around {
        for _ in 1..count {
            if end >= scan.len() {
                break;
            }
            end = run_end(scan, end, big)?;
        }
    } else {
        let on_space = wanted == Some(WordClass::Whitespace);
        let mut remaining = count - usize::from(!on_space);
        while remaining > 0 && end < scan.len() {
            while end < scan.len() && scan.space(end)? {
                end = scan.next(end)?;
            }
            if end == scan.len() {
                break;
            }
            end = run_end(scan, end, big)?;
            remaining -= 1;
        }
        if !on_space {
            let after = end;
            while end < scan.len() && matches!(scan.byte(end)?, Some(b' ' | b'\t' | b'\r')) {
                end += 1;
            }
            if after == end {
                while start > 0 && matches!(scan.byte(start - 1)?, Some(b' ' | b'\t' | b'\r')) {
                    start -= 1;
                }
            }
        }
    }
    Ok(Some((start, end, false)))
}

fn pair(
    scan: &mut Scan<'_>,
    position: usize,
    open: u8,
    close: u8,
    around: bool,
    count: usize,
) -> Result<Option<Range>, String> {
    let mut at = position.min(scan.len().saturating_sub(1));
    let mut depth = 0usize;
    let mut remaining = count;
    let mut opening = None;
    loop {
        let byte = scan.byte(at)?;
        if (byte == Some(open) || byte == Some(close)) && !scan.escaped(at)? {
            if byte == Some(open) {
                if depth == 0 {
                    remaining -= 1;
                    if remaining == 0 {
                        opening = Some(at);
                        break;
                    }
                } else {
                    depth -= 1;
                }
            } else if scan.byte(at)? == Some(close) && at != position {
                depth += 1;
            }
        }
        if at == 0 {
            break;
        }
        at -= 1;
    }
    // Vim's inner-parenthesis object can find the next block from before it.
    if opening.is_none() && open == b'(' && !around && count == 1 {
        at = position;
        while at < scan.len() {
            if scan.byte(at)? == Some(open) && !scan.escaped(at)? {
                opening = Some(at);
                break;
            }
            at += 1;
        }
    }
    let Some(opening) = opening else {
        return Ok(None);
    };
    at = opening + 1;
    depth = 1;
    while at < scan.len() {
        let byte = scan.byte(at)?;
        if (byte == Some(open) || byte == Some(close)) && !scan.escaped(at)? {
            if byte == Some(open) {
                depth += 1;
            } else if byte == Some(close) {
                depth -= 1;
                if depth == 0 {
                    let quoted = scan.in_quotes(opening)?;
                    if quoted != scan.in_quotes(at)? || quoted && !scan.in_quotes(position)? {
                        return Ok(None);
                    }
                    if around {
                        return Ok(Some((opening, at + 1, false)));
                    }
                    let mut start = opening + 1;
                    let mut end = at;
                    {
                        if scan.byte(start)? == Some(b'\r') && scan.byte(start + 1)? == Some(b'\n')
                        {
                            start += 2;
                        } else if scan.byte(start)? == Some(b'\n') {
                            start += 1;
                        }
                        let close_line = scan.line_start(at)?;
                        let mut content = close_line;
                        while content < at
                            && matches!(scan.byte(content)?, Some(b' ' | b'\t' | b'\r'))
                        {
                            content += 1;
                        }
                        if content == at && close_line >= start {
                            end = close_line;
                        }
                    }
                    let linewise = start > opening + 1
                        && scan.line_start(start)? == start
                        && scan.line_start(end)? == end;
                    return Ok(Some((start, end, linewise)));
                }
            }
        }
        at += 1;
    }
    Ok(None)
}

fn quoted(
    scan: &mut Scan<'_>,
    position: usize,
    quote: u8,
    around: bool,
    count: usize,
) -> Result<Option<Range>, String> {
    let beginning = scan.line_start(position)?;
    let limit = scan.line_end(position)?;
    let mut opening = None;
    for at in beginning..limit {
        if scan.byte(at)? != Some(quote) || scan.escaped(at)? {
            continue;
        }
        if let Some(start) = opening.take() {
            if at < position {
                continue;
            }
            if !around {
                return Ok(Some(if count > 1 {
                    (start, at + 1, false)
                } else {
                    (start + 1, at, false)
                }));
            }
            let mut start = start;
            let mut end = at + 1;
            while end < limit && scan.space(end)? {
                end = scan.next(end)?;
            }
            if end == at + 1 {
                while start > beginning {
                    let previous = scan.prev(start)?;
                    if !scan.space(previous)? {
                        break;
                    }
                    start = previous;
                }
            }
            return Ok(Some((start, end, false)));
        } else {
            opening = Some(at);
        }
    }
    Ok(None)
}

fn blank_line(scan: &mut Scan<'_>, start: usize) -> Result<bool, String> {
    let mut at = start;
    while at < scan.len() && scan.byte(at)? != Some(b'\n') {
        if !scan.space(at)? {
            return Ok(false);
        }
        at = scan.next(at)?;
    }
    Ok(true)
}

fn paragraph_unit(scan: &mut Scan<'_>, position: usize) -> Result<(usize, usize, bool), String> {
    let mut start = scan.line_start(position)?;
    let blank = blank_line(scan, start)?;
    while start > 0 {
        let previous = scan.line_start(start - 1)?;
        if blank_line(scan, previous)? != blank {
            break;
        }
        start = previous;
    }
    let mut end = scan.line_end(position)?;
    if end < scan.len() {
        end += 1;
    }
    while end < scan.len() && blank_line(scan, end)? == blank {
        end = scan.line_end(end)?;
        if end < scan.len() {
            end += 1;
        }
    }
    Ok((start, end, blank))
}

fn paragraph(
    scan: &mut Scan<'_>,
    position: usize,
    around: bool,
    count: usize,
) -> Result<Option<Range>, String> {
    if scan.len() == 0 {
        return Ok(None);
    }
    let (mut start, mut end, blank) = paragraph_unit(scan, position)?;
    let mut remaining = count - 1;
    if around && blank {
        remaining += 1;
    }
    while remaining > 0 && end < scan.len() {
        let (_, next, whitespace) = paragraph_unit(scan, end)?;
        end = next;
        if !around || !whitespace {
            remaining -= 1;
        }
    }
    if around && !blank {
        if end < scan.len() {
            end = paragraph_unit(scan, end)?.1;
        } else if start > 0 {
            let (before, _, whitespace) = paragraph_unit(scan, start - 1)?;
            if whitespace {
                start = before;
            }
        }
    }
    Ok(Some((start, end, true)))
}

fn sentence_end(scan: &mut Scan<'_>, start: usize, limit: usize) -> Result<usize, String> {
    let mut at = start;
    while at < limit {
        if matches!(scan.byte(at)?, Some(b'.' | b'!' | b'?')) {
            let mut end = at + 1;
            while matches!(scan.byte(end)?, Some(b')' | b']' | b'"' | b'\'')) {
                end += 1;
            }
            if end >= limit || scan.space(end)? {
                return Ok(end);
            }
        }
        at = scan.next(at)?;
    }
    // The paragraph's newline is separating whitespace, not sentence content.
    while at > start {
        let previous = scan.prev(at)?;
        if !scan.space(previous)? {
            break;
        }
        at = previous;
    }
    Ok(at)
}

fn sentence(
    scan: &mut Scan<'_>,
    position: usize,
    around: bool,
    count: usize,
) -> Result<Option<Range>, String> {
    if position >= scan.len() {
        return Ok(None);
    }
    let (paragraph_start, limit, _) = paragraph_unit(scan, position)?;
    let mut start = paragraph_start;
    let mut end;
    loop {
        let before = start;
        while start < scan.len() && scan.space(start)? {
            start = scan.next(start)?;
        }
        if position < start {
            start = before;
            end = run_end(scan, before, true)?;
            if !around && count == 1 {
                return Ok(Some((start, end, false)));
            }
            break;
        }
        end = sentence_end(scan, start, limit)?;
        if position < end || end == scan.len() {
            break;
        }
        if end <= start {
            return Ok(None);
        }
        start = end;
    }
    let on_space = scan.space(start)?;
    if !around {
        for _ in 1..count {
            if end >= scan.len() {
                break;
            }
            if scan.space(end)? {
                while end < scan.len() && scan.space(end)? {
                    end = scan.next(end)?;
                }
            } else {
                let (_, limit, _) = paragraph_unit(scan, end)?;
                end = sentence_end(scan, end, limit)?;
            }
        }
        return Ok(Some((start, end, false)));
    }
    let mut remaining = count - usize::from(!on_space);
    while remaining > 0 && end < scan.len() {
        while end < scan.len() && scan.space(end)? {
            end = scan.next(end)?;
        }
        if end == scan.len() {
            break;
        }
        let (_, limit, _) = paragraph_unit(scan, end)?;
        end = sentence_end(scan, end, limit)?;
        remaining -= 1;
    }
    if around && !on_space {
        let after = end;
        while end < scan.len() && scan.space(end)? {
            end = scan.next(end)?;
        }
        if end == after {
            while start > paragraph_start {
                let previous = scan.prev(start)?;
                if !scan.space(previous)? {
                    break;
                }
                start = previous;
            }
        }
    }
    Ok(Some((start, end, false)))
}

struct Tag {
    start: usize,
    end: usize,
    name: Vec<u8>,
    closing: bool,
    empty: bool,
}
fn starts_with(scan: &mut Scan<'_>, at: usize, bytes: &[u8]) -> Result<bool, String> {
    for (offset, byte) in bytes.iter().enumerate() {
        if scan.byte(at + offset)? != Some(*byte) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn skip_until(scan: &mut Scan<'_>, mut at: usize, ending: &[u8]) -> Result<usize, String> {
    while at < scan.len() {
        if starts_with(scan, at, ending)? {
            return Ok(at + ending.len());
        }
        at += 1;
    }
    Ok(scan.len())
}
fn read_tag(scan: &mut Scan<'_>, start: usize) -> Result<(Option<Tag>, usize), String> {
    let closing = scan.byte(start + 1)? == Some(b'/');
    let mut at = start + 1 + usize::from(closing);
    if !scan
        .byte(at)?
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
    {
        return Ok((None, at + 1));
    }
    let mut name = Vec::new();
    while let Some(byte) = scan.byte(at)? {
        if !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'-' | b'.')) {
            break;
        }
        if name.len() == 256 {
            return Ok((None, scan.len()));
        }
        name.push(byte.to_ascii_lowercase());
        at += 1;
    }
    let mut quote = None;
    while let Some(byte) = scan.byte(at)? {
        if let Some(wanted) = quote {
            if byte == wanted {
                quote = None;
            }
        } else if matches!(byte, b'"' | b'\'') {
            quote = Some(byte);
        } else if byte == b'<' {
            return Ok((None, at));
        } else if byte == b'>' {
            let empty = scan.byte(at - 1)? == Some(b'/')
                || matches!(
                    name.as_slice(),
                    b"area"
                        | b"base"
                        | b"br"
                        | b"col"
                        | b"embed"
                        | b"hr"
                        | b"img"
                        | b"input"
                        | b"link"
                        | b"meta"
                        | b"param"
                        | b"source"
                        | b"track"
                        | b"wbr"
                );
            return Ok((
                Some(Tag {
                    start,
                    end: at + 1,
                    name,
                    closing,
                    empty,
                }),
                at + 1,
            ));
        }
        at += 1;
    }
    Ok((None, scan.len()))
}
fn tag(
    scan: &mut Scan<'_>,
    position: usize,
    around: bool,
    mut count: usize,
) -> Result<Option<Range>, String> {
    let mut stack: Vec<Tag> = Vec::new();
    let mut at = 0;
    while at < scan.len() {
        if scan.byte(at)? != Some(b'<') {
            at += 1;
            continue;
        }
        if starts_with(scan, at, b"<!--")? {
            at = skip_until(scan, at + 4, b"-->")?;
            continue;
        }
        if starts_with(scan, at, b"<![CDATA[")? {
            at = skip_until(scan, at + 9, b"]]>")?;
            continue;
        }
        if starts_with(scan, at, b"<?")? {
            at = skip_until(scan, at + 2, b"?>")?;
            continue;
        }
        if starts_with(scan, at, b"<!")? {
            at = skip_until(scan, at + 2, b">")?;
            continue;
        }
        let (tag, next) = read_tag(scan, at)?;
        at = next;
        let Some(tag) = tag else {
            continue;
        };
        if tag.empty {
            continue;
        }
        if !tag.closing {
            if stack.len() == 256 {
                return Ok(None);
            }
            stack.push(tag);
        } else if let Some(index) = stack.iter().rposition(|open| open.name == tag.name) {
            let open = stack.remove(index);
            stack.truncate(index);
            if open.start <= position && tag.end > position {
                count -= 1;
                if count == 0 {
                    return Ok(Some(if around {
                        (open.start, tag.end, false)
                    } else if open.end == tag.start {
                        (open.start, open.end, false)
                    } else {
                        (open.end, tag.start, false)
                    }));
                }
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_match_default_vim_ranges() {
        // Expected selections checked against Vim 9.1 with vim -Nu NONE.
        for (name, text, position, kind, around, count, expected) in [
            (
                "word",
                "one two three",
                5,
                Kind::Word(false),
                false,
                1,
                Some("two"),
            ),
            (
                "word-around",
                "one two three",
                5,
                Kind::Word(false),
                true,
                1,
                Some("two "),
            ),
            (
                "word2",
                "one two three",
                1,
                Kind::Word(false),
                false,
                2,
                Some("one "),
            ),
            (
                "word3",
                "one two three",
                1,
                Kind::Word(false),
                false,
                3,
                Some("one two"),
            ),
            (
                "around2",
                "one two three",
                1,
                Kind::Word(false),
                true,
                2,
                Some("one two "),
            ),
            (
                "white",
                "one   two",
                4,
                Kind::Word(false),
                false,
                1,
                Some("   "),
            ),
            (
                "white-around",
                "one   two",
                4,
                Kind::Word(false),
                true,
                1,
                Some("   two"),
            ),
            (
                "last-around",
                "one two",
                5,
                Kind::Word(false),
                true,
                1,
                Some(" two"),
            ),
            (
                "punct",
                "foo.bar",
                3,
                Kind::Word(false),
                false,
                1,
                Some("."),
            ),
            (
                "WORD",
                "foo.bar baz",
                4,
                Kind::Word(true),
                false,
                1,
                Some("foo.bar"),
            ),
            (
                "braces",
                "x {one {two} three} z",
                9,
                Kind::Pair(b'{', b'}'),
                false,
                1,
                Some("two"),
            ),
            (
                "outer",
                "x {one {two} three} z",
                9,
                Kind::Pair(b'{', b'}'),
                false,
                2,
                Some("one {two} three"),
            ),
            (
                "multi-braces",
                "fn() {\n  one\n  two\n}\nnext",
                11,
                Kind::Pair(b'{', b'}'),
                false,
                1,
                Some("  one\n  two\n"),
            ),
            (
                "multi-braces-indent",
                "fn() {\n  one\n  }\nnext",
                11,
                Kind::Pair(b'{', b'}'),
                false,
                1,
                Some("  one\n"),
            ),
            (
                "paren",
                "call(one, nested(two))",
                19,
                Kind::Pair(b'(', b')'),
                false,
                1,
                Some("two"),
            ),
            (
                "paren-next",
                "call(one)",
                0,
                Kind::Pair(b'(', b')'),
                false,
                1,
                Some("one"),
            ),
            (
                "multi-paren",
                "f(\n  one\n)",
                6,
                Kind::Pair(b'(', b')'),
                false,
                1,
                Some("  one\n"),
            ),
            (
                "multi-square",
                "[\n  one\n]",
                5,
                Kind::Pair(b'[', b']'),
                false,
                1,
                Some("  one\n"),
            ),
            (
                "empty",
                "foo() bar",
                4,
                Kind::Pair(b'(', b')'),
                false,
                1,
                Some(""),
            ),
            (
                "quotes",
                "say \"hello\" next",
                7,
                Kind::Quote(b'"'),
                false,
                1,
                Some("hello"),
            ),
            (
                "quotes-around",
                "say \"hello\" next",
                7,
                Kind::Quote(b'"'),
                true,
                1,
                Some("\"hello\" "),
            ),
            (
                "quotes2",
                "say \"hello\" next",
                7,
                Kind::Quote(b'"'),
                false,
                2,
                Some("\"hello\""),
            ),
            (
                "quotes-before",
                "say \"hello\" next",
                0,
                Kind::Quote(b'"'),
                false,
                1,
                Some("hello"),
            ),
            (
                "quotes-after",
                "say \"hello\" next",
                14,
                Kind::Quote(b'"'),
                false,
                1,
                None,
            ),
            (
                "sentence",
                "One thing.  Two things! Last?",
                5,
                Kind::Sentence,
                false,
                1,
                Some("One thing."),
            ),
            (
                "sentence2",
                "One thing.  Two things! Last?",
                5,
                Kind::Sentence,
                false,
                2,
                Some("One thing.  "),
            ),
            (
                "sentence-around",
                "One thing.  Two things! Last?",
                5,
                Kind::Sentence,
                true,
                1,
                Some("One thing.  "),
            ),
            (
                "sentence-space",
                "One thing.  Two things! Last?",
                10,
                Kind::Sentence,
                false,
                1,
                Some("  "),
            ),
            (
                "sentence-space2",
                "One thing.  Two things! Last?",
                10,
                Kind::Sentence,
                false,
                2,
                Some("  Two things!"),
            ),
            (
                "sentence-space-around",
                "One thing.  Two things! Last?",
                10,
                Kind::Sentence,
                true,
                1,
                Some("  Two things!"),
            ),
            (
                "paragraph",
                "one\nline\n\nnext\n\nlast\n",
                1,
                Kind::Paragraph,
                false,
                1,
                Some("one\nline\n"),
            ),
            (
                "paragraph2",
                "one\nline\n\nnext\n\nlast\n",
                1,
                Kind::Paragraph,
                false,
                2,
                Some("one\nline\n\n"),
            ),
            (
                "paragraph-around",
                "one\nline\n\nnext\n\nlast\n",
                1,
                Kind::Paragraph,
                true,
                1,
                Some("one\nline\n\n"),
            ),
            (
                "paragraphblank",
                "one\n\n\nnext\n\nlast\n",
                4,
                Kind::Paragraph,
                false,
                1,
                Some("\n\n"),
            ),
            (
                "paragraphblankaround",
                "one\n\n\nnext\n\nlast\n",
                4,
                Kind::Paragraph,
                true,
                1,
                Some("\n\nnext\n"),
            ),
            (
                "paragraphlast",
                "one\n\nlast\n",
                7,
                Kind::Paragraph,
                true,
                1,
                Some("\nlast\n"),
            ),
            (
                "tag",
                "<div><b>hi</b></div>",
                9,
                Kind::Tag,
                false,
                1,
                Some("hi"),
            ),
            (
                "tag2",
                "<div><b>hi</b></div>",
                9,
                Kind::Tag,
                false,
                2,
                Some("<b>hi</b>"),
            ),
            (
                "tag-around",
                "<div><b>hi</b></div>",
                9,
                Kind::Tag,
                true,
                1,
                Some("<b>hi</b>"),
            ),
        ] {
            let mut table = PieceTable::empty().unwrap();
            table.insert(0, text).unwrap();
            let actual = range(&table, position, Object { kind, around }, count)
                .unwrap()
                .map(|(start, end, _)| {
                    String::from_utf8(table.read_range(start, end - start).unwrap()).unwrap()
                });
            assert_eq!(actual.as_deref(), expected, "{name}");
        }
    }
    #[test]
    fn buffered_objects_handle_large_unicode_regions_and_malformed_tags() {
        let text = format!("({}é{})", "a".repeat(8190), "b".repeat(32000));
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, &text).unwrap();
        let mut scan = Scan::new(&table);
        assert_eq!(scan.character(8191).unwrap(), Some('é'));
        assert_eq!(scan.prev(8193).unwrap(), 8191);
        assert!(scan.page.len() <= 8192);
        let span = range(
            &table,
            20000,
            Object {
                kind: Kind::Pair(b'(', b')'),
                around: false,
            },
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!((span.0, span.1), (1, text.len() - 1));

        for text in ["<a broken=\"<a><a><a>", "<a>unclosed", "<a"]
            .iter()
            .map(|s| s.to_string())
            .chain([format!("{}value{}", "<a>".repeat(257), "</a>".repeat(257))])
        {
            let mut table = PieceTable::empty().unwrap();
            table.insert(0, &text).unwrap();
            assert!(
                range(
                    &table,
                    1,
                    Object {
                        kind: Kind::Tag,
                        around: false
                    },
                    1
                )
                .unwrap()
                .is_none()
            );
        }
    }

    #[test]
    fn tag_objects_skip_comments_void_tags_and_quoted_angle_brackets() {
        let text = "<DIV><!-- <div> --><br><span title='>'>hello</span><![CDATA[<div>]]></div>";
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, text).unwrap();
        for (count, expected) in [
            (1, "hello"),
            (
                2,
                "<!-- <div> --><br><span title='>'>hello</span><![CDATA[<div>]]>",
            ),
        ] {
            let (start, end, _) = range(
                &table,
                text.find("hello").unwrap(),
                Object {
                    kind: Kind::Tag,
                    around: false,
                },
                count,
            )
            .unwrap()
            .unwrap();
            assert_eq!(&text[start..end], expected);
        }
    }
}
