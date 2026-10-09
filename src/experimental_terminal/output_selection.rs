// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only output navigation. Anchors describe retained text, never pixels.
use super::{
    browser,
    transcript::{Anchor, Record, RecordData},
};
use std::{collections::VecDeque, io};

pub(crate) const MAX_COPY_BYTES: usize = 8 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 64 * 1024 + 2 + 255;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecordText {
    pub(crate) text: String,
    /// A terminal soft wrap continues in the next record without a newline.
    /// Sources must set false when the next record belongs to another source.
    pub(crate) join_next: bool,
    /// Completion metadata with no visible text is not a logical output row.
    pub(crate) omitted: bool,
    /// Native/header rows and completed command rows end with a newline.
    pub(crate) terminated: bool,
}

pub(crate) fn record_text(record: &Record) -> io::Result<RecordText> {
    let row = match &record.data {
        RecordData::Native(bytes) => {
            let mut text = browser::decode_native(bytes)?.text;
            let fixed = bytes[14] & 16 != 0;
            if fixed {
                text.extend(std::iter::repeat_n(' ', browser::native_padding(bytes)));
            }
            RecordText {
                text,
                join_next: fixed && bytes[14] & 8 != 0,
                omitted: false,
                terminated: !fixed || bytes[14] & 8 == 0,
            }
        }
        RecordData::Header(text) => RecordText {
            text: if text.starts_with("$ ")
                || text.starts_with("Commit ")
                    && text.contains(" · Back / Alt+Left returns to the log")
            {
                text.clone()
            } else {
                format!("$ {text}")
            },
            join_next: false,
            omitted: false,
            terminated: true,
        },
        RecordData::Result { status, text, .. } => RecordText {
            text: if *status == 0 {
                String::new()
            } else if *status == 130
                && (text.contains("Browsing stopped") || text == "[listing stopped]")
            {
                "[listing stopped]".into()
            } else if *status == 130 {
                "[command terminated]".into()
            } else {
                format!("[finished with exit code {status}]")
            },
            join_next: false,
            omitted: *status == 0,
            terminated: *status != 0,
        },
        RecordData::Terminal {
            cols: 0,
            wrapped,
            formatted,
        } => RecordText {
            text: super::output_colors::plain_text(formatted)?.into(),
            join_next: *wrapped,
            omitted: false,
            terminated: false,
        },
        RecordData::Terminal {
            cols,
            wrapped,
            formatted,
        } => {
            let mut parser = vt100::Parser::new(1, (*cols).clamp(1, 256), 0);
            parser.process(formatted);
            RecordText {
                text: parser.screen().contents(),
                join_next: *wrapped,
                omitted: false,
                terminated: false,
            }
        }
    };
    validate_text(&row)?;
    Ok(row)
}

/// The final successful completion retains plaintext's empty EOF row. Its
/// metadata disappears from presentation once a later record follows it.
pub(crate) fn record_text_at_end(record: &Record, at_eof: bool) -> io::Result<RecordText> {
    let mut row = record_text(record)?;
    if at_eof {
        if row.omitted {
            row.omitted = false;
        } else if matches!(record.data, RecordData::Result { .. }) {
            row.text.push('\n');
            row.terminated = false;
        }
    }
    Ok(row)
}

/// Physical soft wraps and fixed native columns join only the same source and
/// record type. Transient-tail sources apply the same provenance check.
pub(crate) fn joins_next(record: &Record, next: &Record) -> bool {
    record.epoch == next.epoch
        && record.source == next.source
        && record.id.checked_add(1) == Some(next.id)
        && match (&record.data, &next.data) {
            (
                RecordData::Terminal {
                    cols: left,
                    wrapped: true,
                    ..
                },
                RecordData::Terminal { cols: right, .. },
            ) => (*left == 0) == (*right == 0),
            (RecordData::Native(bytes), RecordData::Native(following)) => {
                bytes.len() >= 16
                    && following.len() >= 16
                    && bytes[14] & 24 == 24
                    && following[14] & 16 != 0
            }
            _ => false,
        }
}

