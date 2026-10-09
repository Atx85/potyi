// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::layout::Fragment;
use super::{output_selection::Range as OutputRange, transcript::Anchor};
use sdl3::{
    pixels::Color,
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    ttf::{Font, FontStyle},
    video::{Window, WindowContext},
};
use std::collections::HashMap;

pub(super) const MARGIN: i32 = 8;
// Status lines belong to the editor's terminal chrome, outside the body.
pub(super) const FOOTER: i32 = 0;
const CACHE_BYTES: usize = 8 * 1024 * 1024;
const CACHE_ENTRIES: usize = 512;
type Key = (String, (u8, u8, u8), u8);

struct Glyph<'a> {
    texture: Texture<'a>,
    bytes: usize,
    used: u64,
}
#[derive(Default)]
pub(crate) struct GlyphCache<'a> {
    glyphs: HashMap<Key, Glyph<'a>>,
    bytes: usize,
    clock: u64,
}

impl<'a> GlyphCache<'a> {
    #[cfg(test)]
    pub(crate) fn has_glyphs(&self) -> bool {
        !self.glyphs.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn draw(
        &mut self,
        canvas: &mut Canvas<Window>,
        creator: &'a TextureCreator<WindowContext>,
        font: &mut Font<'_>,
        text: &str,
        color: Color,
        style: u8,
        x: i32,
        y: i32,
        scale: f32,
        logical_size: Option<(u32, u32)>,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        let key = (text.to_string(), (color.r, color.g, color.b), style);
        self.clock = self.clock.wrapping_add(1);
        if !self.glyphs.contains_key(&key) {
            let flags = if style & 1 != 0 {
                FontStyle::BOLD
            } else {
                FontStyle::NORMAL
            } | if style & 2 != 0 {
                FontStyle::ITALIC
            } else {
                FontStyle::NORMAL
            };
            font.set_style(flags);
            let surface = font
                .render(text)
                .blended(color)
                .map_err(|error| error.to_string())?;
            let texture = creator
                .create_texture_from_surface(&surface)
                .map_err(|error| error.to_string())?;
            let query = texture.query();
            let bytes = query.width as usize * query.height as usize * 4 + text.len() + 128;
            while self.glyphs.len() >= CACHE_ENTRIES
                || self.bytes.saturating_add(bytes) > CACHE_BYTES
            {
                let Some(oldest) = self
                    .glyphs
                    .iter()
                    .min_by_key(|(_, glyph)| glyph.used)
                    .map(|(key, _)| key.clone())
                else {
                    break;
                };
                self.bytes -= self.glyphs.remove(&oldest).unwrap().bytes;
            }
            if bytes > CACHE_BYTES {
                return Err("Terminal glyph exceeds its texture budget".into());
            }
            self.bytes += bytes;
            self.glyphs.insert(
                key.clone(),
                Glyph {
                    texture,
                    bytes,
                    used: self.clock,
                },
            );
        }
        let glyph = self.glyphs.get_mut(&key).unwrap();
        glyph.used = self.clock;
        let size = glyph.texture.query();
        let (width, height) = logical_size.unwrap_or_else(|| {
            (
                (size.width as f32 / scale).round().max(1.0) as u32,
                (size.height as f32 / scale).round().max(1.0) as u32,
            )
        });
        canvas
            .copy(&glyph.texture, None, Rect::new(x, y, width, height))
            .map_err(|error| error.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Point {
    pub row: u16,
    pub col: u16,
}
#[derive(Default)]
pub(super) struct Selection {
    pub anchor: Option<Point>,
    pub end: Option<Point>,
    pub dragging: bool,
    pub spans: Vec<(u16, u16, u16)>,
}

impl Selection {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
    fn ordered(&self) -> Option<(Point, Point)> {
        let (a, b) = (self.anchor?, self.end?);
        Some(if a <= b { (a, b) } else { (b, a) })
    }
    fn contains(&self, point: Point) -> bool {
        if !self.spans.is_empty() {
            return self.spans.iter().any(|&(row, start, end)| {
                point.row == row && point.col >= start && point.col < end
            });
        }
        self.ordered()
            .is_some_and(|(start, end)| start != end && point >= start && point < end)
    }
    pub(super) fn text(&self, screen: &vt100::Screen) -> String {
        self.ordered().map_or_else(String::new, |(start, end)| {
            screen.contents_between(start.row, start.col, end.row, end.col)
        })
    }
}

/// Normal output follows the editor font advances, rather than round-tripping
/// text through a wide-cell VT grid. Only the bounded visible projection is
/// painted; the terminal transcript and its anchors remain on disk.
pub(super) fn draw_fragments<'a>(
    canvas: &mut Canvas<Window>,
    creator: &'a TextureCreator<WindowContext>,
    font: &mut Font<'_>,
    cache: &mut GlyphCache<'a>,
    fragments: &[Fragment],
    selection: &Selection,
    output_range: Option<OutputRange>,
    cursor: Option<(Point, Anchor)>,
    cell_width: i32,
    cell_height: i32,
    scale: f32,
    focused: bool,
    area: Rect,
) -> Result<(), String> {
    canvas.set_viewport(area);
    canvas.set_draw_color(Color::RGB(30, 30, 30));
    canvas
        .fill_rect(Rect::new(0, 0, area.width(), area.height()))
        .map_err(|error| error.to_string())?;
    // Renderer insets this body by4px; :term clips output10px from the pane
    // edge, including selected newlines and overlong whitespace backgrounds.
    let clip_margin = MARGIN - 2;
    canvas.set_clip_rect(Rect::new(
        clip_margin,
        0,
        area.width().saturating_sub((clip_margin * 2) as u32).max(1),
        area.height(),
    ));
    font.set_style(FontStyle::NORMAL);
    // Renderer supplies ceil(7*logical_font_height/5). Inverting that exact
    // ratio avoids fractional-DPI rounding moving the glyphs or underlines.
    let glyph_height = (cell_height * 5 / 7).max(1);
    let rows = ((area.height() as i32 - MARGIN) / cell_height).max(1) as usize;
    for &(row, start, end) in &selection.spans {
        if usize::from(row) >= rows || end <= start {
            continue;
        }
        if fragments
            .get(usize::from(row))
            .is_some_and(zero_column_fragment)
        {
            continue;
        }
        canvas.set_draw_color(Color::RGB(55, 78, 110));
        canvas
            .fill_rect(Rect::new(
                MARGIN + i32::from(start) * cell_width,
                MARGIN + i32::from(row) * cell_height,
                i32::from(end - start).saturating_mul(cell_width).max(1) as u32,
                cell_height as u32,
            ))
            .map_err(|error| error.to_string())?;
    }
    // A row made only of combining marks has logical anchors but no columns.
    // Its caret/selection still follow the original font's prefix ink width.
    if let Some(range) = output_range.filter(|range| !range.is_empty()) {
        for (row, fragment) in fragments.iter().take(rows).enumerate() {
            if !zero_column_fragment(fragment)
                || range.end <= fragment.start
                || range.start > fragment.end
            {
                continue;
            }
            let left = if range.start <= fragment.start {
                0
            } else {
                zero_column_prefix_width(fragment, range.start, font, scale)?
            };
            let right = if range.end > fragment.end {
                zero_column_prefix_width(fragment, fragment.end, font, scale)?
                    .saturating_add(cell_width as u32)
            } else {
                zero_column_prefix_width(fragment, range.end, font, scale)?
            };
            if right > left {
                canvas.set_draw_color(Color::RGB(55, 78, 110));
                canvas
                    .fill_rect(Rect::new(
                        MARGIN + left as i32,
                        MARGIN + row as i32 * cell_height,
                        right - left,
                        cell_height as u32,
                    ))
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    if focused
        && let Some((point, anchor)) = cursor.filter(|(point, _)| usize::from(point.row) < rows)
    {
        let cursor_x = if let Some(fragment) = fragments
            .get(usize::from(point.row))
            .filter(|fragment| zero_column_fragment(fragment))
        {
            zero_column_prefix_width(fragment, anchor, font, scale)? as i32
        } else {
            i32::from(point.col) * cell_width
        };
        canvas.set_draw_color(Color::RGB(245, 245, 245));
        canvas
            .fill_rect(Rect::new(
                MARGIN + cursor_x,
                MARGIN + i32::from(point.row) * cell_height,
                2,
                cell_height as u32,
            ))
            .map_err(|error| error.to_string())?;
    }
    for (row, fragment) in fragments.iter().take(rows).enumerate() {
        let y = MARGIN + row as i32 * cell_height;
        for segment in &fragment.segments {
            let x = MARGIN + i32::from(segment.column) * cell_width;
            if x >= area.width() as i32 - MARGIN {
                continue;
            }
            let remaining = ((area.width() as i32 - MARGIN - x) / cell_width).max(0) as usize;
            let full = &fragment.text[segment.range.clone()];
            let mut end = 0;
            let mut columns = 0;
            // A pathological zero-width sequence or whitespace record must not
            // allocate a transcript-sized text texture on the UI thread.
            for (offset, character) in full.char_indices() {
                let width = fragment.text_columns(&full[offset..offset + character.len_utf8()]);
                if columns + width > remaining || offset + character.len_utf8() > 4096 {
                    break;
                }
                columns += width;
                end = offset + character.len_utf8();
            }
            let text = &full[..end];
            if text.is_empty() {
                continue;
            }
            // A standalone combining mark has no text columns, but the editor
            // font can still give it visible ink. Preserve that ink using the
            // same raster-to-logical measurement as the original renderer.
            let logical_width = if columns == 0 {
                let (width, _) = font.size_of(text).map_err(|error| error.to_string())?;
                (width as f32 / scale).round() as u32
            } else {
                (columns as i32 * cell_width).max(1) as u32
            };
            if logical_width == 0 {
                continue;
            }
            let foreground = segment
                .color
                .map(|(r, g, b)| Color::RGB(r, g, b))
                .unwrap_or_else(|| color(segment.style.fg, true));
            cache.draw(
                canvas,
                creator,
                font,
                text,
                foreground,
                0,
                x,
                y,
                scale,
                Some((logical_width, glyph_height as u32)),
            )?;
            if segment.style.underline {
                canvas.set_draw_color(foreground);
                canvas
                    .fill_rect(Rect::new(x, y + glyph_height - 2, logical_width, 1))
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    font.set_style(FontStyle::NORMAL);
    canvas.set_clip_rect(None);
    canvas.set_viewport(None);
    Ok(())
}

fn zero_column_fragment(fragment: &Fragment) -> bool {
    !fragment.text.is_empty() && fragment.text_columns(&fragment.text) == 0
}

fn zero_column_prefix_width(
    fragment: &Fragment,
    anchor: Anchor,
    font: &Font<'_>,
    scale: f32,
) -> Result<u32, String> {
    let mut end = if anchor >= fragment.end {
        fragment.text.len()
    } else {
        0
    };
    for segment in &fragment.segments {
        if anchor.epoch == segment.start.epoch
            && anchor.record_id == segment.record_id
            && anchor.utf8_byte_offset >= segment.start.utf8_byte_offset
            && anchor.utf8_byte_offset <= segment.end.utf8_byte_offset
        {
            end = segment.range.start
                + anchor
                    .utf8_byte_offset
                    .saturating_sub(segment.start.utf8_byte_offset)
                    .min(segment.range.len());
            break;
        }
    }
    // Match the normal glyph projection's fixed per-run texture input budget.
    end = end.min(4096);
    while !fragment.text.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        return Ok(0);
    }
    let (width, _) = font
        .size_of(&fragment.text[..end])
        .map_err(|error| error.to_string())?;
    Ok((width as f32 / scale).round() as u32)
}

#[cfg(test)]
pub(super) fn draw<'a>(
    canvas: &mut Canvas<Window>,
    creator: &'a TextureCreator<WindowContext>,
    font: &mut Font<'_>,
    cache: &mut GlyphCache<'a>,
    screen: &vt100::Screen,
    selection: &Selection,
    status: Option<&str>,
    cell_width: i32,
    cell_height: i32,
    scale: f32,
    focused: bool,
) -> Result<(), String> {
    let (width, height) = canvas.window().size();
    draw_area(
        canvas,
        creator,
        font,
        cache,
        screen,
        selection,
        status,
        cell_width,
        cell_height,
        scale,
        focused,
        true,
        Rect::new(0, 0, width, height),
    )?;
    canvas.present();
    Ok(())
}

pub(super) fn draw_area<'a>(
    canvas: &mut Canvas<Window>,
    creator: &'a TextureCreator<WindowContext>,
    font: &mut Font<'_>,
    cache: &mut GlyphCache<'a>,
    screen: &vt100::Screen,
    selection: &Selection,
    _status: Option<&str>,
    cell_width: i32,
    cell_height: i32,
    scale: f32,
    focused: bool,
    raw_grid: bool,
    area: Rect,
) -> Result<(), String> {
    canvas.set_viewport(area);
    canvas.set_draw_color(Color::RGB(30, 30, 30));
    canvas
        .fill_rect(Rect::new(0, 0, area.width(), area.height()))
        .map_err(|e| e.to_string())?;
    let (rows, cols) = screen.size();
    let glyph_height = (font.height() as f32 / scale).round().max(1.0) as i32;
    // Paint backgrounds first: a wide glyph extends into its continuation cell,
    // whose background must not be painted over the glyph afterwards.
    for row in 0..rows {
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            let mut bg = color(
                if cell.inverse() {
                    cell.fgcolor()
                } else {
                    cell.bgcolor()
                },
                cell.inverse(),
            );
            if selection.contains(Point { row, col }) {
                bg = Color::RGB(55, 78, 110);
            }
            if bg != Color::RGB(30, 30, 30) {
                canvas.set_draw_color(bg);
                canvas
                    .fill_rect(Rect::new(
                        MARGIN + i32::from(col) * cell_width,
                        MARGIN + i32::from(row) * cell_height,
                        cell_width as u32,
                        cell_height as u32,
                    ))
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    for row in 0..rows {
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            let x = MARGIN + i32::from(col) * cell_width;
            let y = MARGIN + i32::from(row) * cell_height;
            let mut fg = color(
                if cell.inverse() {
                    cell.bgcolor()
                } else {
                    cell.fgcolor()
                },
                !cell.inverse(),
            );
            if cell.dim() {
                fg = Color::RGB(fg.r / 2, fg.g / 2, fg.b / 2);
            }
            // Wide cells occupy two columns, with a continuation in the second.
            if !cell.is_wide_continuation() && cell.has_contents() {
                canvas.set_clip_rect(Rect::new(
                    x,
                    y,
                    cell_width as u32 * if cell.is_wide() { 2 } else { 1 },
                    cell_height as u32,
                ));
                cache.draw(
                    canvas,
                    creator,
                    font,
                    cell.contents(),
                    fg,
                    u8::from(cell.bold()) | (u8::from(cell.italic()) << 1),
                    x,
                    y,
                    scale,
                    None,
                )?;
                canvas.set_clip_rect(None);
            }
            if cell.underline() {
                canvas.set_draw_color(fg);
                canvas
                    .fill_rect(Rect::new(x, y + glyph_height - 2, cell_width as u32, 1))
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    if !screen.hide_cursor() && screen.scrollback() == 0 {
        let (row, col) = screen.cursor_position();
        let cursor = Rect::new(
            MARGIN + i32::from(col) * cell_width,
            MARGIN + i32::from(row) * cell_height,
            cell_width as u32,
            cell_height as u32,
        );
        canvas.set_draw_color(if raw_grid {
            Color::RGB(190, 200, 190)
        } else {
            Color::RGB(245, 245, 245)
        });
        if !raw_grid && focused {
            canvas
                .fill_rect(Rect::new(cursor.x(), cursor.y(), 2, cell_height as u32))
                .map_err(|error| error.to_string())?;
        } else if focused {
            canvas
                .fill_rect(Rect::new(
                    cursor.x(),
                    cursor.y() + cell_height - 2,
                    cell_width as u32,
                    2,
                ))
                .map_err(|error| error.to_string())?;
        } else if raw_grid {
            canvas
                .draw_rect(cursor)
                .map_err(|error| error.to_string())?;
        }
    }
    canvas.set_clip_rect(None);
    font.set_style(FontStyle::NORMAL);
    canvas.set_viewport(None);
    Ok(())
}

fn color(value: vt100::Color, foreground: bool) -> Color {
    const PALETTE: [(u8, u8, u8); 16] = [
        (30, 30, 30),
        (205, 80, 80),
        (90, 185, 110),
        (205, 175, 80),
        (90, 130, 210),
        (180, 110, 195),
        (80, 180, 185),
        (205, 205, 205),
        (105, 105, 105),
        (245, 125, 125),
        (130, 225, 150),
        (245, 215, 125),
        (130, 170, 245),
        (225, 155, 235),
        (130, 225, 230),
        (245, 245, 245),
    ];
    let (r, g, b) = match value {
        vt100::Color::Default => {
            if foreground {
                (215, 220, 215)
            } else {
                (30, 30, 30)
            }
        }
        vt100::Color::Rgb(r, g, b) => (r, g, b),
        vt100::Color::Idx(index) if index < 16 => PALETTE[index as usize],
        vt100::Color::Idx(index) if index < 232 => {
            let n = index - 16;
            let component = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (component(n / 36), component(n / 6 % 6), component(n % 6))
        }
        vt100::Color::Idx(index) => {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        }
    };
    Color::RGB(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reversed_selection_keeps_unicode_and_soft_wrapped_lines() {
        let mut parser = vt100::Parser::new(3, 6, 0);
        parser.process("ab界défg".as_bytes());
        let selection = Selection {
            anchor: Some(Point { row: 1, col: 2 }),
            end: Some(Point { row: 0, col: 0 }),
            dragging: false,
            spans: Vec::new(),
        };
        assert_eq!(selection.text(parser.screen()), "ab界défg");
        assert!(!Selection::default().contains(Point { row: 0, col: 0 }));
    }

    #[test]
    #[ignore = "requires SDL video (SDL_VIDEODRIVER=dummy is supported)"]
    fn experimental_terminal_render_keeps_ansi_colors_wide_backgrounds_and_bounded_cache() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video
            .window("Terminal render check", 800, 450)
            .build()
            .unwrap();
        let mut canvas = sdl3::render::create_renderer(window, None).unwrap();
        let creator = canvas.texture_creator();
        let ttf = sdl3::ttf::init().unwrap();
        let mut font = ttf
            .load_font_from_iostream(
                sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(),
                16.0,
            )
            .unwrap();
        let cell_width = font.size_of("M").unwrap().0 as i32;
        let cell_height = font.height() + 2;
        let mut parser = vt100::Parser::new(18, 76, 0);
        parser.process(
            concat!(
                "Pötyi · experimental terminal\r\n\r\n",
                "$ read name\r\nAda\r\nHello, Ada\r\n\r\n",
                "\x1b[32mPersistent shell · interactive input\x1b[0m\r\n",
                "\x1b[31mANSI colors\x1b[0m · é · Unicode\r\n",
                "\x1b[48;2;10;40;70m界é\x1b[0m wide cells with a background\r\n",
                "\r\n$ "
            )
            .as_bytes(),
        );
        let mut cache = GlyphCache::default();
        draw(
            &mut canvas,
            &creator,
            &mut font,
            &mut cache,
            parser.screen(),
            &Selection::default(),
            Some("Opt-in experiment · Existing :term remains independent"),
            cell_width,
            cell_height,
            1.0,
            true,
        )
        .unwrap();
        let pixels = canvas
            .read_pixels(None)
            .unwrap()
            .convert_format(sdl3::pixels::PixelFormat::RGBA32)
            .unwrap();
        assert!(pixels.with_lock(|bytes| {
            bytes
                .chunks_exact(4)
                .any(|pixel| pixel[..3] == [205, 80, 80])
        }));
        assert!(pixels.with_lock(|bytes| {
            bytes
                .chunks_exact(4)
                .any(|pixel| pixel[..3] == [10, 40, 70])
        }));
        assert!(cache.bytes <= CACHE_BYTES);
        assert!(cache.glyphs.len() <= CACHE_ENTRIES);
        if let Some(path) = std::env::var_os("POTYI_TERM_PREVIEW") {
            pixels.save_bmp(path).unwrap();
        }
    }
}
