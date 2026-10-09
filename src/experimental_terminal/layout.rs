// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded projection of the disk transcript. Sparse checkpoints are transient;
//! logical anchors and native paths remain in the one shared transcript file.
use super::{
    Wake,
    browser::{Kind, decode_native, native_entry_range, native_packing},
    output_colors::{self, Decoration},
    output_selection::{record_text, terminal_row_text},
    transcript::{Anchor, Record, RecordData, SharedTranscript},
};
use std::{
    io, mem,
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};
use unicode_width::UnicodeWidthChar;
#[cfg(test)]
use unicode_width::UnicodeWidthStr;

const CHECKPOINTS: usize = 1024;
const MAX_VISIBLE_BYTES: usize = 256 * 1024;
const MAX_FRAGMENT_BYTES: usize = 64 * 1024;
const MAX_FRAGMENT_SEGMENTS: usize = 1024;
const MAX_PLAIN_TAIL_BYTES: usize = 60 * 1024;

fn column_width(character: char, font_columns: bool) -> usize {
    let width = character.width().unwrap_or(0);
    if font_columns {
        usize::from(width != 0)
    } else {
        width
    }
}

fn text_width(text: &str, font_columns: bool) -> usize {
    text.chars()
        .map(|character| column_width(character, font_columns))
        .sum()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Metrics {
    pub(crate) columns: u16,
    pub(crate) tab_width: usize,
}
impl Default for Metrics {
    fn default() -> Self {
        Self {
            columns: 80,
            tab_width: 4,
        }
    }
}
impl Metrics {
    fn bounded(self) -> Self {
        Self {
            columns: self.columns.clamp(1, 1000),
            tab_width: self.tab_width.clamp(1, 256),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PixelMetrics {
    width: usize,
    cell: usize,
    font_key: u32,
}
#[derive(Default)]
struct ScalarWidths {
    cached: Vec<(char, usize)>,
    needed: bool,
}
impl ScalarWidths {
    fn width(
        &mut self,
        character: char,
        pixels: Option<PixelMetrics>,
        font_columns: bool,
        measure: &mut Option<&mut dyn FnMut(char) -> i32>,
    ) -> Option<usize> {
        let Some(pixels) = pixels else {
            return Some(column_width(character, font_columns));
        };
        let cells = column_width(character, font_columns);
        if !font_columns || cells != 0 {
            return Some(cells.saturating_mul(pixels.cell));
        }
        if let Some((_, width)) = self.cached.iter().find(|(old, _)| *old == character) {
            return Some(*width);
        }
        let Some(measure) = measure.as_deref_mut() else {
            self.needed = true;
            return None;
        };
        let width = measure(character).max(0) as usize;
        if self.cached.len() == 512 {
            self.cached.remove(0);
        }
        self.cached.push((character, width));
        Some(width)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Style {
    pub(crate) fg: vt100::Color,
    pub(crate) bg: vt100::Color,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) dim: bool,
    pub(crate) inverse: bool,
    pub(crate) underline: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Segment {
    pub(crate) range: Range<usize>,
    pub(crate) start: Anchor,
    pub(crate) end: Anchor,
    pub(crate) column: u16,
    pub(crate) record_id: u64,
    pub(crate) source: u64,
    pub(crate) native: bool,
    pub(crate) color: Option<(u8, u8, u8)>,
    pub(crate) style: Style,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fragment {
    pub(crate) start: Anchor,
    pub(crate) end: Anchor,
    pub(crate) text: String,
    pub(crate) segments: Vec<Segment>,
    font_columns: bool,
}
impl Fragment {
    /// The embedded editor font uses one advance for missing wide glyphs.
    /// Raw VT grids retain Unicode cell widths; normal output uses font columns.
    pub(crate) fn text_columns(&self, text: &str) -> usize {
        text_width(text, self.font_columns)
    }
    fn bytes(&self) -> usize {
        mem::size_of::<Self>()
            + self.text.capacity()
            + self.segments.capacity() * mem::size_of::<Segment>()
    }
    fn trim(&mut self, length: usize, end: Anchor) {
        self.text.truncate(length);
        self.segments.retain(|segment| segment.range.start < length);
        if let Some(segment) = self.segments.last_mut() {
            if segment.range.end > length {
                let removed = segment.range.end - length;
                segment.range.end = length;
                segment.end.utf8_byte_offset = segment.end.utf8_byte_offset.saturating_sub(removed);
            }
        }
        self.end = end;
    }
    pub(crate) fn anchor_column(&self, wanted: u16) -> Anchor {
        let mut previous = self.start;
        for segment in &self.segments {
            if wanted < segment.column {
                return previous;
            }
            let text = &self.text[segment.range.clone()];
            let mut column = usize::from(segment.column);
            let synthetic_tab =
                segment.end.utf8_byte_offset - segment.start.utf8_byte_offset != text.len();
            if synthetic_tab {
                if usize::from(wanted) < column + text.len() {
                    return segment.start;
                }
            } else {
                for (offset, character) in text.char_indices() {
                    let width = column_width(character, self.font_columns);
                    if usize::from(wanted) < column + width {
                        return Anchor {
                            utf8_byte_offset: segment.start.utf8_byte_offset + offset,
                            ..segment.start
                        };
                    }
                    column += width;
                }
            }
            previous = segment.end;
        }
        self.end
    }
    pub(crate) fn anchor_visual_column(&self, anchor: Anchor) -> Option<u16> {
        for segment in &self.segments {
            if anchor.epoch != segment.start.epoch
                || anchor.record_id != segment.record_id
                || anchor.utf8_byte_offset < segment.start.utf8_byte_offset
                || anchor.utf8_byte_offset > segment.end.utf8_byte_offset
            {
                continue;
            }
            let text = &self.text[segment.range.clone()];
            let offset = anchor.utf8_byte_offset - segment.start.utf8_byte_offset;
            let width =
                if segment.end.utf8_byte_offset - segment.start.utf8_byte_offset != text.len() {
                    if anchor == segment.end { text.len() } else { 0 }
                } else {
                    if !text.is_char_boundary(offset) {
                        return None;
                    }
                    self.text_columns(&text[..offset])
                };
            return Some((usize::from(segment.column) + width).min(u16::MAX as usize) as u16);
        }
        (anchor == self.start).then_some(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cursor {
    id: u64,
    byte: usize,
}
impl Cursor {
    fn anchor(self, epoch: u64) -> Anchor {
        Anchor {
            epoch,
            record_id: self.id,
            utf8_byte_offset: self.byte,
        }
    }
}
#[derive(Clone, Copy)]
struct Checkpoint {
    row: usize,
    cursor: Cursor,
    hard: bool,
}

struct StyledRange {
    range: Range<usize>,
    style: Style,
}
struct Loaded {
    id: u64,
    source: u64,
    text: String,
    terminal: bool,
    plain: bool,
    native: bool,
    fixed_native: bool,
    omitted: bool,
    join_next: bool,
    packed: Option<u16>,
    color: Option<(u8, u8, u8)>,
    styles: Vec<StyledRange>,
}
impl Loaded {
    fn new(record: Record) -> io::Result<Self> {
        Self::with_decoration(record, None)
    }
    fn with_decoration(record: Record, decoration: Option<&Decoration>) -> io::Result<Self> {
        let plain = record_text(&record)?;
        let mut loaded = Self {
            id: record.id,
            source: record.source,
            text: plain.text,
            terminal: false,
            plain: false,
            native: false,
            fixed_native: false,
            omitted: plain.omitted,
            join_next: plain.join_next,
            packed: None,
            color: None,
            styles: Vec::new(),
        };
        match &record.data {
            RecordData::Native(bytes) => {
                loaded.native = true;
                loaded.fixed_native = bytes[14] & 16 != 0;
                loaded.packed = native_packing(bytes);
                let row = decode_native(bytes)?;
                let foreground = row.color.unwrap_or(row.kind.color());
                let hash_color = row.color.unwrap_or((225, 195, 120));
                let entry = native_entry_range(bytes)?;
                let range = entry.clone().or_else(|| {
                    (bytes[14] & 0x80 != 0)
                        .then(|| super::git_detail::hash_range(&row.text))
                        .flatten()
                });
                // A listing's Git decoration applies only to its filename.
                // Non-entry Git output retains its semantic color across the row.
                loaded.color = entry.is_none().then_some(foreground);
                if let Some(range) = range {
                    loaded.styles.push(StyledRange {
                        range,
                        style: Style {
                            fg: if entry.is_none() {
                                vt100::Color::Rgb(hash_color.0, hash_color.1, hash_color.2)
                            } else {
                                vt100::Color::Rgb(foreground.0, foreground.1, foreground.2)
                            },
                            underline: entry.is_none()
                                || matches!(row.kind, Kind::Text | Kind::Directory),
                            ..Style::default()
                        },
                    });
                }
            }
            RecordData::Terminal {
                cols: 0, formatted, ..
            } => {
                loaded.terminal = true;
                loaded.plain = true;
                let (tone, underline) = decoration.map_or_else(
                    || output_colors::stored_style(formatted),
                    |style| (style.tone, style.underline.clone()),
                );
                let mut base = Style::default();
                if let Some(tone) = tone {
                    let (r, g, b) = tone.rgb();
                    base.fg = vt100::Color::Rgb(r, g, b);
                }
                let underline = underline.filter(|range| {
                    range.start < range.end
                        && range.end <= loaded.text.len()
                        && loaded.text.is_char_boundary(range.start)
                        && loaded.text.is_char_boundary(range.end)
                });
                if let Some(range) = underline {
                    if range.start > 0 {
                        loaded.styles.push(StyledRange {
                            range: 0..range.start,
                            style: base,
                        });
                    }
                    loaded.styles.push(StyledRange {
                        range: range.clone(),
                        style: Style {
                            underline: true,
                            ..base
                        },
                    });
                    if range.end < loaded.text.len() {
                        loaded.styles.push(StyledRange {
                            range: range.end..loaded.text.len(),
                            style: base,
                        });
                    }
                } else if !loaded.text.is_empty() {
                    loaded.styles.push(StyledRange {
                        range: 0..loaded.text.len(),
                        style: base,
                    });
                }
            }
            RecordData::Terminal {
                cols, formatted, ..
            } => {
                loaded.terminal = true;
                let mut parser = vt100::Parser::new(1, (*cols).clamp(1, 256), 0);
                parser.process(formatted);
                let (tone, underline) = decoration.map_or_else(
                    || output_colors::stored_style(formatted),
                    |style| (style.tone, style.underline.clone()),
                );
                let mut offset = 0;
                for column in 0..(*cols).clamp(1, 256) {
                    let Some(cell) = parser.screen().cell(0, column) else {
                        continue;
                    };
                    if cell.is_wide_continuation() {
                        continue;
                    }
                    let length = if cell.has_contents() {
                        cell.contents().len()
                    } else {
                        1
                    };
                    let end = (offset + length).min(loaded.text.len());
                    if end > offset {
                        let mut style = Style::default();
                        if let Some(tone) = tone {
                            let (r, g, b) = tone.rgb();
                            style.fg = vt100::Color::Rgb(r, g, b);
                        }
                        style.underline |= underline
                            .as_ref()
                            .is_some_and(|range| range.start < end && offset < range.end);
                        if let Some(previous) = loaded.styles.last_mut() {
                            if previous.style == style {
                                previous.range.end = end;
                            } else {
                                loaded.styles.push(StyledRange {
                                    range: offset..end,
                                    style,
                                });
                            }
                        } else {
                            loaded.styles.push(StyledRange {
                                range: offset..end,
                                style,
                            });
                        }
                    }
                    offset = end;
                    if offset == loaded.text.len() {
                        break;
                    }
                }
            }
            RecordData::Header(_) | RecordData::Result { .. } => {}
        }
        Ok(loaded)
    }
    fn style(&self, byte: usize) -> Style {
        self.styles
            .iter()
            .find(|range| range.range.contains(&byte))
            .map_or_else(
                || Style {
                    fg: self.color.map_or(vt100::Color::Default, |(r, g, b)| {
                        vt100::Color::Rgb(r, g, b)
                    }),
                    ..Style::default()
                },
                |range| range.style,
            )
    }
    // The first matching style wins. Stop a bulk span at every earlier range
    // boundary, including one which begins inside the currently selected range.
    fn style_span(&self, byte: usize) -> (Style, usize) {
        let mut end = self.text.len();
        for range in &self.styles {
            if range.range.contains(&byte) {
                return (range.style, end.min(range.range.end));
            }
            if range.range.start > byte {
                end = end.min(range.range.start);
            }
        }
        (self.style(byte), end)
    }
    fn plain(text: &str, id: u64, source: u64, decoration: &Decoration) -> Self {
        let mut style = Style::default();
        if let Some(tone) = decoration.tone {
            let (r, g, b) = tone.rgb();
            style.fg = vt100::Color::Rgb(r, g, b);
        }
        let mut styles = Vec::with_capacity(3);
        if let Some(range) = &decoration.underline {
            let start = range.start.min(text.len());
            let end = range.end.min(text.len());
            styles.push(StyledRange {
                range: 0..start,
                style,
            });
            styles.push(StyledRange {
                range: start..end,
                style: Style {
                    underline: true,
                    ..style
                },
            });
            styles.push(StyledRange {
                range: end..text.len(),
                style,
            });
        } else {
            styles.push(StyledRange {
                range: 0..text.len(),
                style,
            });
        }
        Self {
            id,
            source,
            text: text.to_owned(),
            terminal: true,
            plain: true,
            native: false,
            fixed_native: false,
            omitted: false,
            join_next: false,
            packed: None,
            color: None,
            styles,
        }
    }
    fn live(
        screen: &vt100::Screen,
        row: u16,
        id: u64,
        source: u64,
        decoration: &Decoration,
    ) -> Self {
        let plain = terminal_row_text(screen, row);
        let mut loaded = Self {
            id,
            source,
            text: plain.text,
            terminal: true,
            plain: false,
            native: false,
            fixed_native: false,
            omitted: plain.omitted,
            join_next: plain.join_next,
            packed: None,
            color: None,
            styles: Vec::new(),
        };
        let mut offset = 0;
        for column in 0..screen.size().1 {
            let Some(cell) = screen.cell(row, column) else {
                continue;
            };
            if cell.is_wide_continuation() {
                continue;
            }
            let length = if cell.has_contents() {
                cell.contents().len()
            } else {
                1
            };
            let end = (offset + length).min(loaded.text.len());
            if end > offset {
                let mut style = Style::default();
                if let Some(tone) = decoration.tone {
                    let (r, g, b) = tone.rgb();
                    style.fg = vt100::Color::Rgb(r, g, b);
                }
                style.underline |= decoration
                    .underline
                    .as_ref()
                    .is_some_and(|range| range.start < end && offset < range.end);
                if let Some(previous) = loaded
                    .styles
                    .last_mut()
                    .filter(|previous| previous.style == style)
                {
                    previous.range.end = end;
                } else {
                    loaded.styles.push(StyledRange {
                        range: offset..end,
                        style,
                    });
                }
            }
            offset = end;
            if offset == loaded.text.len() {
                break;
            }
        }
        loaded
    }
}

fn native_segment_color(row: &Loaded, style: Style) -> Option<(u8, u8, u8)> {
    if row.native {
        match style.fg {
            vt100::Color::Rgb(r, g, b) => Some((r, g, b)),
            _ => None,
        }
    } else {
        row.color
    }
}

struct Tail {
    screen: Option<vt100::Screen>,
    plain: Option<Arc<str>>,
    first: u64,
    start: u16,
    count: u16,
    source: u64,
    decoration: Vec<Decoration>,
    prefix_decoration: Vec<(u64, Decoration)>,
}
impl Tail {
    fn end(&self) -> u64 {
        self.first + u64::from(self.count)
    }
}
struct Source<'a> {
    store: &'a SharedTranscript,
    disk_end: u64,
    tail: Option<&'a Tail>,
}
impl Source<'_> {
    fn read(&self, id: u64, end: u64) -> io::Result<Loaded> {
        if id < self.disk_end {
            let decoration = self.tail.and_then(|tail| {
                tail.prefix_decoration
                    .iter()
                    .find(|(row, _)| *row == id)
                    .map(|(_, style)| style)
            });
            let record = self.store.lock().unwrap().read_id(id)?;
            let final_result = id + 1 == end && matches!(record.data, RecordData::Result { .. });
            let mut loaded = Loaded::with_decoration(record, decoration)?;
            if final_result {
                if loaded.omitted {
                    loaded.omitted = false;
                } else {
                    loaded.text.push('\n');
                }
            }
            return Ok(loaded);
        }
        let tail = self
            .tail
            .filter(|tail| id >= tail.first && id < tail.end())
            .ok_or_else(|| io::Error::other("Layout row is unavailable"))?;
        if let Some(text) = &tail.plain {
            return Ok(Loaded::plain(text, id, tail.source, &tail.decoration[0]));
        }
        Ok(Loaded::live(
            tail.screen.as_ref().expect("Grid tail has a screen"),
            tail.start + (id - tail.first) as u16,
            id,
            tail.source,
            &tail.decoration[(id - tail.first) as usize],
        ))
    }
}

struct Budget {
    deadline: Instant,
    reads: usize,
}
impl Budget {
    fn new() -> Self {
        Self {
            deadline: Instant::now() + Duration::from_millis(2),
            reads: 128,
        }
    }
    fn exhausted(&self) -> bool {
        self.reads == 0 || Instant::now() >= self.deadline
    }
}

enum Mode {
    Text {
        last_break: Option<(Cursor, usize)>,
    },
    Packed {
        cell: usize,
        count: usize,
        entries: usize,
        source: u64,
    },
}
struct Building {
    start: Cursor,
    hard: bool,
    fragment: Fragment,
    projected: bool,
    text_bytes: usize,
    segment_count: usize,
    last_segment: Option<Segment>,
    width: usize,
    tab_column: usize,
    wrap_pixels: usize,
    mode: Mode,
}
struct Flow {
    cursor: Cursor,
    loaded: Option<Loaded>,
    building: Option<Building>,
    hard: bool,
    output_hard: bool,
    projected: bool,
}
impl Flow {
    fn new(cursor: Cursor) -> Self {
        Self {
            cursor,
            loaded: None,
            building: None,
            hard: cursor.byte == 0,
            output_hard: cursor.byte == 0,
            projected: true,
        }
    }
    // Counting and sparse checkpoints need the exact wrapping state, not a
    // second copy of the visible text and hit/style segments. Keep their byte
    // and segment limits identical to the projecting traversal.
    fn counting(cursor: Cursor) -> Self {
        let mut flow = Self::new(cursor);
        flow.projected = false;
        flow
    }
    fn checkpoint(checkpoint: Checkpoint) -> Self {
        let mut flow = Self::new(checkpoint.cursor);
        flow.hard = checkpoint.hard;
        flow
    }
    fn counting_checkpoint(checkpoint: Checkpoint) -> Self {
        let mut flow = Self::checkpoint(checkpoint);
        flow.projected = false;
        flow
    }
    fn finish(&mut self, build: Building, hard: bool) -> Option<(Cursor, Fragment)> {
        self.hard = hard;
        self.output_hard = build.hard;
        Some((build.start, build.fragment))
    }
    fn load(&mut self, source: &Source<'_>, end: u64, budget: &mut Budget) -> io::Result<bool> {
        while self.cursor.id < end {
            if self
                .loaded
                .as_ref()
                .is_some_and(|row| row.id == self.cursor.id)
            {
                return Ok(true);
            }
            if budget.exhausted() {
                return Ok(false);
            }
            budget.reads -= 1;
            self.loaded = Some(source.read(self.cursor.id, end)?);
            let row = self.loaded.as_ref().unwrap();
            if row.omitted {
                self.cursor = Cursor {
                    id: self.cursor.id + 1,
                    byte: 0,
                };
                self.loaded = None;
                continue;
            }
            if self.cursor.byte > row.text.len() || !row.text.is_char_boundary(self.cursor.byte) {
                return Err(io::Error::other("Layout anchor is not a UTF-8 boundary"));
            }
            return Ok(true);
        }
        Ok(false)
    }
    fn finished(&self, end: u64) -> bool {
        self.cursor.id >= end && self.building.is_none()
    }
    fn next(
        &mut self,
        source: &Source<'_>,
        end: u64,
        epoch: u64,
        metrics: Metrics,
        font_columns: bool,
        pixels: Option<PixelMetrics>,
        scalar_widths: &mut ScalarWidths,
        measure: &mut Option<&mut dyn FnMut(char) -> i32>,
        budget: &mut Budget,
    ) -> io::Result<Option<(Cursor, Fragment)>> {
        self.next_inner::<true>(
            source,
            end,
            epoch,
            metrics,
            font_columns,
            pixels,
            scalar_widths,
            measure,
            budget,
        )
    }
    fn next_inner<const ASCII_RUNS: bool>(
        &mut self,
        source: &Source<'_>,
        end: u64,
        epoch: u64,
        metrics: Metrics,
        font_columns: bool,
        pixels: Option<PixelMetrics>,
        scalar_widths: &mut ScalarWidths,
        measure: &mut Option<&mut dyn FnMut(char) -> i32>,
        budget: &mut Budget,
    ) -> io::Result<Option<(Cursor, Fragment)>> {
        if self.building.is_none() {
            if !self.load(source, end, budget)? {
                return Ok(None);
            }
            let row = self.loaded.as_ref().unwrap();
            let packed = row
                .packed
                .filter(|cell| usize::from(*cell) <= usize::from(metrics.columns));
            let mode = if let Some(cell) = packed {
                let count = ((usize::from(metrics.columns) + 2) / usize::from(cell)).max(1);
                if count > 1 && text_width(&row.text, font_columns) <= usize::from(metrics.columns)
                {
                    Mode::Packed {
                        cell: usize::from(cell),
                        count,
                        entries: 0,
                        source: row.source,
                    }
                } else {
                    Mode::Text { last_break: None }
                }
            } else {
                Mode::Text { last_break: None }
            };
            let anchor = self.cursor.anchor(epoch);
            self.building = Some(Building {
                start: self.cursor,
                hard: self.hard,
                fragment: Fragment {
                    start: anchor,
                    end: anchor,
                    text: String::new(),
                    segments: Vec::new(),
                    font_columns,
                },
                projected: self.projected,
                text_bytes: 0,
                segment_count: 0,
                last_segment: None,
                width: 0,
                tab_column: 0,
                wrap_pixels: 0,
                mode,
            });
        }
        let mut build = self.building.take().unwrap();
        let mut characters = 0;
        loop {
            if !self.load(source, end, budget)? {
                if self.cursor.id >= end {
                    return Ok(self.finish(build, false));
                }
                self.building = Some(build);
                return Ok(None);
            }
            let row = self.loaded.as_ref().unwrap();
            if let Mode::Packed {
                cell,
                count,
                entries,
                source,
            } = &mut build.mode
            {
                if row.source != *source || row.packed != Some(*cell as u16) {
                    return Ok(self.finish(build, true));
                }
                let width = text_width(&row.text, font_columns);
                let padding = if *entries == 0 {
                    0
                } else {
                    cell.saturating_sub(build.width % *cell).max(2)
                };
                if *entries > 0 && build.width + padding + width > usize::from(metrics.columns) {
                    return Ok(self.finish(build, false));
                }
                if padding != 0 {
                    if build.projected {
                        build
                            .fragment
                            .text
                            .extend(std::iter::repeat_n(' ', padding));
                    }
                    build.text_bytes += padding;
                    build.width += padding;
                }
                let anchor = Cursor {
                    id: row.id,
                    byte: 0,
                }
                .anchor(epoch);
                let end_anchor = Cursor {
                    id: row.id,
                    byte: row.text.len(),
                }
                .anchor(epoch);
                let start = build.text_bytes;
                build.text_bytes += row.text.len();
                if build.projected {
                    build.fragment.text.push_str(&row.text);
                }
                let segment = Segment {
                    range: start..build.text_bytes,
                    start: anchor,
                    end: end_anchor,
                    column: build.width as u16,
                    record_id: row.id,
                    source: row.source,
                    native: row.native,
                    color: native_segment_color(row, row.style(0)),
                    style: row.style(0),
                };
                build.segment_count += 1;
                if build.projected {
                    build.fragment.segments.push(segment.clone());
                }
                build.last_segment = Some(segment);
                build.fragment.end = end_anchor;
                build.width += width;
                *entries += 1;
                self.cursor = Cursor {
                    id: row.id + 1,
                    byte: 0,
                };
                self.loaded = None;
                if *entries == *count || budget.exhausted() {
                    if *entries == *count {
                        return Ok(self.finish(build, false));
                    }
                    self.building = Some(build);
                    return Ok(None);
                }
                continue;
            }
            if self.cursor.byte >= row.text.len() {
                let old_source = row.source;
                let join = (row.terminal || row.fixed_native) && row.join_next;
                let old_terminal = row.terminal;
                let old_plain = row.plain;
                let old_fixed_native = row.fixed_native;
                let previous = self.cursor;
                self.cursor = Cursor {
                    id: row.id + 1,
                    byte: 0,
                };
                self.loaded = None;
                if !join || self.cursor.id >= end {
                    return Ok(self.finish(build, true));
                }
                // A saved soft wrap only joins another row from the same command.
                // The next load may yield; remember that provenance in the flow.
                if !self.load(source, end, budget)? {
                    self.cursor = previous;
                    self.building = Some(build);
                    return Ok(None);
                }
                let next = self.loaded.as_ref().unwrap();
                if next.source != old_source
                    || !(old_terminal && next.terminal && old_plain == next.plain
                        || old_fixed_native && next.fixed_native)
                {
                    return Ok(self.finish(build, true));
                }
                continue;
            }
            if ASCII_RUNS
                && !build.projected
                && matches!(row.text.as_bytes()[self.cursor.byte], b' '..=b'~')
            {
                // Printable ASCII has one cell and never needs a font lookup.
                // Leave overflow whitespace, tabs, UTF-8 and the first oversized
                // glyph to the scalar path. Keep its exact 64-scalar yield points.
                // Visible projection keeps scalar String/Vec growth unchanged,
                // since their capacity also contributes to its byte allowance.
                let cell_pixels = pixels.map_or(1, |pixels| pixels.cell);
                let wrap_limit = pixels.map_or(usize::from(metrics.columns), |pixels| pixels.width);
                let wrap_room = wrap_limit.saturating_sub(build.wrap_pixels) / cell_pixels;
                let (style, style_end) = row.style_span(self.cursor.byte);
                let anchor = self.cursor.anchor(epoch);
                let color = native_segment_color(row, style);
                let merge = build.last_segment.as_ref().is_some_and(|previous| {
                    previous.end == anchor
                        && previous.style == style
                        && previous.color == color
                        && previous.range.end - previous.range.start
                            == previous.end.utf8_byte_offset - previous.start.utf8_byte_offset
                });
                let segment_room = if !merge && build.segment_count + 1 >= MAX_FRAGMENT_SEGMENTS {
                    1
                } else {
                    usize::MAX
                };
                let limit = wrap_room
                    .min(MAX_FRAGMENT_BYTES - build.text_bytes)
                    .min(style_end - self.cursor.byte)
                    .min(64 - characters % 64)
                    .min(segment_room);
                let bytes = &row.text.as_bytes()[self.cursor.byte..];
                let run = bytes
                    .iter()
                    .take(limit)
                    .take_while(|byte| (b' '..=b'~').contains(*byte))
                    .count();
                if run > 1 {
                    let start = build.text_bytes;
                    self.cursor.byte += run;
                    build.text_bytes += run;
                    let end_anchor = self.cursor.anchor(epoch);
                    if merge {
                        let previous = build.last_segment.as_mut().unwrap();
                        previous.range.end = build.text_bytes;
                        previous.end = end_anchor;
                    } else {
                        let segment = Segment {
                            range: start..build.text_bytes,
                            start: anchor,
                            end: end_anchor,
                            column: build.width.min(u16::MAX as usize) as u16,
                            record_id: anchor.record_id,
                            source: row.source,
                            native: row.native,
                            color,
                            style,
                        };
                        build.segment_count += 1;
                        build.last_segment = Some(segment);
                    }
                    if let Some(space) = bytes[..run].iter().rposition(|byte| *byte == b' ') {
                        if let Mode::Text { last_break } = &mut build.mode {
                            *last_break = Some((
                                Cursor {
                                    id: row.id,
                                    byte: self.cursor.byte - run + space + 1,
                                },
                                start + space + 1,
                            ));
                        }
                    }
                    build.fragment.end = end_anchor;
                    build.width = build.width.saturating_add(run);
                    build.tab_column = build.tab_column.saturating_add(run);
                    build.wrap_pixels = build.wrap_pixels.saturating_add(run * cell_pixels);
                    characters += run;
                    if build.text_bytes >= MAX_FRAGMENT_BYTES {
                        return Ok(self.finish(build, false));
                    }
                    if characters % 64 == 0 && budget.exhausted() {
                        self.building = Some(build);
                        return Ok(None);
                    }
                    continue;
                }
            }
            let character = row.text[self.cursor.byte..].chars().next().unwrap();
            let anchor = self.cursor.anchor(epoch);
            if character == '\n' {
                self.cursor.byte += 1;
                build.fragment.end = anchor;
                return Ok(self.finish(build, true));
            }
            let width = if character == '\t' {
                metrics.tab_width
                    - (if font_columns {
                        build.tab_column
                    } else {
                        build.width
                    }) % metrics.tab_width
            } else {
                column_width(character, font_columns)
            };
            let character_pixels = if character == '\t' {
                width.saturating_mul(pixels.map_or(1, |pixels| pixels.cell))
            } else {
                let Some(width) = scalar_widths.width(character, pixels, font_columns, measure)
                else {
                    self.building = Some(build);
                    return Ok(None);
                };
                width
            };
            let wrap_limit = pixels.map_or(usize::from(metrics.columns), |pixels| pixels.width);
            if self.cursor != build.start
                && build.wrap_pixels.saturating_add(character_pixels) > wrap_limit
            {
                if !character.is_whitespace() {
                    if let Mode::Text {
                        last_break: Some((cursor, length)),
                    } = &build.mode
                    {
                        if *length != 0 && *length < build.text_bytes {
                            self.cursor = *cursor;
                            self.loaded = None;
                            if build.projected {
                                build.fragment.trim(*length, cursor.anchor(epoch));
                            } else {
                                build.fragment.end = cursor.anchor(epoch);
                            }
                            build.text_bytes = *length;
                        }
                    }
                    return Ok(self.finish(build, false));
                }
                // Legacy wrapping keeps separators on the preceding visual row.
                // Still bound very long runs of whitespace rather than buffering
                // a transcript-sized logical line.
                if build.text_bytes >= MAX_FRAGMENT_BYTES - 4 {
                    return Ok(self.finish(build, false));
                }
            }
            let style = row.style(self.cursor.byte);
            let color = native_segment_color(row, style);
            if build.text_bytes + character.len_utf8().max(width) > MAX_FRAGMENT_BYTES {
                return Ok(self.finish(build, false));
            }
            let start = build.text_bytes;
            if character == '\t' {
                if build.projected {
                    build.fragment.text.extend(std::iter::repeat_n(' ', width));
                }
                build.text_bytes += width;
            } else {
                if build.projected {
                    build.fragment.text.push(character);
                }
                build.text_bytes += character.len_utf8();
            }
            self.cursor.byte += character.len_utf8();
            let end_anchor = self.cursor.anchor(epoch);
            let tab = character == '\t';
            let previous = build.last_segment.as_mut();
            if let Some(previous) = previous.filter(|previous| {
                !tab && previous.end == anchor
                    && previous.style == style
                    && previous.color == color
                    && previous.range.end - previous.range.start
                        == previous.end.utf8_byte_offset - previous.start.utf8_byte_offset
            }) {
                previous.range.end = build.text_bytes;
                previous.end = end_anchor;
                if build.projected {
                    let projected = build.fragment.segments.last_mut().unwrap();
                    projected.range.end = build.text_bytes;
                    projected.end = end_anchor;
                }
            } else {
                let segment = Segment {
                    range: start..build.text_bytes,
                    start: anchor,
                    end: end_anchor,
                    column: build.width.min(u16::MAX as usize) as u16,
                    record_id: anchor.record_id,
                    source: row.source,
                    native: row.native,
                    color,
                    style,
                };
                build.segment_count += 1;
                if build.projected {
                    build.fragment.segments.push(segment.clone());
                }
                build.last_segment = Some(segment);
            }
            build.fragment.end = end_anchor;
            build.width = build.width.saturating_add(width);
            build.wrap_pixels = build.wrap_pixels.saturating_add(character_pixels);
            build.tab_column =
                build
                    .tab_column
                    .saturating_add(if character == '\t' || !font_columns {
                        width
                    } else {
                        1
                    });
            if character.is_whitespace() {
                if let Mode::Text { last_break } = &mut build.mode {
                    *last_break = Some((self.cursor, build.text_bytes));
                }
            }
            characters += 1;
            if build.text_bytes >= MAX_FRAGMENT_BYTES
                || build.segment_count >= MAX_FRAGMENT_SEGMENTS
                || (characters % 64 == 0 && budget.exhausted())
            {
                if build.text_bytes >= MAX_FRAGMENT_BYTES
                    || build.segment_count >= MAX_FRAGMENT_SEGMENTS
                {
                    return Ok(self.finish(build, false));
                }
                self.building = Some(build);
                return Ok(None);
            }
        }
    }
}

// The cap applies to each projection, including Vec slack. Published rows and
// Pane rows normally share it. While a query grows or a viewport is replaced,
// a prior published frame and the Pane's prior frame can remain alive: at most
// three viewport buffers plus one navigation buffer (4 * MAX_VISIBLE_BYTES),
// excluding small Arc/Vec headers, Flow scratch and the separately bounded tail.
// A saved Git Back view owns a separate, frozen Layout/Pane set.
struct Query {
    start: usize,
    count: usize,
    row: usize,
    flow: Flow,
    end: u64,
    tail: Option<Arc<Tail>>,
    fragments: Arc<Vec<Fragment>>,
    bytes: usize,
    revision: u64,
    ready: bool,
}
impl Query {
    fn frame_mut(fragments: &mut Arc<Vec<Fragment>>) -> &mut Vec<Fragment> {
        if Arc::get_mut(fragments).is_none() {
            // A published frame remains immutable for painting and hit tests.
            // Copy only when this still-growing query next adds a row, retaining
            // its already-accounted Vec slack and the existing content cap.
            let mut rows = Vec::with_capacity(fragments.capacity());
            rows.extend(fragments.iter().cloned());
            *fragments = Arc::new(rows);
        }
        Arc::get_mut(fragments).unwrap()
    }
    #[cfg(test)]
    fn fragments_mut(&mut self) -> &mut Vec<Fragment> {
        Self::frame_mut(&mut self.fragments)
    }
}
struct Published {
    start: usize,
    revision: u64,
    rows: Arc<Vec<Fragment>>,
    tail: Option<Arc<Tail>>,
}

struct Locate {
    anchor: Anchor,
    row: usize,
    flow: Flow,
    end: u64,
    tail: Option<Arc<Tail>>,
    result: Option<usize>,
    ready: bool,
}
struct Rebase {
    target: Checkpoint,
    flow: Flow,
    rows: usize,
}

pub(crate) struct Layout {
    store: SharedTranscript,
    wake: Wake,
    metrics: Metrics,
    font_columns: bool,
    pixels: Option<PixelMetrics>,
    scalar_widths: ScalarWidths,
    epoch: u64,
    base: u64,
    end: u64,
    disk_end: u64,
    tail: Option<Arc<Tail>>,
    pending_tail: Option<Option<Arc<Tail>>>,
    pending_end: bool,
    revision: u64,
    projection_revision: u64,
    rows: usize,
    origin: usize,
    rebase: Option<Rebase>,
    complete: bool,
    flow: Flow,
    last_start: Option<Cursor>,
    last_hard: bool,
    checkpoints: Vec<Checkpoint>,
    stride: usize,
    query: Option<Query>,
    navigation: Option<Query>,
    locate: Option<Locate>,
    published: Option<Published>,
    published_rows: usize,
}
impl Layout {
    pub(crate) fn new(store: SharedTranscript, wake: Wake) -> io::Result<Self> {
        let (epoch, base, end, revision) = {
            let transcript = store.lock().unwrap();
            (
                transcript.epoch(),
                transcript.base_id(),
                transcript.base_id() + transcript.len() as u64,
                transcript.serial(),
            )
        };
        Ok(Self {
            store,
            wake,
            metrics: Metrics::default(),
            font_columns: false,
            pixels: None,
            scalar_widths: ScalarWidths::default(),
            epoch,
            base,
            end,
            disk_end: end,
            tail: None,
            pending_tail: None,
            pending_end: false,
            revision,
            projection_revision: revision,
            rows: 0,
            origin: 0,
            rebase: None,
            complete: base == end,
            flow: Flow::counting(Cursor { id: base, byte: 0 }),
            last_start: None,
            last_hard: true,
            checkpoints: Vec::with_capacity(CHECKPOINTS),
            stride: 16,
            query: None,
            navigation: None,
            locate: None,
            published: None,
            published_rows: 0,
        })
    }
    pub(crate) fn configure(&mut self, metrics: Metrics) -> bool {
        let metrics = metrics.bounded();
        if self.metrics == metrics {
            return false;
        }
        self.metrics = metrics;
        if let Some(next) = self.pending_tail.take() {
            self.tail = next;
        }
        self.end = self
            .tail
            .as_ref()
            .map_or(self.disk_end, |tail| self.disk_end.max(tail.end()));
        self.pending_end = false;
        self.reset();
        (self.wake)();
        true
    }
    pub(crate) fn set_font_columns(&mut self, enabled: bool) -> bool {
        if self.font_columns == enabled {
            return false;
        }
        self.font_columns = enabled;
        self.reset();
        (self.wake)();
        true
    }
    /// Match the original per-scalar wrapping width while drawing keeps the
    /// combined glyph advance. Only zero-column scalar widths need a cache.
    pub(crate) fn set_wrap_pixels(&mut self, width: i32, cell: i32, font_key: u32) -> bool {
        let pixels = PixelMetrics {
            width: width.max(1) as usize,
            cell: cell.max(1) as usize,
            font_key,
        };
        if self.pixels == Some(pixels) {
            return false;
        }
        if self
            .pixels
            .is_none_or(|old| old.cell != pixels.cell || old.font_key != pixels.font_key)
        {
            self.scalar_widths = ScalarWidths::default();
        }
        self.pixels = Some(pixels);
        self.reset();
        (self.wake)();
        true
    }
    /// The live screen is the bounded mutable tail, never another disk history.
    /// A query retains an Arc to the snapshot it began with while new output
    /// arrives, so continuous output cannot discard its unfinished projection.
    pub(crate) fn set_tail(
        &mut self,
        screen: &vt100::Screen,
        start: u16,
        count: u16,
        source: u64,
    ) -> io::Result<bool> {
        self.sync();
        let next = if count == 0 {
            if self.tail.is_none() && self.pending_tail.as_ref().is_none_or(Option::is_none) {
                return Ok(false);
            }
            None
        } else {
            let (rows, columns) = screen.size();
            if rows > 160 || columns > 256 || start >= rows || count > rows - start {
                return Err(io::Error::other(
                    "Live terminal snapshot exceeds its bounded grid",
                ));
            }
            let mut colors = self.store.lock().unwrap().color_preview(source);
            let mut decoration = Vec::with_capacity(usize::from(count));
            let mut prefix_decoration = Vec::new();
            for row in start..start + count {
                let text = terminal_row_text(screen, row);
                let change = colors.row(
                    source,
                    &text.text,
                    text.join_next,
                    Some(self.disk_end + u64::from(row - start)),
                );
                for (id, style) in change.previous {
                    if id < self.disk_end {
                        if let Some((_, previous)) =
                            prefix_decoration.iter_mut().find(|(old, _)| *old == id)
                        {
                            *previous = style;
                        } else if prefix_decoration.len() < 256 {
                            prefix_decoration.push((id, style));
                        }
                    } else if let Some(previous) = decoration.get_mut((id - self.disk_end) as usize)
                    {
                        *previous = style;
                    }
                }
                decoration.push(change.current);
            }
            Some(Arc::new(Tail {
                screen: Some(screen.normal_screen_clone()),
                plain: None,
                first: self.disk_end,
                start,
                count,
                source,
                decoration,
                prefix_decoration,
            }))
        };
        // A captured pass always finishes. Updates replace one pending snapshot
        // rather than rewinding the same first16 rows indefinitely under output.
        self.pending_tail = Some(next);
        self.pending_end = true;
        if self.complete {
            self.refresh();
        }
        (self.wake)();
        Ok(true)
    }
    /// A plaintext command retains only its unfinished logical line. Complete
    /// lines already live in the shared disk transcript, preserving literal
    /// tabs and trailing whitespace without another history buffer.
    pub(crate) fn set_plain_tail(&mut self, text: &str, source: u64) -> io::Result<bool> {
        if text.len() > MAX_PLAIN_TAIL_BYTES || text.contains('\n') {
            return Err(io::Error::other(
                "Plain output tail exceeds its bounded logical line",
            ));
        }
        self.sync();
        if text.is_empty()
            && self.tail.is_none()
            && self.pending_tail.as_ref().is_none_or(Option::is_none)
        {
            return Ok(false);
        }
        let next = if text.is_empty() {
            None
        } else {
            let mut colors = self.store.lock().unwrap().color_preview(source);
            let change = colors.row(source, text, false, Some(self.disk_end));
            Some(Arc::new(Tail {
                screen: None,
                plain: Some(Arc::from(text)),
                first: self.disk_end,
                start: 0,
                count: 1,
                source,
                decoration: vec![change.current],
                prefix_decoration: change
                    .previous
                    .into_iter()
                    .filter(|(id, _)| *id < self.disk_end)
                    .take(256)
                    .collect(),
            }))
        };
        self.pending_tail = Some(next);
        self.pending_end = true;
        if self.complete {
            self.refresh();
        }
        (self.wake)();
        Ok(true)
    }
    fn refresh(&mut self) {
        let previous_first = self.tail.as_ref().map(|tail| tail.first);
        let tail_changed = self.pending_tail.is_some();
        if let Some(next) = self.pending_tail.take() {
            self.tail = next;
        }
        self.end = self
            .tail
            .as_ref()
            .map_or(self.disk_end, |tail| self.disk_end.max(tail.end()));
        self.pending_end = false;
        if tail_changed {
            let first = previous_first
                .into_iter()
                .chain(self.tail.as_ref().map(|tail| tail.first))
                .min()
                .unwrap_or(self.disk_end);
            self.projection_revision = self.projection_revision.wrapping_add(1);
            self.rewind(first);
        } else if let Some(start) = self.last_start {
            // A final error footer includes plaintext's EOF newline. Appending
            // removes that contextual byte, so its former EOF cursor cannot be
            // reused. Rebuild that record from a bounded sparse checkpoint.
            let contextual_eof = start.byte != 0
                && self
                    .store
                    .lock()
                    .unwrap()
                    .read_id(start.id)
                    .is_ok_and(|record| {
                        matches!(record.data,
                    RecordData::Result { status, .. } if status != 0)
                    });
            if contextual_eof {
                self.rewind(start.id);
            } else {
                self.rows = self.rows.saturating_sub(1);
                self.checkpoints
                    .retain(|checkpoint| checkpoint.row < self.rows);
                self.flow = Flow::counting(start);
                self.flow.hard = self.last_hard;
                self.complete = false;
            }
        } else {
            self.flow = Flow::counting(Cursor {
                id: self.base,
                byte: 0,
            });
            self.complete = self.flow.finished(self.end);
        }
    }
    fn rewind(&mut self, id: u64) {
        let checkpoint = self
            .checkpoints
            .iter()
            .rev()
            .find(|point| point.cursor.id < id)
            .copied()
            .unwrap_or(Checkpoint {
                row: self.origin,
                cursor: Cursor {
                    id: self.base,
                    byte: 0,
                },
                hard: true,
            });
        self.rows = checkpoint.row;
        self.checkpoints.retain(|point| point.row < checkpoint.row);
        self.flow = Flow::counting_checkpoint(checkpoint);
        self.last_start = None;
        self.complete = self.flow.finished(self.end);
        // Prefix queries keep their captured end and bounded screen snapshot.
        // Their completed projection is published before a refresh is scheduled.
    }
    fn reset(&mut self) {
        self.scalar_widths.needed = false;
        self.rows = 0;
        self.origin = 0;
        self.rebase = None;
        self.complete = self.base == self.end;
        self.flow = Flow::counting(Cursor {
            id: self.base,
            byte: 0,
        });
        self.last_start = None;
        self.checkpoints.clear();
        self.stride = 16;
        self.query = None;
        self.navigation = None;
        self.locate = None;
        self.published = None;
        self.published_rows = 0;
    }
    pub(crate) fn len(&self) -> usize {
        self.rows
            .max(self.published_rows)
            .saturating_sub(self.origin)
    }
    pub(crate) fn complete(&self) -> bool {
        self.complete && self.rebase.is_none()
    }
    pub(crate) fn pending(&self) -> bool {
        // A hidden pane has no font callback. Suspend at the unmeasured scalar
        // without repeated wakes; the next measured draw resumes this cursor.
        !self.scalar_widths.needed
            && (!self.complete
                || self.rebase.is_some()
                || self.pending_end
                || self.query.as_ref().is_some_and(|query| !query.ready)
                || self.navigation.as_ref().is_some_and(|query| !query.ready)
                || self.locate.as_ref().is_some_and(|locate| !locate.ready))
    }
    fn sync(&mut self) -> bool {
        let (epoch, base, disk_end, revision) = {
            let transcript = self.store.lock().unwrap();
            (
                transcript.epoch(),
                transcript.base_id(),
                transcript.base_id() + transcript.len() as u64,
                transcript.serial(),
            )
        };
        let changed = revision != self.revision;
        if changed {
            self.projection_revision = self.projection_revision.wrapping_add(1);
        }
        self.revision = revision;
        self.disk_end = disk_end;
        if epoch != self.epoch {
            self.tail = None;
            self.pending_tail = None;
            self.epoch = epoch;
            self.base = base;
            self.end = self
                .tail
                .as_ref()
                .map_or(disk_end, |tail| disk_end.max(tail.end()));
            self.pending_end = false;
            self.reset();
        } else {
            if base != self.base && !self.trim_base(base) {
                // A retained suffix of one continued logical line has no
                // independent wrap boundary. Its shape genuinely needs reflow.
                self.base = base;
                if let Some(next) = self.pending_tail.take() {
                    self.tail = next;
                }
                self.end = self
                    .tail
                    .as_ref()
                    .map_or(disk_end, |tail| disk_end.max(tail.end()));
                self.pending_end = false;
                self.reset();
            }
            let latest = self.pending_tail.as_ref().unwrap_or(&self.tail);
            let desired_end = latest
                .as_ref()
                .map_or(disk_end, |tail| disk_end.max(tail.end()));
            if desired_end != self.end || self.pending_tail.is_some() {
                self.pending_end = true;
            }
            if self.complete && self.pending_end {
                self.refresh();
            }
        }
        changed
    }
    /// Eviction changes the first visible row, not the layout of every retained
    /// hard line. Keep the suffix in absolute visual coordinates and determine
    /// its new origin with a bounded prefix pass. Queries which already reached
    /// retained records continue instead of restarting under every output batch.
    fn trim_base(&mut self, base: u64) -> bool {
        let immutable_end = self
            .tail
            .as_ref()
            .map_or(self.end, |tail| tail.first.min(self.end));
        let Some(target) = self
            .checkpoints
            .iter()
            .find(|point| {
                point.cursor.id >= base && point.cursor.id + 1 < immutable_end && point.hard
            })
            .copied()
        else {
            return false;
        };
        self.base = base;
        self.rebase = Some(Rebase {
            target,
            flow: Flow::counting(Cursor { id: base, byte: 0 }),
            rows: 0,
        });
        self.checkpoints.retain(|point| point.row >= target.row);
        let retained = |flow: &Flow| {
            flow.cursor.id >= base
                && flow
                    .building
                    .as_ref()
                    .is_none_or(|build| build.start.id >= base)
        };
        if !retained(&self.flow) {
            self.rows = target.row;
            self.flow = Flow::counting_checkpoint(target);
            self.last_start = None;
            self.complete = false;
        }
        for query in [&mut self.query, &mut self.navigation] {
            if query.as_ref().is_some_and(|query| {
                !retained(&query.flow)
                    || query
                        .fragments
                        .first()
                        .is_some_and(|row| row.start.record_id < base)
            }) {
                *query = None;
            }
        }
        if self
            .locate
            .as_ref()
            .is_some_and(|query| query.anchor.record_id < base || !retained(&query.flow))
        {
            self.locate = None;
        }
        if self.published.as_ref().is_some_and(|frame| {
            frame
                .rows
                .first()
                .is_some_and(|row| row.start.record_id < base)
        }) {
            self.published = None;
        }
        true
    }
    fn checkpoint(
        points: &mut Vec<Checkpoint>,
        stride: &mut usize,
        row: usize,
        cursor: Cursor,
        hard: bool,
    ) {
        if row % *stride != 0 {
            return;
        }
        if points.len() == CHECKPOINTS {
            *stride = stride.saturating_mul(2);
            points.retain(|checkpoint| checkpoint.row % *stride == 0);
            if row % *stride != 0 {
                return;
            }
        }
        points.push(Checkpoint { row, cursor, hard });
    }
    fn before_row(&self, row: usize) -> Checkpoint {
        self.checkpoints
            .iter()
            .rev()
            .find(|checkpoint| checkpoint.row <= row)
            .copied()
            .unwrap_or(Checkpoint {
                row: self.origin,
                cursor: Cursor {
                    id: self.base,
                    byte: 0,
                },
                hard: true,
            })
    }
    fn before_anchor(&self, anchor: Anchor) -> Checkpoint {
        self.checkpoints
            .iter()
            .rev()
            .find(|checkpoint| checkpoint.cursor.anchor(self.epoch) <= anchor)
            .copied()
            .unwrap_or(Checkpoint {
                row: self.origin,
                cursor: Cursor {
                    id: self.base,
                    byte: 0,
                },
                hard: true,
            })
    }
    /// Each call yields after at most128 record reads or roughly2ms. Color-only
    /// revisions repaint projections without discarding the shape checkpoints.
    pub(crate) fn poll(&mut self) -> io::Result<bool> {
        self.poll_inner(None)
    }
    pub(crate) fn poll_with_measure(
        &mut self,
        mut measure: impl FnMut(char) -> i32,
    ) -> io::Result<bool> {
        self.poll_inner(Some(&mut measure))
    }
    fn poll_inner(&mut self, mut measure: Option<&mut dyn FnMut(char) -> i32>) -> io::Result<bool> {
        if measure.is_none() && self.scalar_widths.needed {
            return Ok(self.sync());
        }
        self.scalar_widths.needed = false;
        let mut changed = self.sync();
        let mut budget = Budget::new();
        let source = Source {
            store: &self.store,
            disk_end: self.disk_end,
            tail: self.tail.as_deref(),
        };
        if let Some(rebase) = &mut self.rebase {
            while !budget.exhausted() {
                let Some((start, _)) = rebase.flow.next(
                    &source,
                    self.end,
                    self.epoch,
                    self.metrics,
                    self.font_columns,
                    self.pixels,
                    &mut self.scalar_widths,
                    &mut measure,
                    &mut budget,
                )?
                else {
                    break;
                };
                if start == rebase.target.cursor {
                    self.origin = rebase
                        .target
                        .row
                        .checked_sub(rebase.rows)
                        .ok_or_else(|| io::Error::other("Invalid retained layout origin"))?;
                    self.rebase = None;
                    changed = true;
                    break;
                }
                if start.id > rebase.target.cursor.id
                    || start.id == rebase.target.cursor.id && start.byte > rebase.target.cursor.byte
                {
                    return Err(io::Error::other("Retained layout boundary changed"));
                }
                rebase.rows += 1;
            }
        }
        for query in [self.navigation.as_mut(), self.query.as_mut()]
            .into_iter()
            .flatten()
        {
            let source = Source {
                store: &self.store,
                disk_end: self.disk_end,
                tail: query.tail.as_deref(),
            };
            while !query.ready && !budget.exhausted() {
                let Some((_, fragment)) = query.flow.next(
                    &source,
                    query.end,
                    self.epoch,
                    self.metrics,
                    self.font_columns,
                    self.pixels,
                    &mut self.scalar_widths,
                    &mut measure,
                    &mut budget,
                )?
                else {
                    if query.flow.finished(query.end) {
                        query.ready = true;
                    }
                    break;
                };
                if query.row >= query.start {
                    let size = fragment.bytes();
                    // Empty rows still cost Fragment storage. Include unused
                    // Vec capacity in the byte budget, then grow in small exact
                    // batches instead of imposing a visible-row count limit.
                    let item = mem::size_of::<Fragment>();
                    let extra = if query.fragments.len() == query.fragments.capacity() {
                        ((MAX_VISIBLE_BYTES.saturating_sub(query.bytes + size)) / item + 1).min(32)
                    } else {
                        0
                    };
                    let slack = (query.fragments.capacity() + extra)
                        .saturating_sub(query.fragments.len() + 1)
                        * item;
                    if query.bytes.saturating_add(size).saturating_add(slack) > MAX_VISIBLE_BYTES {
                        query.ready = true;
                        break;
                    }
                    let fragments = Query::frame_mut(&mut query.fragments);
                    if extra > 0 {
                        fragments.reserve_exact(extra);
                    }
                    fragments.push(fragment);
                    query.bytes += size;
                    if query.fragments.len() == query.count {
                        query.ready = true;
                    }
                }
                query.row += 1;
                changed = true;
            }
        }
        if let Some(locate) = &mut self.locate {
            let source = Source {
                store: &self.store,
                disk_end: self.disk_end,
                tail: locate.tail.as_deref(),
            };
            while !locate.ready && !budget.exhausted() {
                let Some((_, fragment)) = locate.flow.next(
                    &source,
                    locate.end,
                    self.epoch,
                    self.metrics,
                    self.font_columns,
                    self.pixels,
                    &mut self.scalar_widths,
                    &mut measure,
                    &mut budget,
                )?
                else {
                    if locate.flow.finished(locate.end) {
                        locate.ready = true;
                    }
                    break;
                };
                let hard_end = locate.flow.cursor.id != fragment.end.record_id;
                if locate.anchor >= fragment.start
                    && (locate.anchor < fragment.end || (hard_end && locate.anchor == fragment.end))
                {
                    locate.result = Some(locate.row);
                    locate.ready = true;
                } else if locate.anchor < fragment.start {
                    locate.ready = true;
                }
                locate.row += 1;
                changed = true;
            }
        }
        while !self.complete && !budget.exhausted() {
            let Some((start, _)) = self.flow.next(
                &source,
                self.end,
                self.epoch,
                self.metrics,
                self.font_columns,
                self.pixels,
                &mut self.scalar_widths,
                &mut measure,
                &mut budget,
            )?
            else {
                if self.flow.finished(self.end) {
                    self.complete = true;
                    self.published_rows = self.rows;
                }
                break;
            };
            Self::checkpoint(
                &mut self.checkpoints,
                &mut self.stride,
                self.rows,
                start,
                self.flow.output_hard,
            );
            self.last_start = Some(start);
            self.last_hard = self.flow.output_hard;
            self.rows += 1;
            changed = true;
        }
        if self.pending() {
            (self.wake)();
        }
        Ok(changed || self.scalar_widths.needed)
    }
    /// A distant viewport may return no rows while its bounded projection is
    /// prepared by poll(). Repeating the same request returns the ready cache.
    pub(crate) fn request_visible(&mut self, start: usize, count: usize) -> io::Result<()> {
        self.sync();
        let start = start.saturating_add(self.origin);
        if count == 0 {
            self.query = None;
            self.published = None;
            return Ok(());
        }
        let mut replace = self.query.is_none();
        if let Some(query) = &self.query {
            if !query.fragments.is_empty() {
                let changed = self.published.as_ref().is_none_or(|frame| {
                    frame.start != query.start
                        || frame.revision != query.revision
                        || frame.rows.len() != query.fragments.len()
                });
                if changed {
                    self.published = Some(Published {
                        start: query.start,
                        revision: query.revision,
                        rows: query.fragments.clone(),
                        tail: query.tail.clone(),
                    });
                }
            }
            // Finish the captured prefix even if following output moves the
            // requested start. Once ready, coalesce to the newest request.
            replace = query.ready
                && (query.start != start
                    || query.count != count
                    || query.revision != self.projection_revision);
        }
        // Until eviction's prefix pass establishes the new origin, a requested
        // head row cannot be translated to absolute visual coordinates. Keep a
        // retained frame/query instead of beginning it at the former origin.
        if replace && self.rebase.is_none() && !(self.complete && start >= self.rows) {
            let checkpoint = self.before_row(start);
            self.query = Some(Query {
                start,
                count,
                row: checkpoint.row,
                flow: Flow::checkpoint(checkpoint),
                end: self.end,
                tail: self.tail.clone(),
                fragments: Arc::new(Vec::new()),
                bytes: 0,
                revision: self.projection_revision,
                ready: false,
            });
            (self.wake)();
        }
        Ok(())
    }
    /// Borrowed ownership of a published frame avoids copying every visible
    /// string/span into the Pane while retaining its exact immutable anchors.
    pub(crate) fn visible_shared(
        &mut self,
        start: usize,
        count: usize,
    ) -> io::Result<Arc<Vec<Fragment>>> {
        self.request_visible(start, count)?;
        Ok(self
            .published
            .as_ref()
            .map_or_else(|| Arc::new(Vec::new()), |frame| frame.rows.clone()))
    }
    #[cfg(test)]
    pub(crate) fn visible(&mut self, start: usize, count: usize) -> io::Result<Vec<Fragment>> {
        self.visible_shared(start, count)
            .map(|rows| (*rows).clone())
    }
    /// Actual visual start for the returned cached frame while a newer target
    /// is being prepared. The frame's anchors remain authoritative.
    pub(crate) fn cached_start(&self) -> Option<usize> {
        self.published
            .as_ref()
            .map(|frame| frame.start.saturating_sub(self.origin))
    }
    /// Selection can pin the bounded screen which produced the visible frame,
    /// rather than a newer mutable screen whose text has already changed.
    pub(crate) fn published_tail(&self) -> Option<(&vt100::Screen, u16, u16, u64, u64)> {
        let tail = self.published.as_ref()?.tail.as_ref()?;
        Some((
            tail.screen.as_ref()?,
            tail.start,
            tail.count,
            tail.source,
            tail.first,
        ))
    }
    /// The shared plaintext snapshot which produced the visible frame. It is
    /// independent of the command's newer mutable line when selection starts.
    pub(crate) fn published_plain_tail(&self) -> Option<(Arc<str>, u64, u64)> {
        let tail = self.published.as_ref()?.tail.as_ref()?;
        Some((tail.plain.as_ref()?.clone(), tail.source, tail.first))
    }
    pub(crate) fn row_at(&mut self, anchor: Anchor) -> io::Result<Option<usize>> {
        self.sync();
        if anchor.epoch != self.epoch
            || anchor.record_id < self.base
            || anchor.record_id >= self.end
        {
            return Ok(None);
        }
        if self.rebase.is_some() {
            return Ok(None);
        }
        if let Some(locate) = &self.locate {
            if locate.anchor == anchor {
                return Ok(if locate.ready {
                    locate.result.map(|row| row.saturating_sub(self.origin))
                } else {
                    None
                });
            }
        }
        let checkpoint = self.before_anchor(anchor);
        self.locate = Some(Locate {
            anchor,
            row: checkpoint.row,
            flow: Flow::checkpoint(checkpoint),
            end: self.end,
            tail: self.tail.clone(),
            result: None,
            ready: false,
        });
        (self.wake)();
        Ok(None)
    }
    pub(crate) fn neighbour(
        &mut self,
        anchor: Anchor,
        delta: i32,
        preferred_col: u16,
    ) -> io::Result<Option<Anchor>> {
        let Some(row) = self.row_at(anchor)? else {
            return Ok(None);
        };
        let target = row
            .saturating_add_signed(delta as isize)
            .min(self.len().saturating_sub(1));
        Ok(self
            .navigation_row(target)?
            .map(|fragment| fragment.anchor_column(preferred_col)))
    }
    pub(crate) fn row_edge(&mut self, anchor: Anchor, end: bool) -> io::Result<Option<Anchor>> {
        let Some(row) = self.row_at(anchor)? else {
            return Ok(None);
        };
        Ok(self
            .navigation_row(row)?
            .map(|fragment| if end { fragment.end } else { fragment.start }))
    }
    fn navigation_row(&mut self, row: usize) -> io::Result<Option<Fragment>> {
        let row = row.saturating_add(self.origin);
        if let Some(query) = &self.navigation {
            if query.start == row {
                return Ok(if query.ready {
                    query.fragments.first().cloned()
                } else {
                    None
                });
            }
        }
        let checkpoint = self.before_row(row);
        self.navigation = Some(Query {
            start: row,
            count: 1,
            row: checkpoint.row,
            flow: Flow::checkpoint(checkpoint),
            end: self.end,
            tail: self.tail.clone(),
            fragments: Arc::new(Vec::new()),
            bytes: 0,
            revision: self.projection_revision,
            ready: false,
        });
        (self.wake)();
        Ok(None)
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