pub(crate) fn terminal_row_text(screen: &vt100::Screen, row: u16) -> RecordText {
    let (_, cols) = screen.size();
    RecordText {
        text: screen.contents_between(row, 0, row, cols),
        join_next: screen.row_wrapped(row),
        omitted: false,
        terminated: false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub(crate) epoch: u64,
    pub(crate) first: u64,
    /// Exclusive end, including future IDs of the bounded transient tail.
    pub(crate) end: u64,
}

pub(crate) trait TextSource {
    fn bounds(&self) -> Bounds;
    fn text(&mut self, record_id: u64) -> io::Result<RecordText>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Visual {
    #[default]
    Off,
    Character,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Escape,
    A,
    C,
    H,
    J,
    K,
    L,
    W,
    B,
    V,
    Y,
    G,
    Zero,
    Dollar,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Modifiers {
    pub(crate) shift: bool,
    pub(crate) control: bool,
    pub(crate) gui: bool,
    pub(crate) alt: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Motion {
    PreviousCharacter,
    NextCharacter,
    PreviousWord,
    NextWord,
    LineStart,
    LineLastCharacter,
    First,
    End,
    LastLineStart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Request {
    None,
    Move { motion: Motion, extend: bool },
    Rows { delta: isize, extend: bool },
    Pages { delta: isize, extend: bool },
    VisualEdge { end: bool, extend: bool },
    SelectAll,
    Copy { all: bool, yank: bool },
    FocusPrompt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Range {
    pub(crate) start: Anchor,
    pub(crate) end: Anchor,
}
impl Range {
    pub(crate) fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Granularity {
    #[default]
    Character,
    Word,
    Line,
}

#[derive(Clone, Default)]
pub(crate) struct Selection {
    focused: bool,
    anchor: Option<Anchor>,
    cursor: Option<Anchor>,
    visual: Visual,
    pending_g: bool,
    whole_output: bool,
    desired_x: Option<i32>,
    mouse: Option<(Range, Granularity)>,
}

impl Selection {
    pub(crate) fn focused(&self) -> bool {
        self.focused
    }
    pub(crate) fn cursor(&self) -> Option<Anchor> {
        self.cursor
    }
    pub(crate) fn anchor(&self) -> Option<Anchor> {
        self.anchor
    }
    pub(crate) fn visual(&self) -> Visual {
        self.visual
    }
    pub(crate) fn desired_x(&self) -> Option<i32> {
        self.desired_x
    }
    pub(crate) fn set_desired_x(&mut self, x: i32) {
        self.desired_x = Some(x);
    }
    pub(crate) fn focus_prompt(&mut self) {
        *self = Self::default();
    }
    pub(crate) fn cancel_mouse(&mut self) {
        self.mouse = None;
    }

    /// Appending records and reflow do not change anchors. Eviction clips them
    /// to retained text, while an explicit Clear's epoch resets the selection.
    pub(crate) fn clamp(&mut self, source: &mut impl TextSource) -> io::Result<()> {
        let mut cache = Cache::new(source);
        if cache.bounds.first == cache.bounds.end {
            self.focus_prompt();
            return Ok(());
        }
        if self.cursor.is_some_and(|a| a.epoch != cache.bounds.epoch)
            || self.anchor.is_some_and(|a| a.epoch != cache.bounds.epoch)
        {
            self.focus_prompt();
            return Ok(());
        }
        let cursor = self.cursor.map(|a| cache.clamp(a)).transpose()?;
        let anchor = self.anchor.map(|a| cache.clamp(a)).transpose()?;
        if cursor != self.cursor || anchor != self.anchor {
            self.whole_output = false;
            self.mouse = None;
            self.desired_x = None;
        }
        self.cursor = cursor;
        self.anchor = anchor;
        Ok(())
    }

    pub(crate) fn apply(
        &mut self,
        source: &mut impl TextSource,
        target: Anchor,
        extend: bool,
    ) -> io::Result<()> {
        self.clamp(source)?;
        self.whole_output = false;
        let mut cache = Cache::new(source);
        if cache.bounds.first == cache.bounds.end {
            return Ok(());
        }
        let target = cache.clamp(target)?;
        if !extend && self.visual == Visual::Off || self.anchor.is_none() {
            self.anchor = Some(target);
        }
        self.cursor = Some(target);
        self.focused = true;
        self.desired_x = None;
        Ok(())
    }

    pub(crate) fn move_logical(
        &mut self,
        source: &mut impl TextSource,
        motion: Motion,
        extend: bool,
    ) -> io::Result<()> {
        self.clamp(source)?;
        let target = {
            let mut cache = Cache::new(source);
            if cache.bounds.first == cache.bounds.end {
                return Ok(());
            }
            let position = self.cursor.unwrap_or(cache.first()?);
            match motion {
                Motion::PreviousCharacter => cache.previous(position)?,
                Motion::NextCharacter => cache.next(position)?,
                Motion::PreviousWord => cache.word(position, false)?,
                Motion::NextWord => cache.word(position, true)?,
                Motion::LineStart => cache.line_start(position)?,
                Motion::LineLastCharacter => {
                    let end = cache.line_end(position)?;
                    let start = cache.line_start(position)?;
                    if end > start {
                        cache.previous(end)?
                    } else {
                        end
                    }
                }
                Motion::First => cache.first()?,
                Motion::End => cache.end()?,
                Motion::LastLineStart => {
                    let end = cache.end()?;
                    let last = cache.previous(end)?;
                    cache.line_start(last)?
                }
            }
        };
        self.apply(source, target, extend)
    }

    pub(crate) fn select_all(&mut self, source: &mut impl TextSource) -> io::Result<()> {
        let mut cache = Cache::new(source);
        if cache.bounds.first == cache.bounds.end {
            self.focus_prompt();
            return Ok(());
        }
        self.anchor = Some(cache.first()?);
        self.cursor = Some(cache.end()?);
        self.whole_output = true;
        self.focused = true;
        self.visual = Visual::Off;
        self.desired_x = None;
        self.mouse = None;
        Ok(())
    }

    pub(crate) fn range(&self, source: &mut impl TextSource) -> io::Result<Option<Range>> {
        let (Some(anchor), Some(cursor)) = (self.anchor, self.cursor) else {
            return Ok(None);
        };
        let mut cache = Cache::new(source);
        if cache.bounds.first == cache.bounds.end
            || anchor.epoch != cache.bounds.epoch
            || cursor.epoch != cache.bounds.epoch
        {
            return Ok(None);
        }
        let anchor = cache.clamp(anchor)?;
        let cursor = cache.clamp(cursor)?;
        let mut range = Range {
            start: anchor.min(cursor),
            end: anchor.max(cursor),
        };
        match self.visual {
            Visual::Off => {}
            Visual::Character => range.end = cache.next(range.end)?,
            Visual::Line => {
                range.start = cache.line_start(range.start)?;
                let end = cache.line_end(range.end)?;
                range.end = cache.next(end)?;
            }
        }
        Ok(Some(range))
    }

    pub(crate) fn mouse_begin(
        &mut self,
        source: &mut impl TextSource,
        target: Anchor,
        clicks: u8,
        extend: bool,
    ) -> io::Result<()> {
        self.whole_output = false;
        self.clamp(source)?;
        let mut cache = Cache::new(source);
        if cache.bounds.first == cache.bounds.end {
            return Ok(());
        }
        let target = cache.clamp(target)?;
        let granularity = match clicks {
            0 | 1 => Granularity::Character,
            2 => Granularity::Word,
            _ => Granularity::Line,
        };
        let base = match granularity {
            Granularity::Character => Range {
                start: target,
                end: target,
            },
            Granularity::Word => cache.word_range(target)?,
            Granularity::Line => cache.line_range(target)?,
        };
        self.visual = Visual::Off;
        self.focused = true;
        self.desired_x = None;
        if extend {
            let anchor = self.anchor.unwrap_or(base.start);
            self.anchor = Some(anchor);
            self.cursor = Some(if target < anchor {
                base.start
            } else {
                base.end
            });
            self.mouse = Some((
                Range {
                    start: anchor,
                    end: anchor,
                },
                granularity,
            ));
        } else {
            self.anchor = Some(base.start);
            self.cursor = Some(base.end);
            self.mouse = Some((base, granularity));
        }
        Ok(())
    }

    pub(crate) fn mouse_drag(
        &mut self,
        source: &mut impl TextSource,
        target: Anchor,
    ) -> io::Result<()> {
        self.clamp(source)?;
        let Some((base, granularity)) = self.mouse else {
            return Ok(());
        };
        let mut cache = Cache::new(source);
        let target = cache.clamp(target)?;
        let range = match granularity {
            Granularity::Character => Range {
                start: target,
                end: target,
            },
            Granularity::Word => cache.word_range(target)?,
            Granularity::Line => cache.line_range(target)?,
        };
        if target < base.start {
            self.anchor = Some(base.end);
            self.cursor = Some(range.start);
        } else {
            self.anchor = Some(base.start);
            self.cursor = Some(range.end);
        }
        Ok(())
    }

    pub(crate) fn key(&mut self, key: Key, modifiers: Modifiers, vim: bool) -> Request {
        let Modifiers {
            shift,
            control,
            gui,
            alt,
        } = modifiers;
        let pending_g = std::mem::take(&mut self.pending_g);
        if key == Key::Escape {
            self.focus_prompt();
            return Request::FocusPrompt;
        }
        if (control || gui) && key == Key::A {
            return Request::SelectAll;
        }
        if (control || gui) && key == Key::C {
            return Request::Copy {
                all: shift,
                yank: false,
            };
        }
        let plain_vim = vim && !control && !gui && !alt;
        let visual = self.visual != Visual::Off;
        let extend = visual || shift;
        if plain_vim {
            match key {
                Key::V => {
                    self.whole_output = false;
                    let mode = if shift {
                        Visual::Line
                    } else {
                        Visual::Character
                    };
                    self.visual = if self.visual == mode {
                        Visual::Off
                    } else {
                        mode
                    };
                    self.anchor = self.cursor;
                    return Request::None;
                }
                Key::Y => {
                    return Request::Copy {
                        all: false,
                        yank: true,
                    };
                }
                Key::G if shift || pending_g => {
                    return Request::Move {
                        motion: if shift {
                            Motion::LastLineStart
                        } else {
                            Motion::First
                        },
                        extend: visual,
                    };
                }
                Key::G => {
                    self.pending_g = true;
                    return Request::None;
                }
                _ => {}
            }
        }
        let word = (control || alt) && !gui;
        let motion = match key {
            Key::Up => return Request::Rows { delta: -1, extend },
            Key::Down => return Request::Rows { delta: 1, extend },
            Key::K if plain_vim => {
                return Request::Rows {
                    delta: -1,
                    extend: visual,
                };
            }
            Key::J if plain_vim => {
                return Request::Rows {
                    delta: 1,
                    extend: visual,
                };
            }
            Key::PageUp => return Request::Pages { delta: -1, extend },
            Key::PageDown => return Request::Pages { delta: 1, extend },
            Key::Home if control || gui => Motion::First,
            Key::End if control || gui => Motion::End,
            Key::Home => return Request::VisualEdge { end: false, extend },
            Key::End => return Request::VisualEdge { end: true, extend },
            Key::Left if word => Motion::PreviousWord,
            Key::Right if word => Motion::NextWord,
            Key::Left => Motion::PreviousCharacter,
            Key::Right => Motion::NextCharacter,
            Key::H if plain_vim => Motion::PreviousCharacter,
            Key::L if plain_vim => Motion::NextCharacter,
            Key::W if plain_vim => Motion::NextWord,
            Key::B if plain_vim => Motion::PreviousWord,
            Key::Zero if plain_vim && !shift => Motion::LineStart,
            Key::Dollar if plain_vim => Motion::LineLastCharacter,
            _ => return Request::None,
        };
        Request::Move {
            motion,
            extend: if plain_vim { visual } else { extend },
        }
    }

    /// Return a bounded clipboard payload; the caller updates SDL once only
    /// after success. Storage errors or a size limit never replace clipboard.
    pub(crate) fn copy(
        &mut self,
        source: &mut impl TextSource,
        all: bool,
        yank: bool,
    ) -> io::Result<String> {
        self.clamp(source)?;
        let range = if all {
            let mut cache = Cache::new(source);
            if cache.bounds.first == cache.bounds.end {
                return Ok(String::new());
            }
            Some(Range {
                start: cache.first()?,
                end: cache.end()?,
            })
        } else {
            self.range(source)?
        };
        let Some(range) = range else {
            return Ok(String::new());
        };
        let mut text = copy_range(source, range, MAX_COPY_BYTES)?;
        if all || self.whole_output || self.visual == Visual::Line {
            let mut cache = Cache::new(source);
            if range.end == cache.end()? && cache.row(range.end.record_id)?.terminated {
                append_checked(&mut text, "\n", MAX_COPY_BYTES)?;
            }
        }
        if yank {
            self.finish_yank(source)?;
        }
        Ok(text)
    }

    /// The UI can call copy(..., false) then this only after SDL accepted the
    /// payload, keeping visual selection intact if the clipboard fails.
    pub(crate) fn finish_yank(&mut self, source: &mut impl TextSource) -> io::Result<()> {
        if let Some(range) = self.range(source)? {
            self.visual = Visual::Off;
            self.whole_output = false;
            self.anchor = Some(range.start);
            self.cursor = Some(range.start);
            self.desired_x = None;
        }
        Ok(())
    }
}

pub(crate) fn copy_range(
    source: &mut impl TextSource,
    range: Range,
    limit: usize,
) -> io::Result<String> {
    let mut cache = Cache::new(source);
    if cache.bounds.first == cache.bounds.end {
        return Ok(String::new());
    }
    if range.start.epoch != cache.bounds.epoch || range.end.epoch != cache.bounds.epoch {
        return Err(io::Error::other(
            "Selection belongs to a cleared transcript",
        ));
    }
    let start = cache.clamp(range.start)?;
    let end = cache.clamp(range.end)?;
    if start >= end {
        return Ok(String::new());
    }
    let mut text = String::new();
    for id in start.record_id..=end.record_id {
        let row = cache.row(id)?;
        if row.omitted {
            continue;
        }
        let first = if id == start.record_id {
            start.utf8_byte_offset
        } else {
            0
        };
        let last = if id == end.record_id {
            end.utf8_byte_offset
        } else {
            row.text.len()
        };
        append_checked(&mut text, &row.text[first..last], limit)?;
        if id < end.record_id && !row.join_next {
            append_checked(&mut text, "\n", limit)?;
        }
    }
    Ok(text)
}

fn append_checked(text: &mut String, value: &str, limit: usize) -> io::Result<()> {
    let limit = limit.min(MAX_COPY_BYTES);
    let Some(length) = text
        .len()
        .checked_add(value.len())
        .filter(|len| *len <= limit)
    else {
        return Err(io::Error::other("Output exceeds the 8 MiB clipboard limit"));
    };
    if value.contains('\0') {
        return Err(io::Error::other("Output contains NUL and cannot be copied"));
    }
    if length > text.capacity() {
        // Vec's ordinary geometric growth can reserve beyond the byte cap
        // near the last append. Request bounded growth explicitly instead.
        let capacity = length.max(text.capacity().saturating_mul(2)).min(limit);
        text.reserve_exact(capacity - text.len());
    }
    text.push_str(value);
    Ok(())
}

fn validate_text(row: &RecordText) -> io::Result<()> {
    if row.text.len() > MAX_TEXT_BYTES {
        return Err(io::Error::other("Output row exceeds its text limit"));
    }
    Ok(())
}

struct Cache<'a, T: TextSource> {
    source: &'a mut T,
    bounds: Bounds,
    rows: VecDeque<(u64, RecordText)>,
}
impl<'a, T: TextSource> Cache<'a, T> {
    fn new(source: &'a mut T) -> Self {
        let bounds = source.bounds();
        Self {
            source,
            bounds,
            rows: VecDeque::with_capacity(2),
        }
    }
    fn row(&mut self, id: u64) -> io::Result<&RecordText> {
        if let Some(index) = self.rows.iter().position(|(saved, _)| *saved == id) {
            return Ok(&self.rows[index].1);
        }
        if id < self.bounds.first || id >= self.bounds.end {
            return Err(io::Error::other("Output anchor is outside retained text"));
        }
        let text = self.source.text(id)?;
        validate_text(&text)?;
        if self.rows.len() == 2 {
            self.rows.pop_front();
        }
        self.rows.push_back((id, text));
        Ok(&self.rows.back().unwrap().1)
    }
    fn at(&self, id: u64, offset: usize) -> Anchor {
        Anchor {
            epoch: self.bounds.epoch,
            record_id: id,
            utf8_byte_offset: offset,
        }
    }
    fn first(&mut self) -> io::Result<Anchor> {
        for id in self.bounds.first..self.bounds.end {
            if !self.row(id)?.omitted {
                return Ok(self.at(id, 0));
            }
        }
        Ok(self.at(self.bounds.first, 0))
    }
    fn end(&mut self) -> io::Result<Anchor> {
        for id in (self.bounds.first..self.bounds.end).rev() {
            let row = self.row(id)?;
            if !row.omitted {
                let len = row.text.len();
                return Ok(self.at(id, len));
            }
        }
        self.first()
    }
    fn next_visible(&mut self, id: u64) -> io::Result<Option<u64>> {
        for next in id.saturating_add(1)..self.bounds.end {
            if !self.row(next)?.omitted {
                return Ok(Some(next));
            }
        }
        Ok(None)
    }
    fn previous_visible(&mut self, id: u64) -> io::Result<Option<u64>> {
        for previous in (self.bounds.first..id).rev() {
            if !self.row(previous)?.omitted {
                return Ok(Some(previous));
            }
        }
        Ok(None)
    }
    fn clamp(&mut self, anchor: Anchor) -> io::Result<Anchor> {
        if anchor.epoch != self.bounds.epoch {
            return Err(io::Error::other(
                "Output target belongs to a cleared transcript",
            ));
        }
        if self.bounds.first == self.bounds.end || anchor.record_id < self.bounds.first {
            return self.first();
        }
        if anchor.record_id >= self.bounds.end {
            return self.end();
        }
        let row = self.row(anchor.record_id)?;
        if row.omitted {
            if let Some(id) = self.next_visible(anchor.record_id)? {
                return Ok(self.at(id, 0));
            }
            if let Some(id) = self.previous_visible(anchor.record_id)? {
                let len = self.row(id)?.text.len();
                return Ok(self.at(id, len));
            }
            return self.first();
        }
        let mut offset = anchor.utf8_byte_offset.min(row.text.len());
        while !row.text.is_char_boundary(offset) {
            offset -= 1;
        }
        Ok(self.at(anchor.record_id, offset))
    }
    fn next(&mut self, mut position: Anchor) -> io::Result<Anchor> {
        loop {
            let row = self.row(position.record_id)?;
            if row.omitted {
                return self.clamp(position);
            }
            if let Some(character) = row.text[position.utf8_byte_offset..].chars().next() {
                let offset = position.utf8_byte_offset + character.len_utf8();
                let joined = offset == row.text.len() && row.join_next;
                if joined && let Some(id) = self.next_visible(position.record_id)? {
                    return Ok(self.at(id, 0));
                }
                return Ok(self.at(position.record_id, offset));
            }
            let joined = row.join_next;
            let Some(id) = self.next_visible(position.record_id)? else {
                return Ok(position);
            };
            position = self.at(id, 0);
            if !joined {
                return Ok(position);
            }
        }
    }
    fn previous(&mut self, mut position: Anchor) -> io::Result<Anchor> {
        loop {
            let row = self.row(position.record_id)?;
            if row.omitted {
                return self.clamp(position);
            }
            if position.utf8_byte_offset > 0 {
                let offset = row.text[..position.utf8_byte_offset]
                    .char_indices()
                    .next_back()
                    .unwrap()
                    .0;
                return Ok(self.at(position.record_id, offset));
            }
            let Some(id) = self.previous_visible(position.record_id)? else {
                return Ok(position);
            };
            let row = self.row(id)?;
            let len = row.text.len();
            let joined = row.join_next;
            position = self.at(id, len);
            if !joined {
                return Ok(position);
            }
        }
    }
    fn character(&mut self, mut position: Anchor) -> io::Result<Option<char>> {
        loop {
            let row = self.row(position.record_id)?;
            if let Some(character) = row.text[position.utf8_byte_offset..].chars().next() {
                return Ok(Some(character));
            }
            let joined = row.join_next;
            let Some(id) = self.next_visible(position.record_id)? else {
                return Ok(None);
            };
            if !joined {
                return Ok(Some('\n'));
            }
            position = self.at(id, 0);
        }
    }
    fn class(&mut self, position: Anchor) -> io::Result<Option<u8>> {
        Ok(self.character(position)?.map(|c| {
            if c.is_whitespace() {
                0
            } else if c.is_alphanumeric() || c == '_' {
                1
            } else {
                2
            }
        }))
    }
    fn word(&mut self, mut position: Anchor, forward: bool) -> io::Result<Anchor> {
        if forward {
            let class = self.class(position)?;
            while self.class(position)? == class && class.is_some() {
                let next = self.next(position)?;
                if next == position {
                    break;
                }
                position = next;
            }
            while self.class(position)? == Some(0) {
                let next = self.next(position)?;
                if next == position {
                    break;
                }
                position = next;
            }
        } else {
            position = self.previous(position)?;
            while self.class(position)? == Some(0) {
                let previous = self.previous(position)?;
                if previous == position {
                    break;
                }
                position = previous;
            }
            let class = self.class(position)?;
            loop {
                let previous = self.previous(position)?;
                if previous == position || self.class(previous)? != class {
                    break;
                }
                position = previous;
            }
        }
        Ok(position)
    }
    fn line_start(&mut self, mut position: Anchor) -> io::Result<Anchor> {
        loop {
            let row = self.row(position.record_id)?;
            if let Some(offset) = row.text[..position.utf8_byte_offset].rfind('\n') {
                return Ok(self.at(position.record_id, offset + 1));
            }
            let Some(id) = self.previous_visible(position.record_id)? else {
                return Ok(self.at(position.record_id, 0));
            };
            let previous = self.row(id)?;
            if !previous.join_next {
                return Ok(self.at(position.record_id, 0));
            }
            let len = previous.text.len();
            position = self.at(id, len);
        }
    }
    fn line_end(&mut self, mut position: Anchor) -> io::Result<Anchor> {
        loop {
            let row = self.row(position.record_id)?;
            if let Some(offset) = row.text[position.utf8_byte_offset..].find('\n') {
                return Ok(self.at(position.record_id, position.utf8_byte_offset + offset));
            }
            let len = row.text.len();
            let joined = row.join_next;
            let next = self.next_visible(position.record_id)?;
            if !joined || next.is_none() {
                return Ok(self.at(position.record_id, len));
            }
            position = self.at(next.unwrap(), 0);
        }
    }
    fn line_range(&mut self, position: Anchor) -> io::Result<Range> {
        let start = self.line_start(position)?;
        let end = self.line_end(position)?;
        Ok(Range {
            start,
            end: self.next(end)?,
        })
    }
    fn word_range(&mut self, position: Anchor) -> io::Result<Range> {
        let class = self.class(position)?;
        let mut start = position;
        loop {
            let previous = self.previous(start)?;
            if previous == start || self.class(previous)? != class {
                break;
            }
            start = previous;
        }
        let mut end = position;
        while self.class(end)? == class && class.is_some() {
            let next = self.next(end)?;
            if next == end {
                break;
            }
            end = next;
        }
        Ok(Range { start, end })
    }
}

#[cfg(test)]
#[path = "output_selection_tests.rs"]
mod tests;
