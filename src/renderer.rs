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


mod completion;

use sdl3::event::Event;
use sdl3::pixels::Color;
use sdl3::rect::{Point, Rect};
use sdl3::render::{FRect, TextureCreator};
use sdl3::video::WindowContext;
use crate::terminal_text_cache::TextCache;
use sdl3::sys::render::
    SDL_LOGICAL_PRESENTATION_STRETCH;
use sdl3::{
    render::Canvas,
    ttf::Font,
    video::Window,
};

use crate::dpi_text::DpiTextMetrics;
use crate::command_bar::{
    CommandBar,
    CommandBarHit,
    COMMAND_BAR_MARGIN,
    COMMAND_FONT_SIZE,
    COMMAND_INPUT_HEIGHT,
    COMMAND_RUN_WIDTH,
    COMMAND_SUGGESTION_HEIGHT,
};
use crate::config::LineNumberMode;
use crate::line_numbers::LineNumbers;
use crate::piece_table::PieceTable;
use crate::search_ui::{SearchField, SearchUi};
use crate::syntax::SyntaxDefinition;
use crate::terminal::{EntryKind, OutputCommand, Terminal, TerminalAction};
use crate::terminal_layout::{TerminalLayout, WrapMetrics};
use crate::window::{
    logical_render_size,
    window_coordinate_scale,
    WindowHitTestState,
    TITLE_BAR_HEIGHT,
    WINDOW_BUTTON_WIDTH,
};

const WINDOW_BUTTONS_WIDTH: i32 =
    WINDOW_BUTTON_WIDTH * 3;

const SEARCH_BAR_MARGIN: i32 = 8;
const SEARCH_ROW_HEIGHT: i32 = 42;
const MAX_TEXT_TEXTURE_WIDTH: i32 = 8192;
const TEXT_TEXTURE_SAFETY_MARGIN: i32 = 64;
const TERMINAL_BAR_HEIGHT: i32 = 46;
const TERMINAL_LINE_HEIGHT_SCALE_NUMERATOR: i32 = 7;
const TERMINAL_LINE_HEIGHT_SCALE_DENOMINATOR: i32 = 5;
const TERMINAL_BAR_MARGIN: i32 = 8;
const TERMINAL_ACTION_WIDTH: i32 = 84;
const TERMINAL_CLEAR_WIDTH: i32 = 72;
const TERMINAL_EDITOR_WIDTH: i32 = 82;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WindowControl {
    None,
    Minimize,
    Maximize,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalHit {
    Outside,
    Output,
    Input,
    StopOrRunAgain,
    Clear,
    Editor,
}

#[derive(Clone, Copy)]
struct CursorHitTestLayout {
    text_left: i32,
    text_top: i32,
    right: i32,
    bottom: i32,
    line_height: i32,
    visible_lines: usize,
    scroll_line: usize,
    scroll_x: i32,
    char_width: i32,
    tab_width: usize,
}

struct StoredViewState {
    scroll_line: usize,
    scroll_x: i32,
    cursor_line: usize,
    cursor_column: usize,
    scroll_start_byte: usize,
    scroll_start_line: usize,
    scroll_start_valid: bool,
    scroll_revision: u64,
    syntax: Option<SyntaxDefinition>,
}

impl StoredViewState {
    fn empty() -> Self {
        Self {
            scroll_line: 0,
            scroll_x: 0,
            cursor_line: 0,
            cursor_column: 0,
            scroll_start_byte: 0,
            scroll_start_line: 0,
            scroll_start_valid: true,
            scroll_revision: 0,
            syntax: None,
        }
    }
}

// ==========================================================================
// Renderer
// ==========================================================================

pub struct Renderer<'a> {
    completion: Option<crate::lsp_ui::Display>,
    completion_bounds: Option<Rect>,
    canvas: Canvas<Window>,
    texture_creator: &'a TextureCreator<WindowContext>,
    terminal_text_cache: TextCache<'a>,
    cache_terminal_text: bool,
    font: Font<'a>,
    raster_font: Font<'a>,
    command_font: Font<'a>,
    command_raster_font: Font<'a>,
    command_char_width: i32,
    dpi_text: DpiTextMetrics,
    logical_font_size: f32,

    scroll_line: usize,
    scroll_x: i32,

    char_width: i32,

    window_width: i32,
    window_height: i32,
    bottom_inset: i32,
    terminal_pane: usize,
    terminal_bottom_inset: i32,
    terminal_reserved_height: i32,
    rendering_terminal_pane: bool,
    rendering_document_pane: bool,
    terminal_layout: TerminalLayout,
    terminal_layout_origin: (u64, usize),

    tab_width: usize,
    line_number_mode: LineNumberMode,

    cursor_line: usize,
    cursor_column: usize,

    scroll_start_byte: usize,
    scroll_start_line: usize,
    scroll_start_valid: bool,
    scroll_revision: u64,

    window_hit_test: WindowHitTestState,
    hovered_window_control: WindowControl,
    syntax: Option<SyntaxDefinition>,

    inactive_view: StoredViewState,
    split_mode: bool,
    split_ratio: f64,
    active_pane: usize,
    mode_label: Option<&'static str>,
}

impl<'a> Renderer<'a> {
    pub fn new(
        mut canvas: Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        font: Font<'a>,
        raster_font: Font<'a>,
        logical_font_size: f32,
        command_fonts: (Font<'a>, Font<'a>),
        window_hit_test: WindowHitTestState,
    ) -> Result<Self, String> {
        let (
            window_width,
            window_height,
        ) = logical_render_size(
            canvas.window(),
        );

        canvas
            .set_logical_size(
                window_width,
                window_height,
                SDL_LOGICAL_PRESENTATION_STRETCH,
            )
            .map_err(|e| e.to_string())?;

        let dpi_text =
            DpiTextMetrics::new(
                canvas.window()
                    .display_scale(),
            );

        raster_font
            .set_size(
                dpi_text.raster_font_size(
                    logical_font_size,
                )
            )
            .map_err(|e| e.to_string())?;

        let char_width =
            font.size_of("M")
                .map(|(width, _)| width as i32)
                .unwrap_or(11);

        let (command_font, command_raster_font) = command_fonts;
        command_font.set_size(COMMAND_FONT_SIZE).map_err(|e| e.to_string())?;
        command_raster_font.set_size(dpi_text.raster_font_size(COMMAND_FONT_SIZE))
            .map_err(|e| e.to_string())?;
        let command_char_width = command_font.size_of("M")
            .map(|(width, _)| width as i32).unwrap_or(8);

        window_hit_test.set_metrics(
            window_width as i32,
            window_coordinate_scale(
                canvas.window(),
            ),
        );

        Ok(Self {
            completion: None,
            completion_bounds: None,
            canvas,
            texture_creator,
            terminal_text_cache: TextCache::default(),
            cache_terminal_text: false,
            font,
            raster_font,
            command_font,
            command_raster_font,
            command_char_width,
            dpi_text,
            logical_font_size,

            scroll_line: 0,
            scroll_x: 0,

            char_width,

            window_width: window_width as i32,
            window_height: window_height as i32,
            bottom_inset: 0,
            terminal_pane: 0,
            terminal_bottom_inset: 0,
            terminal_reserved_height: 0,
            rendering_terminal_pane: false,
            rendering_document_pane: false,
            terminal_layout: TerminalLayout::default(),
            terminal_layout_origin: (0, 0),

            tab_width: 4,
            line_number_mode:
                LineNumberMode::default(),

            cursor_line: 0,
            cursor_column: 0,

            scroll_start_byte: 0,
            scroll_start_line: 0,
            scroll_start_valid: true,
            scroll_revision: 0,

            window_hit_test,
            hovered_window_control: WindowControl::None,
            syntax: None,

            inactive_view: StoredViewState::empty(),
            split_mode: false,
            split_ratio: 0.5,
            active_pane: 0,
            mode_label: None,
        })
    }

    pub fn set_mode_label(
        &mut self,
        label: Option<&'static str>,
    ) {
        self.mode_label = label;
    }

    pub(crate) fn is_split(&self) -> bool {
        self.split_mode
    }

    pub fn set_split_mode(
        &mut self,
        split_mode: bool,
    ) {
        self.split_mode = split_mode;
    }

    pub(crate) fn place_terminal_in_active_pane(&mut self) {
        self.terminal_pane = self.active_pane;
    }

    pub(crate) fn terminal_focused(&self, terminal: &Terminal) -> bool {
        terminal.is_active() && self.terminal_pane == self.active_pane
    }

    pub(crate) fn terminal_visible(&self, terminal: &Terminal) -> bool {
        terminal.is_active() && (self.split_mode || self.terminal_pane == self.active_pane)
    }

    pub(crate) fn terminal_is_split(&self, terminal: &Terminal) -> bool {
        self.split_mode && self.terminal_focused(terminal)
    }

    pub(crate) fn terminal_pane(&self) -> usize {
        self.terminal_pane
    }

    fn terminal_bounds(&self) -> (i32, i32) {
        if self.rendering_terminal_pane {
            (0, self.window_width)
        } else {
            self.pane_bounds(self.terminal_pane)
        }
    }

    fn terminal_height(&self) -> i32 {
        self.window_height
            - if self.rendering_terminal_pane {
                0
            } else {
                self.terminal_reserved_height
            }
    }

    pub fn set_active_pane(
        &mut self,
        active_pane: usize,
    ) {
        self.active_pane = active_pane.min(1);
    }

    pub fn pane_at(
        &self,
        x: i32,
    ) -> usize {
        if !self.split_mode {
            return self.active_pane;
        }

        if x < self.split_divider() {
            0
        } else {
            1
        }
    }

    pub fn pane_at_point(
        &self,
        x: i32,
        y: i32,
    ) -> Option<usize> {
        if y < TITLE_BAR_HEIGHT
            || y >= self.window_height
                .saturating_sub(self.bottom_inset)
        {
            return None;
        }

        Some(self.pane_at(x))
    }

    fn pane_bounds(
        &self,
        pane: usize,
    ) -> (i32, i32) {
        if !self.split_mode {
            return (0, self.window_width.max(1));
        }

        split_pane_bounds(self.window_width, pane, self.split_ratio)
    }

    pub(crate) fn split_divider(&self) -> i32 {
        split_pane_bounds(self.window_width, 1, self.split_ratio).0
    }

    pub(crate) fn split_divider_hit(&self, x: i32, y: i32) -> bool {
        self.split_mode && y >= TITLE_BAR_HEIGHT
            && y < self.window_height - self.bottom_inset
            && (x - self.split_divider()).abs() <= 5
    }

    pub(crate) fn resize_split(&mut self, x: i32) {
        let minimum = 120.min(self.window_width.max(2) / 2);
        let x = x.clamp(minimum, self.window_width.max(2) - minimum);
        self.split_ratio = x as f64 / self.window_width.max(2) as f64;
        self.invalidate_scroll_cache();
        self.inactive_view.scroll_start_valid = false;
    }

    /// Clamp selection drags to the originating pane, including its gutter.
    pub(crate) fn selection_drag_outside(&self, x: i32, y: i32) -> bool {
        let (left, width) = self.pane_bounds(self.active_pane);
        x < left || x >= left + width || y < TITLE_BAR_HEIGHT + 8
            || y >= self.window_height - self.bottom_inset
    }

    pub(crate) fn drag_cursor_target(&mut self, table: &mut PieceTable, x: i32, y: i32)
        -> Result<Option<(usize, usize)>, String>
    {
        let (left, width) = self.pane_bounds(self.active_pane);
        let text_left = left + self.text_left(table)?;
        let top = TITLE_BAR_HEIGHT + 8;
        // The padding below the last complete text row is not a cursor target.
        let bottom = (top + self.visible_line_count() as i32 * self.font.height().max(1))
            .min(self.window_height - self.bottom_inset).max(top + 1);
        if y < top { self.scroll_by(-1, table); }
        if y >= bottom { self.scroll_by(1, table); }
        if x < left { self.scroll_horizontal(-self.char_width); }
        if x >= left + width { self.scroll_horizontal(self.char_width); }
        let right = (left + width - 1).max(text_left);
        self.cursor_target_at(table, x.clamp(text_left, right), y.clamp(top, bottom - 1))
    }

    fn active_content_width(&self) -> i32 {
        // During a pane render, window_width is already local to that pane.
        // Splitting it again clips text halfway across the editor viewport.
        if self.rendering_document_pane {
            self.window_width
        } else {
            self.pane_bounds(self.active_pane).1
        }
    }

    fn active_local_x(&self, x: i32) -> i32 {
        let (left, _) =
            self.pane_bounds(self.active_pane);

        x.saturating_sub(left)
    }

    pub fn swap_view(&mut self) {
        std::mem::swap(
            &mut self.scroll_line,
            &mut self.inactive_view.scroll_line,
        );
        std::mem::swap(
            &mut self.scroll_x,
            &mut self.inactive_view.scroll_x,
        );
        std::mem::swap(
            &mut self.cursor_line,
            &mut self.inactive_view.cursor_line,
        );
        std::mem::swap(
            &mut self.cursor_column,
            &mut self.inactive_view.cursor_column,
        );
        std::mem::swap(
            &mut self.scroll_start_byte,
            &mut self.inactive_view.scroll_start_byte,
        );
        std::mem::swap(
            &mut self.scroll_start_line,
            &mut self.inactive_view.scroll_start_line,
        );
        std::mem::swap(
            &mut self.scroll_start_valid,
            &mut self.inactive_view.scroll_start_valid,
        );
        std::mem::swap(&mut self.scroll_revision, &mut self.inactive_view.scroll_revision);
        std::mem::swap(
            &mut self.syntax,
            &mut self.inactive_view.syntax,
        );
    }

    pub fn set_tab_width(
        &mut self,
        tab_width: usize,
    ) {
        self.tab_width = tab_width.max(1);
    }

    pub fn set_line_number_mode(
        &mut self,
        mode: LineNumberMode,
    ) {
        self.line_number_mode = mode;
    }

    pub fn set_font_size(
        &mut self,
        logical_font_size: f32,
    ) -> Result<(), String> {
        if (self.logical_font_size
            - logical_font_size)
            .abs()
            < f32::EPSILON
        {
            return Ok(());
        }

        let previous =
            self.logical_font_size;

        self.font
            .set_size(logical_font_size)
            .map_err(|error| {
                error.to_string()
            })?;

        if let Err(error) =
            self.raster_font.set_size(
                self.dpi_text
                    .raster_font_size(
                        logical_font_size,
                    ),
            )
        {
            let _ = self.font
                .set_size(previous);

            return Err(error.to_string());
        }

        self.terminal_text_cache.clear();
        self.logical_font_size =
            logical_font_size;

        self.char_width =
            self.font.size_of("M")
                .map(|(width, _)| {
                    width as i32
                })
                .unwrap_or_else(|_| {
                    logical_font_size
                        .round()
                        .max(1.0) as i32
                });

        self.scroll_x = 0;
        self.invalidate_scroll_cache();
        self.inactive_view.scroll_x = 0;
        self.inactive_view.scroll_start_valid = false;

        Ok(())
    }

    fn tab_advance(
        visual_column: usize,
        tab_width: usize,
    ) -> usize {
        let tab_width = tab_width.max(1);

        tab_width
            - (visual_column % tab_width)
    }

    fn visual_advance(
        character: char,
        visual_column: usize,
        tab_width: usize,
    ) -> usize {
        if character == '\t' {
            Self::tab_advance(
                visual_column,
                tab_width,
            )
        } else {
            1
        }
    }

    fn visual_column_after_text(
        text: &str,
        start: usize,
        tab_width: usize,
    ) -> usize {
        text.chars().fold(
            start,
            |visual_column, character| {
                visual_column.saturating_add(
                    Self::visual_advance(
                        character,
                        visual_column,
                        tab_width,
                    ),
                )
            },
        )
    }

    // ----------------------------------------------------------------------
    // Visual column helpers
    // ----------------------------------------------------------------------

    fn visual_column_for_text(
        &self,
        text: &str,
        logical_column: usize,
    ) -> usize {
        let mut visual_column = 0usize;

        for (column, character) in text.chars().enumerate() {
            if column >= logical_column {
                break;
            }

            visual_column =
                visual_column.saturating_add(
                    Self::visual_advance(
                        character,
                        visual_column,
                        self.tab_width,
                    ),
                );
        }

        visual_column
    }

    fn x_for_visual_column(
        &self,
        visual_column: usize,
        text_left: i32,
    ) -> i32 {
        let visual_column =
            i32::try_from(visual_column)
                .unwrap_or(i32::MAX);

        text_left
            .saturating_add(
                visual_column.saturating_mul(
                    self.char_width.max(1),
                ),
            )
            .saturating_sub(self.scroll_x)
    }

    fn visual_x_for_column(
        &self,
        text: &str,
        logical_column: usize,
        text_left: i32,
    ) -> i32 {
        self.x_for_visual_column(
            self.visual_column_for_text(
                text,
                logical_column,
            ),
            text_left,
        )
    }

    fn logical_column_for_document_x(
        text: &str,
        document_x: i32,
        char_width: i32,
        tab_width: usize,
    ) -> usize {
        let target_x =
            i64::from(document_x.max(0));

        let char_width =
            i64::from(char_width.max(1));

        let mut logical_column = 0usize;
        let mut visual_column = 0usize;
        let mut start_x = 0i64;

        for character in text.chars() {
            let advance =
                Self::visual_advance(
                    character,
                    visual_column,
                    tab_width,
                );

            let advance_pixels =
                i64::try_from(advance)
                    .unwrap_or(i64::MAX)
                    .saturating_mul(char_width);

            let end_x =
                start_x.saturating_add(
                    advance_pixels,
                );

            if target_x < end_x {
                let midpoint =
                    start_x.saturating_add(
                        advance_pixels / 2
                            + advance_pixels % 2,
                    );

                return if target_x < midpoint {
                    logical_column
                } else {
                    logical_column.saturating_add(1)
                };
            }

            logical_column =
                logical_column.saturating_add(1);

            visual_column =
                visual_column.saturating_add(
                    advance,
                );

            start_x = end_x;
        }

        logical_column
    }

    fn visible_row_for_y(
        y: i32,
        text_top: i32,
        line_height: i32,
        visible_lines: usize,
    ) -> Option<usize> {
        if y < text_top
            || line_height <= 0
        {
            return None;
        }

        let row =
            usize::try_from(
                (y - text_top) / line_height,
            )
            .ok()?;

        (row < visible_lines)
            .then_some(row)
    }

    fn search_query_scroll_x(
        cursor_width: i32,
        query_width: i32,
        field_width: i32,
        padding: i32,
    ) -> i32 {
        let field_width = field_width.max(1);
        let padding =
            padding.max(0)
                .min(field_width);

        let desired =
            cursor_width
                .saturating_add(padding)
                .saturating_sub(field_width)
                .max(0);

        let maximum =
            query_width
                .saturating_add(padding)
                .saturating_sub(field_width)
                .max(0);

        desired.min(maximum)
    }

    fn search_bar_visibility(
        available_width: i32,
        minimum_query_width: i32,
        mode_width: i32,
        status_width: i32,
        gap: i32,
        has_status: bool,
    ) -> (bool, bool) {
        let available_width =
            available_width.max(0);

        let minimum_query_width =
            minimum_query_width.max(1);

        let gap = gap.max(0);

        let fits = |trailing_width: i32| {
            minimum_query_width
                .saturating_add(trailing_width)
                <= available_width
        };

        let mode_width =
            gap.saturating_add(mode_width.max(0));

        if !has_status {
            return (fits(mode_width), false);
        }

        let status_width =
            gap.saturating_add(status_width.max(0));

        if fits(
            status_width
                .saturating_add(mode_width),
        ) {
            (true, true)
        } else if fits(status_width) {
            // An error or failed-search indicator is more useful than
            // the mode label when only one of them can fit.
            (false, true)
        } else if fits(mode_width) {
            (true, false)
        } else {
            (false, false)
        }
    }

    // ----------------------------------------------------------------------
    // Syntax
    // ----------------------------------------------------------------------

   pub fn set_file_path(
    &mut self,
    path: Option<&std::path::Path>,
) {
    self.syntax = path
        .and_then(|path| path.extension())
        .and_then(|extension| extension.to_str())
        .and_then(|extension| {
            SyntaxDefinition::for_extension(extension)
        });
    self.invalidate_scroll_cache();
}
    // ----------------------------------------------------------------------
    // Cursor
    // ----------------------------------------------------------------------

    pub fn update_cursor(
        &mut self,
        table: &PieceTable,
    ) {
        self.cursor_line =
            table.cursor.line;

        self.cursor_column =
            table.cursor.column;
    }

    pub fn cursor_target_at(
        &self,
        table: &mut PieceTable,
        x: i32,
        y: i32,
    ) -> Result<Option<(usize, usize)>, String> {
        // Match the gutter that was used for the currently displayed frame.
        // Extending the lazy line cache below can increase its digit width.
        let text_left =
            self.text_left(table)?;

        let x = self.active_local_x(x);

        let layout =
            CursorHitTestLayout {
                text_left,
                text_top: TITLE_BAR_HEIGHT + 8,
                right: self.active_content_width(),
                bottom:
                    self.window_height
                        .saturating_sub(
                            self.bottom_inset
                        ),
                line_height:
                    self.font.height().max(1),
                visible_lines:
                    self.visible_line_count(),
                scroll_line: self.scroll_line,
                scroll_x: self.scroll_x,
                char_width: self.char_width,
                tab_width: self.tab_width,
            };

        Self::cursor_target_for_layout(
            table,
            layout,
            x,
            y,
        )
        .map_err(|error| error.to_string())
    }

    fn cursor_target_for_layout(
        table: &mut PieceTable,
        layout: CursorHitTestLayout,
        x: i32,
        y: i32,
    ) -> std::io::Result<
        Option<(usize, usize)>
    > {
        if x < layout.text_left
            || y < layout.text_top
            || x >= layout.right
            || y >= layout.bottom
        {
            return Ok(None);
        }

        let row =
            match Self::visible_row_for_y(
                y,
                layout.text_top,
                layout.line_height,
                layout.visible_lines,
            ) {
                Some(row) => row,
                None => return Ok(None),
            };

        let requested_line =
            layout
                .scroll_line
                .saturating_add(row);

        table
            .ensure_line_cached(requested_line)
            ?;

        let last_line =
            table
                .cached_line_count()
                .saturating_sub(1);

        let line =
            requested_line.min(last_line);

        let column =
            if requested_line > last_line {
                table.line_length(line)?
            } else {
                let document_x = x.saturating_sub(layout.text_left).saturating_add(layout.scroll_x).max(0);
                let cell = layout.char_width.max(1) as usize;
                let target = document_x as usize / cell;
                let window = table.line_window(line, target, target.saturating_add(1), layout.tab_width)?;
                let advance = window.text.chars().next()
                    .map(|c| Self::visual_advance(c, window.first_visual, layout.tab_width)).unwrap_or(0);
                let start = window.first_visual.saturating_mul(cell);
                let width = advance.saturating_mul(cell);
                window.first_column + usize::from(advance > 0 && document_x as usize >= start.saturating_add(width / 2 + width % 2))
            };

        Ok(Some((line, column)))
    }

    // ----------------------------------------------------------------------
    // Window
    // ----------------------------------------------------------------------

    pub fn update_window_size(
        &mut self,
    ) -> Result<(), String> {
        let (width, height) =
            logical_render_size(
                self.canvas.window(),
            );

        self.canvas
            .set_logical_size(
                width,
                height,
                SDL_LOGICAL_PRESENTATION_STRETCH,
            )
            .map_err(|e| e.to_string())?;

        let display_scale =
            self.canvas.window()
                .display_scale();

        if self.dpi_text
            .needs_update(display_scale)
        {
            let dpi_text =
                DpiTextMetrics::new(
                    display_scale,
                );

            self.raster_font
                .set_size(
                    dpi_text.raster_font_size(
                        self.logical_font_size,
                    )
                )
                .map_err(|e| e.to_string())?;

            self.command_raster_font
                .set_size(dpi_text.raster_font_size(COMMAND_FONT_SIZE))
                .map_err(|e| e.to_string())?;
            self.terminal_text_cache.clear();
            self.dpi_text = dpi_text;
        }

        self.window_width =
            width as i32;

        self.window_height =
            height as i32;

        self.window_hit_test
            .set_metrics(
                width as i32,
                window_coordinate_scale(
                    self.canvas.window(),
                ),
            );

        Ok(())
    }

    fn render_dpi_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        color: Color,
    ) -> Result<f32, String> {
        if text.is_empty() {
            return Ok(0.0);
        }

        // Keep layout tied to the logical font so syntax spans, selection,
        // and the caret do not move when the window changes displays. Only
        // the texture's raster resolution changes with DPI.
        let (width, height) =
            self.logical_text_size(text);

        self.render_dpi_text_with_size(
            text,
            x,
            y,
            color,
            width,
            height,
        )?;

        Ok(width)
    }

    fn logical_text_size(
        &self,
        text: &str,
    ) -> (f32, f32) {
        self.font
            .size_of(text)
            .map(|(width, height)| {
                (
                    width as f32,
                    height as f32,
                )
            })
            .unwrap_or_else(|_| {
                (
                    text.chars().count() as f32
                        * self.char_width.max(1) as f32,
                    self.font.height().max(1) as f32,
                )
            })
    }

    fn render_dpi_text_with_size(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        color: Color,
        width: f32,
        height: f32,
    ) -> Result<(), String> {
        // A BOM or another zero-width Unicode run can be isolated by syntax
        // highlighting. SDL_ttf rejects rasterizing it, although it is valid
        // document text. Keep its bytes; there are simply no pixels to draw.
        if width <= 0.0 || height <= 0.0 {
            return Ok(());
        }
        let key = if self.cache_terminal_text { TextCache::key(text, color) } else { 0 };
        if self.cache_terminal_text {
            if let Some(texture) = self.terminal_text_cache.get(key, text, color) {
                return self.canvas.copy(texture, None, FRect::new(x, y, width, height))
                    .map_err(|error| error.to_string());
            }
            #[cfg(test)] { self.terminal_text_cache.misses += 1; }
        }
        let surface = self.raster_font.render(text).blended(color).map_err(|e| e.to_string())?;
        let texture = self.texture_creator.create_texture_from_surface(&surface).map_err(|e| e.to_string())?;
        self.canvas.copy(&texture, None, FRect::new(x, y, width, height)).map_err(|e| e.to_string())?;
        if self.cache_terminal_text {
            self.terminal_text_cache.insert(key, text, color, texture);
        }
        Ok(())
    }

    fn render_clipped_single_line_text(
        &mut self,
        text: &str,
        text_origin_x: i32,
        y: i32,
        viewport: Rect,
        logical_width: i32,
        color: Color,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }

        let safe_texture_width =
            MAX_TEXT_TEXTURE_WIDTH
                .saturating_sub(
                    TEXT_TEXTURE_SAFETY_MARGIN,
                );

        let physical_width =
            logical_width.max(0) as f32
                * self.dpi_text
                    .raster_scale();

        if physical_width
            <= safe_texture_width as f32
        {
            self.render_dpi_text_with_size(
                text,
                text_origin_x as f32,
                y as f32,
                color,
                logical_width.max(0) as f32,
                self.font.height().max(1) as f32,
            )?;

            return Ok(());
        }

        let clip_left =
            viewport.x();

        let clip_right =
            clip_left
                .saturating_add(
                    i32::try_from(
                        viewport.width(),
                    )
                    .unwrap_or(i32::MAX)
                    .max(1),
                );

        self.render_plain_text(
            text,
            text_origin_x,
            y,
            color,
            MAX_TEXT_TEXTURE_WIDTH,
            Some((
                clip_left as f32,
                clip_right as f32,
            )),
        )
    }

    pub fn convert_event_coordinates(
        &self,
        event: &mut Event,
    ) -> bool {
        event.convert_coords(
            &self.canvas,
        )
    }

    pub fn window_mut(
        &mut self,
    ) -> &mut Window {
        self.canvas.window_mut()
    }

    // ----------------------------------------------------------------------
    // Custom title bar
    // ----------------------------------------------------------------------

    fn render_title_bar(
        &mut self,
    ) -> Result<(), String> {
        // Window chrome uses fixed UI fonts, independent of editor zoom.
        std::mem::swap(&mut self.font, &mut self.command_font);
        std::mem::swap(&mut self.raster_font, &mut self.command_raster_font);
        std::mem::swap(&mut self.char_width, &mut self.command_char_width);
        let result = self.render_title_bar_contents();
        std::mem::swap(&mut self.font, &mut self.command_font);
        std::mem::swap(&mut self.raster_font, &mut self.command_raster_font);
        std::mem::swap(&mut self.char_width, &mut self.command_char_width);
        self.canvas.set_clip_rect(None);
        result
    }

    fn render_title_bar_contents(&mut self) -> Result<(), String> {
        let height =
            TITLE_BAR_HEIGHT;

        let width =
            self.window_width.max(1);

        self.canvas.set_draw_color(
            Color::RGB(38, 38, 38),
        );

        self.canvas
            .fill_rect(Rect::new(
                0,
                0,
                width as u32,
                height as u32,
            ))
            .map_err(|e| e.to_string())?;

        self.canvas.set_draw_color(
            Color::RGB(65, 65, 65),
        );

        self.canvas
            .fill_rect(Rect::new(
                0,
                height - 1,
                width as u32,
                1,
            ))
            .map_err(|e| e.to_string())?;

        let title = "Pötyi";

        self.canvas.set_clip_rect(Rect::new(0, 0,
            (width - WINDOW_BUTTONS_WIDTH).max(1) as u32, height as u32));

        let title_y =
            (
                height
                    - self.font.height()
            ) / 2;

        let title_width = self.render_dpi_text(
            title,
            14.0,
            title_y as f32,
            Color::RGB(220, 220, 220),
        )?;

        // Reuse the fixed UI font at a smaller destination size: no extra font
        // or glyph-cache resets when drawing this secondary label.
        let version = env!("POTYI_DISPLAY_VERSION");
        let (version_width, version_height) = self.logical_text_size(version);
        let version_width = version_width * 0.5;
        let version_height = version_height * 0.5;
        let pill_x = (14.0 + title_width + 6.0).round();
        let pill_height = 10.0;
        let pill_y = (title_y + self.font.height()) as f32 - pill_height;
        let pill_width = version_width.ceil() + 10.0;
        let pill_right = self.mode_label.map_or(width - WINDOW_BUTTONS_WIDTH - 12, |label| {
            let label_width = self.logical_text_size(label).0.ceil() as i32;
            (width - WINDOW_BUTTONS_WIDTH - label_width - 16).max(80) - 12
        });
        // Hide the secondary badge first in narrow windows, preserving controls.
        if pill_x + pill_width <= pill_right as f32 {
            // Rounded capsule, batched in one draw call with no heap allocation.
            const INSETS: [f32; 10] = [3., 1., 1., 0., 0., 0., 0., 1., 1., 3.];
            let rows: [FRect; 10] = std::array::from_fn(|row| {
                let inset = INSETS[row];
                FRect::new(pill_x + inset, pill_y + row as f32, pill_width - inset * 2.0, 1.0)
            });
            self.canvas.set_draw_color(Color::RGB(49, 49, 49));
            self.canvas.fill_rects(&rows).map_err(|e| e.to_string())?;
            self.render_dpi_text_with_size(
                version, pill_x + 5.0, pill_y + (pill_height - version_height) / 2.0,
                Color::RGB(155, 155, 155), version_width, version_height,
            )?;
        }

        if let Some(label) = self.mode_label {
            let label_width = self.font.size_of(label)
                .map(|(width, _)| width as i32)
                .unwrap_or_else(|_| {
                    label.chars().count() as i32 * self.char_width
                });
            let label_x = (width
                - WINDOW_BUTTONS_WIDTH
                - label_width
                - 16)
                .max(80);

            self.render_dpi_text(
                label,
                label_x as f32,
                title_y as f32,
                Color::RGB(120, 185, 255),
            )?;
        }

        self.canvas.set_clip_rect(None);

        let buttons_left =
            width
                - WINDOW_BUTTONS_WIDTH;

        let center_y =
            height / 2;

        let hovered_button = match self.hovered_window_control {
            WindowControl::Minimize => Some((0, Color::RGB(62, 62, 62))),
            WindowControl::Maximize => Some((1, Color::RGB(62, 62, 62))),
            WindowControl::Close => Some((2, Color::RGB(196, 43, 50))),
            WindowControl::None => None,
        };
        if let Some((index, color)) = hovered_button {
            self.canvas.set_draw_color(color);
            self.canvas.fill_rect(Rect::new(
                buttons_left + index * WINDOW_BUTTON_WIDTH, 0,
                WINDOW_BUTTON_WIDTH as u32, (height - 1) as u32,
            )).map_err(|e| e.to_string())?;
        }

        self.canvas.set_draw_color(
            Color::RGB(
                210,
                210,
                210,
            ),
        );

        // Minimize
        let minimize_center =
            buttons_left
                + WINDOW_BUTTON_WIDTH / 2;

        self.canvas
            .draw_line(
                Point::new(
                    minimize_center - 6,
                    center_y,
                ),
                Point::new(
                    minimize_center + 6,
                    center_y,
                ),
            )
            .map_err(|e| e.to_string())?;

        // Maximize
        let maximize_center =
            buttons_left
                + WINDOW_BUTTON_WIDTH
                + WINDOW_BUTTON_WIDTH / 2;

        self.canvas
            .draw_rect(Rect::new(
                maximize_center - 6,
                center_y - 6,
                12,
                12,
            ))
            .map_err(|e| e.to_string())?;

        // Close
        if self.hovered_window_control == WindowControl::Close {
            self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        }
        let close_center =
            buttons_left
                + WINDOW_BUTTON_WIDTH * 2
                + WINDOW_BUTTON_WIDTH / 2;

        self.canvas
            .draw_line(
                Point::new(
                    close_center - 6,
                    center_y - 6,
                ),
                Point::new(
                    close_center + 6,
                    center_y + 6,
                ),
            )
            .map_err(|e| e.to_string())?;

        self.canvas
            .draw_line(
                Point::new(
                    close_center + 6,
                    center_y - 6,
                ),
                Point::new(
                    close_center - 6,
                    center_y + 6,
                ),
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    // ----------------------------------------------------------------------
    // Window control hit testing
    // ----------------------------------------------------------------------

    pub(crate) fn set_window_control_hover(&mut self, control: WindowControl) -> bool {
        if self.hovered_window_control == control { return false; }
        self.hovered_window_control = control;
        true
    }

    pub fn window_control_at(
        &self,
        x: i32,
        y: i32,
    ) -> WindowControl {
        if y < 0
            || y >= TITLE_BAR_HEIGHT
        {
            return WindowControl::None;
        }

        let buttons_left =
            self.window_width
                - WINDOW_BUTTONS_WIDTH;

        if x < 0 || x < buttons_left {
            return WindowControl::None;
        }

        let relative =
            x - buttons_left;

        if relative < WINDOW_BUTTON_WIDTH {
            WindowControl::Minimize
        } else if relative
            < WINDOW_BUTTON_WIDTH * 2
        {
            WindowControl::Maximize
        } else if relative
            < WINDOW_BUTTON_WIDTH * 3
        {
            WindowControl::Close
        } else {
            WindowControl::None
        }
    }

    // ----------------------------------------------------------------------
    // Scrolling
    // ----------------------------------------------------------------------

    pub fn invalidate_scroll_cache(
        &mut self,
    ) {
        self.scroll_start_valid = false;
    }

    fn set_scroll_line(
        &mut self,
        table: &mut PieceTable,
        target: usize,
    ) {
        if self.scroll_revision != table.revision() {
            self.scroll_start_valid = false;
            self.scroll_revision = table.revision();
        }
        if !self.scroll_start_valid {
            let byte =
                table
                    .line_start(target)
                    .unwrap_or(table.len());

            self.scroll_start_byte = byte;
            self.scroll_start_line = target;
            self.scroll_line = target;
            self.scroll_start_valid = true;

            return;
        }

        if target == self.scroll_start_line {
            self.scroll_line = target;
            return;
        }

        if target > self.scroll_start_line {
            let mut line =
                self.scroll_start_line;

            let mut byte =
                self.scroll_start_byte;

            while line < target {
                match table
                    .next_line_start_from(byte)
                {
                    Ok(Some(next)) => {
                        byte = next;
                        line += 1;
                    }

                    _ => break,
                }
            }

            self.scroll_start_line = line;
            self.scroll_start_byte = byte;
            self.scroll_line = line;
        } else {
            let mut line =
                self.scroll_start_line;

            let mut byte =
                self.scroll_start_byte;

            while line > target {
                byte =
                    table
                        .previous_line_start_from(byte)
                        .unwrap_or(0);

                line -= 1;
            }

            self.scroll_start_line = line;
            self.scroll_start_byte = byte;
            self.scroll_line = line;
        }

        self.scroll_start_valid = true;
    }

    pub fn scroll_horizontal(
        &mut self,
        amount: i32,
    ) {
        self.scroll_x =
            self.scroll_x
                .saturating_add(amount)
                .max(0);
    }

    pub fn scroll_by(
        &mut self,
        amount: isize,
        table: &mut PieceTable,
    ) {
        let current =
            self.scroll_line as isize;

        let target =
            current
                .saturating_add(amount)
                .max(0) as usize;

        self.set_scroll_line(
            table,
            target,
        );
    }

    pub fn ensure_cursor_visible(
        &mut self,
        table: &mut PieceTable,
    ) {
        self.update_cursor(table);

        let visible =
            self.visible_line_count();

        if visible == 0 {
            return;
        }

        if self.cursor_line < self.scroll_line {
            self.set_scroll_line(
                table,
                self.cursor_line,
            );
        } else {
            let last =
                self.scroll_line
                    + visible
                    - 1;

            if self.cursor_line > last {
                self.set_scroll_line(
                    table,
                    self.cursor_line
                        .saturating_sub(
                            visible - 1,
                        ),
                );
            }
        }

        let text_left =
            self.text_left(table)
                .unwrap_or(20);

        let visual = match table.line_visual_column(self.cursor_line, self.cursor_column, self.tab_width) {
            Ok(visual) => visual,
            Err(_) => return,
        };
        let cursor_x = self.x_for_visual_column(visual, text_left);

        let right =
            self.active_content_width() - 20;

        if cursor_x > right {
            self.scroll_x =
                self.scroll_x
                    .saturating_add(
                        cursor_x - right,
                    );
        }

        if cursor_x < text_left {
            self.scroll_x =
                self.scroll_x
                    .saturating_sub(
                        text_left - cursor_x,
                    )
                    .max(0);
        }
    }

    pub fn ensure_search_match_visible(
        &mut self,
        table: &mut PieceTable,
        search_ui: &SearchUi,
        command_bar: &CommandBar,
    ) {
        let current =
            match search_ui.current_match() {
                Some(current) => current,
                None => return,
            };

        let (line, column) =
            match table.line_column_at(
                current.start,
            ) {
                Ok(value) => value,
                Err(_) => return,
            };

        let visible =
            self.visible_line_count_with_inset(
                command_bar.reserved_height()
            );

        if visible == 0 {
            return;
        }

        if line < self.scroll_line {
            self.set_scroll_line(
                table,
                line,
            );
        } else {
            let last =
                self.scroll_line
                    + visible
                    - 1;

            if line > last {
                self.set_scroll_line(
                    table,
                    line.saturating_sub(
                        visible - 1,
                    ),
                );
            }
        }

        let text_left =
            self.text_left(table)
                .unwrap_or(20);

        let visual = match table.line_visual_column(line, column, self.tab_width) {
            Ok(visual) => visual,
            Err(_) => return,
        };
        let x = self.x_for_visual_column(visual, text_left);

        let right =
            self.active_content_width() - 20;

        if x > right {
            self.scroll_x =
                self.scroll_x
                    .saturating_add(
                        x - right,
                    );
        }

        if x < text_left {
            self.scroll_x =
                self.scroll_x
                    .saturating_sub(
                        text_left - x,
                    )
                    .max(0);
        }
    }

    pub fn visible_line_count(
        &self,
    ) -> usize {
        self.visible_line_count_with_inset(
            self.bottom_inset
        )
    }

    fn terminal_line_height(&self) -> i32 {
        let font_height = self.font.height().max(1);
        // Round up so terminal rows remain at least 1.4× the glyph height.
        (font_height * TERMINAL_LINE_HEIGHT_SCALE_NUMERATOR
            + TERMINAL_LINE_HEIGHT_SCALE_DENOMINATOR - 1)
            / TERMINAL_LINE_HEIGHT_SCALE_DENOMINATOR
    }

    fn visible_terminal_line_count(&self) -> usize {
        let top_margin = TITLE_BAR_HEIGHT + 8;
        let content_bottom = self.terminal_height().saturating_sub(self.terminal_bottom_inset.max(0));
        if content_bottom <= top_margin {
            return 1;
        }
        ((content_bottom - top_margin) / self.terminal_line_height()).max(1) as usize
    }

    fn visible_line_count_with_inset(
        &self,
        bottom_inset: i32,
    ) -> usize {
        let line_height =
            self.font.height().max(1);

        let top_margin =
            TITLE_BAR_HEIGHT + 8;

        let content_bottom =
            self.window_height
                .saturating_sub(
                    bottom_inset.max(0)
                );

        if content_bottom <= top_margin {
            return 1;
        }

        (
            (content_bottom - top_margin)
                / line_height
        )
        .max(1) as usize
    }

    fn search_bar_height(
        search_ui: &SearchUi,
    ) -> i32 {
        if search_ui.is_replace_mode() {
            SEARCH_ROW_HEIGHT * 2
        } else {
            SEARCH_ROW_HEIGHT
        }
    }

    fn search_bar_reserved_height(
        search_ui: &SearchUi,
    ) -> i32 {
        if search_ui.is_active() {
            Self::search_bar_height(search_ui)
                .saturating_add(
                    SEARCH_BAR_MARGIN
                )
        } else {
            0
        }
    }

    // ----------------------------------------------------------------------
    // Rendering
    // ----------------------------------------------------------------------

    pub fn render(
        &mut self,
        table: &mut PieceTable,
        search_ui: &SearchUi,
        command_bar: &CommandBar,
    ) -> Result<(), String> {
        self.terminal_text_cache.clear();
        self.bottom_inset =
            command_bar.reserved_height();

        self.canvas.set_draw_color(
            Color::RGB(
                30,
                30,
                30,
            ),
        );

        self.canvas.clear();

        self.render_title_bar()?;

        self.render_document_body(
            table,
            Some(search_ui),
            true,
        )?;

        self.render_command_bar(
            command_bar,
            search_ui,
        )?;

        self.render_completion(table)?;
        self.canvas.present();

        Ok(())
    }

    pub fn render_split(
        &mut self,
        active_table: &mut PieceTable,
        inactive_table: &mut PieceTable,
        search_ui: &SearchUi,
        command_bar: &CommandBar,
    ) -> Result<(), String> {
        self.terminal_text_cache.clear();
        self.bottom_inset =
            command_bar.reserved_height();

        self.canvas.set_draw_color(
            Color::RGB(30, 30, 30),
        );
        self.canvas.clear();
        self.render_title_bar()?;

        let active_bounds =
            self.pane_bounds(self.active_pane);
        let inactive_pane =
            1usize.saturating_sub(self.active_pane);
        let inactive_bounds =
            self.pane_bounds(inactive_pane);

        self.render_document_pane(
            active_table,
            Some(search_ui),
            true,
            active_bounds,
        )?;

        self.swap_view();
        let inactive_result =
            self.render_document_pane(
                inactive_table,
                None,
                false,
                inactive_bounds,
            );
        self.swap_view();
        inactive_result?;

        self.canvas.set_draw_color(
            Color::RGB(74, 74, 74),
        );
        let divider = self.split_divider();
        self.canvas.fill_rect(Rect::new(
            divider,
            TITLE_BAR_HEIGHT,
            1,
            self.window_height
                .saturating_sub(TITLE_BAR_HEIGHT)
                .max(1) as u32,
        )).map_err(|error| error.to_string())?;

        let (active_left, active_width) =
            self.pane_bounds(self.active_pane);
        self.canvas.set_draw_color(
            Color::RGB(85, 145, 220),
        );
        self.canvas.fill_rect(Rect::new(
            active_left,
            TITLE_BAR_HEIGHT,
            active_width.max(1) as u32,
            2,
        )).map_err(|error| error.to_string())?;

        self.render_command_bar(
            command_bar,
            search_ui,
        )?;

        self.render_completion(active_table)?;
        self.canvas.present();

        Ok(())
    }

    /// Draw one terminal over a split pane. The terminal keeps its own layout
    /// and history while the other pane remains available for editing.
    pub fn render_split_terminal(
        &mut self,
        active_table: &mut PieceTable,
        inactive_table: &mut PieceTable,
        terminal: &mut Terminal,
        search_ui: &SearchUi,
        command_bar: &CommandBar,
    ) -> Result<(), String> {
        let terminal_pane = self.terminal_pane;
        self.bottom_inset = command_bar.reserved_height();
        self.terminal_reserved_height = self.bottom_inset;
        self.canvas.set_draw_color(Color::RGB(30, 30, 30));
        self.canvas.clear();
        self.render_title_bar()?;

        let active_bounds = self.pane_bounds(self.active_pane);
        let inactive_pane = 1usize.saturating_sub(self.active_pane);
        let inactive_bounds = self.pane_bounds(inactive_pane);
        if terminal_pane != self.active_pane {
            self.render_document_pane(active_table, Some(search_ui), true, active_bounds)?;
        } else {
            self.swap_view();
            let inactive_result = self.render_document_pane(inactive_table, None, false, inactive_bounds);
            self.swap_view();
            inactive_result?;
        }
        self.render_terminal_pane(terminal, self.pane_bounds(terminal_pane))?;

        self.canvas.set_draw_color(Color::RGB(74, 74, 74));
        let divider = self.split_divider();
        self.canvas.fill_rect(Rect::new(divider, TITLE_BAR_HEIGHT, 1,
            self.window_height.saturating_sub(TITLE_BAR_HEIGHT).max(1) as u32,
        )).map_err(|error| error.to_string())?;
        let (active_left, active_width) = self.pane_bounds(self.active_pane);
        self.canvas.set_draw_color(Color::RGB(85, 145, 220));
        self.canvas.fill_rect(Rect::new(active_left, TITLE_BAR_HEIGHT, active_width.max(1) as u32, 2))
            .map_err(|error| error.to_string())?;
        self.render_command_bar(command_bar, search_ui)?;
        if terminal_pane != self.active_pane {
            self.render_completion(active_table)?;
        }
        self.canvas.present();
        Ok(())
    }

    fn render_terminal_pane(&mut self, terminal: &mut Terminal, (left, width): (i32, i32)) -> Result<(), String> {
        let full_width = self.window_width;
        let full_height = self.window_height;
        let editor_inset = self.bottom_inset;
        self.window_height = self.terminal_height();
        self.canvas.set_viewport(Rect::new(left, 0, width.max(1) as u32, self.window_height.max(1) as u32));
        self.window_width = width.max(1);
        self.rendering_terminal_pane = true;
        self.cache_terminal_text = true;
        let result = self.render_terminal_contents(terminal, false);
        self.cache_terminal_text = false;
        self.canvas.set_clip_rect(None);
        self.canvas.set_viewport(None);
        self.window_width = full_width;
        self.window_height = full_height;
        self.bottom_inset = editor_inset;
        self.rendering_terminal_pane = false;
        result
    }

    fn render_document_pane(
        &mut self,
        table: &mut PieceTable,
        search_ui: Option<&SearchUi>,
        show_cursor: bool,
        (left, width): (i32, i32),
    ) -> Result<(), String> {
        let full_width = self.window_width;

        self.canvas.set_viewport(Rect::new(
            left,
            0,
            width.max(1) as u32,
            self.window_height.max(1) as u32,
        ));
        self.window_width = width.max(1);
        self.rendering_document_pane = true;

        let result = self.render_document_body(
            table,
            search_ui,
            show_cursor,
        );

        self.canvas.set_clip_rect(None);
        self.canvas.set_viewport(None);
        self.window_width = full_width;
        self.rendering_document_pane = false;

        result
    }

    fn render_document_body(
        &mut self,
        table: &mut PieceTable,
        search_ui: Option<&SearchUi>,
        show_cursor: bool,
    ) -> Result<(), String> {
        self.update_cursor(table);

        let visible =
            self.visible_line_count();

        if visible > 0 {
            table
                .ensure_line_cached(
                    self.scroll_line
                        + visible
                        - 1,
                )
                .map_err(|e| e.to_string())?;
        }

        let known_lines =
            table.cached_line_count();

        LineNumbers::render(
            &mut self.canvas,
            &self.font,
            &self.raster_font,
            self.scroll_line,
            visible,
            known_lines,
            self.font.height(),
            self.line_number_mode,
            self.cursor_line,
        )?;

        let clip =
            self.text_clip_rect(table)?;

        self.canvas.set_clip_rect(
            Some(clip),
        );

        self.render_selection(table)?;

        if let Some(search_ui) = search_ui {
            self.render_search_match(
                table,
                search_ui,
            )?;
        }

        self.render_text(table)?;

        if show_cursor {
            self.render_cursor(table)?;
        }

        self.canvas.set_clip_rect(None);

        Ok(())
    }

    pub fn render_terminal(
        &mut self,
        terminal: &mut Terminal,
    ) -> Result<(), String> {
        self.terminal_reserved_height = 0;
        self.cache_terminal_text = true;
        let result = self.render_terminal_contents(terminal, true);
        self.cache_terminal_text = false;
        result
    }

    fn render_terminal_contents(&mut self, terminal: &mut Terminal, full_view: bool) -> Result<(), String> {
        let mut status_lines = terminal.status()
            .map(|status| self.terminal_status_lines(status))
            .unwrap_or_default();
        status_lines.extend(self.terminal_status_lines(terminal.output_focus_hint()
            .unwrap_or("Command input · Shift+Up: select output · Esc: editor")));
        if terminal.can_go_back() { status_lines.extend(self.terminal_status_lines("Commit diff · Back / Alt+Left: return to log")); }
        let status_height = if status_lines.is_empty() { 0 } else {
            status_lines.len() as i32 * self.font.height().max(1) + 8
        };
        self.bottom_inset =
            TERMINAL_BAR_HEIGHT
                + TERMINAL_BAR_MARGIN
                + status_height;
        self.terminal_bottom_inset = self.bottom_inset;

        self.canvas.set_draw_color(Color::RGB(30, 30, 30));
        if full_view {
            self.canvas.clear();
            self.render_title_bar()?;
        } else {
            self.canvas.fill_rect(Rect::new(0, TITLE_BAR_HEIGHT, self.window_width.max(1) as u32,
                self.window_height.saturating_sub(TITLE_BAR_HEIGHT).max(1) as u32,
            )).map_err(|error| error.to_string())?;
        }

        self.update_terminal_layout(terminal)?;
        let visible = self.visible_terminal_line_count();
        let rows = self.terminal_layout.visible_rows(visible, terminal.scroll_back());

        let clip = Rect::new(
            10,
            TITLE_BAR_HEIGHT,
            self.window_width
                .saturating_sub(20)
                .max(1) as u32,
            self.window_height
                .saturating_sub(
                    TITLE_BAR_HEIGHT
                        + self.bottom_inset
                )
                .max(1) as u32,
        );

        self.canvas.set_clip_rect(
            Some(clip)
        );

        let line_height = self.terminal_line_height();

        let selection = terminal.output_selection();
        let cursor = terminal.output_cursor();
        let cursor_row = self.terminal_layout.row_at(cursor);
        for row in rows.clone() {
            let range = self.terminal_layout.row_range(row, terminal.output_mut())
                .map_err(|error| error.to_string())?.expect("visible row must exist");
            let start = range.start;
            let text = String::from_utf8(terminal.output_mut().read_range(start, range.len())
                .map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
            let y = TITLE_BAR_HEIGHT + 8 + (row - rows.start) as i32 * line_height;
            if !selection.is_empty() && selection.start < range.end.saturating_add(1) && selection.end > start {
                let from = selection.start.saturating_sub(start).min(text.len());
                let to = selection.end.saturating_sub(start).min(text.len());
                let left = self.terminal_text_width(&text[..from]);
                let right = self.terminal_text_width(&text[..to]);
                let newline_width = if selection.end > range.end { self.char_width.max(1) } else { 0 };
                self.canvas.set_draw_color(Color::RGB(55, 78, 110));
                self.canvas.fill_rect(Rect::new(12 + left, y,
                    (right - left + newline_width).max(1) as u32, line_height as u32,
                )).map_err(|error| error.to_string())?;
            }
            if terminal.output_focused() && row == cursor_row
                && (!self.split_mode || self.terminal_pane == self.active_pane) {
                let column = cursor.saturating_sub(start).min(text.len());
                let x = 12 + self.terminal_text_width(&text[..column]);
                self.canvas.set_draw_color(Color::RGB(245, 245, 245));
                self.canvas.fill_rect(Rect::new(x, y, 2, line_height as u32))
                    .map_err(|error| error.to_string())?;
            }
            let semantic_color = terminal.output_color(start).map(|(r,g,b)| Color::RGB(r,g,b));
            let base_color = semantic_color.unwrap_or(Color::RGB(215, 220, 215));
            let mut rendered_to = 0;
            for entry in terminal.entries_in(start..start + text.len()) {
                let from = entry.range.start.saturating_sub(start);
                let to = (entry.range.end - start).min(text.len());
                self.render_terminal_text_chunk(
                    &text[rendered_to..from],
                    12 + self.terminal_text_width(&text[..rendered_to]),
                    Self::visual_column_after_text(&text[..rendered_to], 0, self.tab_width),
                    y, base_color,
                )?;
                let left = self.terminal_text_width(&text[..from]);
                let right = self.terminal_text_width(&text[..to]);
                let color = semantic_color.or_else(|| entry.git_status.map(|status| {
                    let (r, g, b) = status.color();
                    Color::RGB(r, g, b)
                })).unwrap_or(match entry.kind {
                    EntryKind::Commit => Color::RGB(225, 195, 120),
                    _ => base_color,
                });
                self.render_terminal_text_chunk(
                    &text[from..to], 12 + left,
                    Self::visual_column_after_text(&text[..from], 0, self.tab_width), y, color,
                )?;
                rendered_to = to;
                if matches!(entry.kind, EntryKind::Text | EntryKind::Directory | EntryKind::Commit) {
                    self.canvas.set_draw_color(color);
                    self.canvas.fill_rect(Rect::new(
                        12 + left, y + self.font.height().max(1) - 2,
                        (right - left).max(1) as u32, 1,
                    )).map_err(|error| error.to_string())?;
                }
            }
            self.render_terminal_text_chunk(
                &text[rendered_to..],
                12 + self.terminal_text_width(&text[..rendered_to]),
                Self::visual_column_after_text(&text[..rendered_to], 0, self.tab_width),
                y, base_color,
            )?;
        }

        self.canvas.set_clip_rect(None);
        self.render_terminal_bar(terminal, &status_lines)?;
        if full_view {
            self.canvas.present();
        }
        Ok(())
    }

    pub(crate) fn terminal_hit_at(
        &self,
        x: i32,
        y: i32,
    ) -> TerminalHit {
        let (left, width) = self.terminal_bounds();
        let x = x - left;
        if x < 0 || x >= width {
            return TerminalHit::Outside;
        }
        if y < TITLE_BAR_HEIGHT
            || y >= self.terminal_height()
                - TERMINAL_BAR_MARGIN
        {
            return TerminalHit::Outside;
        }

        let bar_top = self.terminal_height()
            - TERMINAL_BAR_MARGIN
            - TERMINAL_BAR_HEIGHT;

        if y < bar_top {
            if y >= self.terminal_height() - self.terminal_bottom_inset {
                return TerminalHit::Outside;
            }
            return TerminalHit::Output;
        }

        let editor_left = self.terminal_bounds().1
            - TERMINAL_BAR_MARGIN
            - TERMINAL_EDITOR_WIDTH;

        let clear_left = editor_left
            - TERMINAL_CLEAR_WIDTH;

        let action_left = clear_left
            - TERMINAL_ACTION_WIDTH;

        if x >= editor_left {
            TerminalHit::Editor
        } else if x >= clear_left {
            TerminalHit::Clear
        } else if x >= action_left {
            TerminalHit::StopOrRunAgain
        } else if x >= TERMINAL_BAR_MARGIN {
            TerminalHit::Input
        } else {
            TerminalHit::Outside
        }
    }

    pub(crate) fn terminal_cursor_at(
        &self,
        terminal: &Terminal,
        x: i32,
    ) -> usize {
        let x = x - self.terminal_bounds().0;
        let text_x = TERMINAL_BAR_MARGIN + 10;

        if x <= text_x {
            return 0;
        }

        let input = terminal.input();
        let editor_left = self.terminal_bounds().1
            - TERMINAL_BAR_MARGIN
            - TERMINAL_EDITOR_WIDTH;
        let action_left = editor_left
            - TERMINAL_CLEAR_WIDTH
            - TERMINAL_ACTION_WIDTH;
        let field_width = action_left
            .saturating_sub(8)
            .saturating_sub(text_x)
            .max(1);
        let input_width = self.font
            .size_of(input)
            .map(|(width, _)| width as i32)
            .unwrap_or_else(|_| {
                input.chars().count() as i32
                    * self.char_width
            });
        let prefix = input.get(
            ..terminal.cursor().min(input.len())
        ).unwrap_or(input);
        let cursor_width = self.font
            .size_of(prefix)
            .map(|(width, _)| width as i32)
            .unwrap_or_else(|_| {
                prefix.chars().count() as i32
                    * self.char_width
            });
        let scroll_x =
            Self::search_query_scroll_x(
                cursor_width,
                input_width,
                field_width,
                4,
            );
        let target = x - text_x + scroll_x;
        let mut previous = 0i32;

        for (index, character) in
            input.char_indices()
        {
            let end = index
                + character.len_utf8();

            let width = self.font
                .size_of(&input[..end])
                .map(|(width, _)| {
                    width as i32
                })
                .unwrap_or_else(|_| {
                    input[..end]
                        .chars()
                        .count() as i32
                        * self.char_width
                });

            if target < previous
                + (width - previous) / 2
            {
                return index;
            }

            previous = width;
        }

        input.len()
    }

    pub(crate) fn terminal_action_at(
        &mut self,
        terminal: &mut Terminal,
        x: i32,
        y: i32,
    ) -> Result<Option<TerminalAction>, String> {
        if self.terminal_hit_at(x, y)
            != TerminalHit::Output
        {
            return Ok(None);
        }

        self.update_terminal_layout(terminal)?;
        let rows = self.terminal_layout.visible_rows(self.visible_terminal_line_count(), terminal.scroll_back());
        let row_y = y - TITLE_BAR_HEIGHT - 8;
        let target_x = x - self.terminal_bounds().0 - 12;
        if row_y < 0 || target_x < 0 {
            return Ok(None);
        }
        let row = rows.start + row_y as usize / self.terminal_line_height() as usize;
        if row >= rows.end {
            return Ok(None);
        }
        let range = self.terminal_layout.row_range(row, terminal.output_mut())
            .map_err(|error| error.to_string())?.expect("visible row must exist");
        let text = String::from_utf8(terminal.output_mut().read_range(range.start, range.len())
            .map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
        for (index, character) in text.char_indices() {
            let end = index + character.len_utf8();
            if target_x < self.terminal_text_width(&text[..end]) {
                return terminal.action_at_output_offset(range.start + index)
                    .map_err(|error| error.to_string());
            }
        }
        Ok(None)
    }

    /// Clamp pointer selection to visible output, including empty lines and row ends.
    pub(crate) fn terminal_output_offset_at(&mut self, terminal: &mut Terminal, x: i32, y: i32) -> Result<usize, String> {
        self.update_terminal_layout(terminal)?;
        let rows = self.terminal_layout.visible_rows(self.visible_terminal_line_count(), terminal.scroll_back());
        let row = (rows.start + (y - TITLE_BAR_HEIGHT - 8).max(0) as usize / self.terminal_line_height() as usize)
            .min(rows.end.saturating_sub(1));
        self.terminal_offset_on_row(terminal, row, x - self.terminal_bounds().0 - 12)
    }

    fn terminal_offset_on_row(&self, terminal: &mut Terminal, row: usize, x: i32) -> Result<usize, String> {
        let Some(range) = self.terminal_layout.row_range(row, terminal.output_mut()).map_err(|e| e.to_string())? else {
            return Ok(0);
        };
        let bytes = terminal.output_mut().read_range(range.start, range.len()).map_err(|e| e.to_string())?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let mut previous = 0;
        for (index, character) in text.char_indices() {
            let end = index + character.len_utf8();
            let width = self.terminal_text_width(&text[..end]);
            if x < previous + (width - previous) / 2 {
                return Ok(range.start + index);
            }
            previous = width;
        }
        Ok(range.end)
    }

    pub(crate) fn navigate_terminal_output(&mut self, terminal: &mut Terminal, command: OutputCommand) -> Result<(), String> {
        self.update_terminal_layout(terminal)?;
        let row = self.terminal_layout.row_at(terminal.output_cursor());
        match command {
            OutputCommand::Rows(delta, extend) => {
                let range = self.terminal_layout.row_range(row, terminal.output_mut()).map_err(|e| e.to_string())?.unwrap_or(0..0);
                let length = terminal.output_cursor().min(range.end).saturating_sub(range.start);
                let prefix = terminal.output_mut().read_range(range.start, length).map_err(|e| e.to_string())?;
                let prefix = String::from_utf8(prefix).map_err(|e| e.to_string())?;
                let x = terminal.output_desired_x().unwrap_or_else(|| self.terminal_text_width(&prefix));
                let target_row = row.saturating_add_signed(delta).min(self.terminal_layout.len().saturating_sub(1));
                let mut target = self.terminal_offset_on_row(terminal, target_row, x)?;
                // A shared wrap boundary belongs to the following row. Keep
                // vertical movement on the requested row, even past its text.
                if self.terminal_layout.row_at(target) > target_row {
                    target = terminal.output_mut().previous_char_boundary(target).map_err(|e| e.to_string())?;
                }
                terminal.move_output_cursor(target, extend).map_err(|e| e.to_string())?;
                terminal.set_output_desired_x(x);
            }
            OutputCommand::RowEdge(end, extend) => {
                let range = self.terminal_layout.row_range(row, terminal.output_mut()).map_err(|e| e.to_string())?.unwrap_or(0..0);
                terminal.move_output_cursor(if end { range.end } else { range.start }, extend).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
        if terminal.output_focused() {
            let row = self.terminal_layout.row_at(terminal.output_cursor());
            let visible = self.visible_terminal_line_count();
            let rows = self.terminal_layout.visible_rows(visible, terminal.scroll_back());
            let start = if row < rows.start { row } else if row >= rows.end {
                row.saturating_add(1).saturating_sub(visible)
            } else { rows.start };
            terminal.set_scroll_back(self.terminal_layout.len().saturating_sub(visible).saturating_sub(start));
        }
        Ok(())
    }

    pub(crate) fn terminal_columns(&self) -> usize {
        ((self.terminal_bounds().1 - 24).max(1) / self.char_width.max(1)).max(1) as usize
    }

    fn update_terminal_layout(&mut self, terminal: &mut Terminal) -> Result<(), String> {
        terminal.set_listing_width(self.terminal_columns());
        let metrics = WrapMetrics {
            width: (self.terminal_bounds().1 - 24).max(1),
            cell_width: self.char_width.max(1),
            tab_width: self.tab_width,
            font_size: self.logical_font_size.to_bits(),
        };
        let generation = terminal.output_generation();
        let origin = terminal.output_layout_origin();
        if origin.0 == self.terminal_layout_origin.0 && origin.1 > self.terminal_layout_origin.1 {
            self.terminal_layout.discard_prefix(origin.1 - self.terminal_layout_origin.1, generation);
        }
        self.terminal_layout_origin = origin;
        let mut layout = std::mem::take(&mut self.terminal_layout);
        let update = layout.update(terminal.output_mut(), generation, metrics, |character| {
            self.logical_text_size(&character.to_string()).0.ceil() as i32
        });
        self.terminal_layout = layout;
        let added_rows = update.map_err(|error| error.to_string())?;
        let scroll_back = terminal.scroll_back();
        let scroll_back = if scroll_back > 0 || terminal.output_focused() { scroll_back.saturating_add(added_rows) } else { 0 };
        terminal.set_scroll_back(scroll_back.min(
            self.terminal_layout.len().saturating_sub(self.visible_terminal_line_count())
        ));
        Ok(())
    }

    fn terminal_text_width(&self, text: &str) -> i32 {
        let mut width = 0;
        let mut column = 0;
        let mut segments = text.split('\t').peekable();
        while let Some(segment) = segments.next() {
            width += self.logical_text_size(segment).0 as i32;
            column = Self::visual_column_after_text(segment, column, self.tab_width);
            if segments.peek().is_some() {
                let advance = Self::visual_advance('\t', column, self.tab_width);
                width += advance as i32 * self.char_width.max(1);
                column += advance;
            }
        }
        width
    }

    fn terminal_status_lines(&self, status: &str) -> Vec<String> {
        let width = (self.window_width - 2 * (TERMINAL_BAR_MARGIN + 10)).max(1);
        wrap_terminal_status(status, width, |text| self.terminal_text_width(text))
    }

    fn render_terminal_bar(
        &mut self,
        terminal: &Terminal,
        status_lines: &[String],
    ) -> Result<(), String> {
        let y = self.window_height
            - TERMINAL_BAR_MARGIN
            - TERMINAL_BAR_HEIGHT;

        let width = self.window_width
            - TERMINAL_BAR_MARGIN * 2;

        let status_y = y - status_lines.len() as i32 * self.font.height().max(1) - 8;
        for (index, line) in status_lines.iter().enumerate() {
            self.render_dpi_text(
                line,
                (TERMINAL_BAR_MARGIN + 10) as f32,
                (status_y + index as i32 * self.font.height().max(1)) as f32,
                Color::RGB(225, 205, 150),
            )?;
        }

        self.canvas.set_draw_color(
            Color::RGB(40, 43, 43),
        );
        self.canvas.fill_rect(Rect::new(
            TERMINAL_BAR_MARGIN,
            y,
            width.max(1) as u32,
            TERMINAL_BAR_HEIGHT as u32,
        )).map_err(|error| {
            error.to_string()
        })?;

        let editor_left = self.window_width
            - TERMINAL_BAR_MARGIN
            - TERMINAL_EDITOR_WIDTH;
        let clear_left = editor_left
            - TERMINAL_CLEAR_WIDTH;
        let action_left = clear_left
            - TERMINAL_ACTION_WIDTH;

        self.render_terminal_button(
            action_left,
            y,
            TERMINAL_ACTION_WIDTH,
            if terminal.is_running() {
                "Stop"
            } else {
                "Again"
            },
            terminal.is_running(),
        )?;

        self.render_terminal_button(
            clear_left,
            y,
            TERMINAL_CLEAR_WIDTH,
            if terminal.can_go_back() { "Back" } else { "Clear" },
            false,
        )?;

        self.render_terminal_button(
            editor_left,
            y,
            TERMINAL_EDITOR_WIDTH,
            "Editor",
            false,
        )?;

        let text_y = y
            + (TERMINAL_BAR_HEIGHT
                - self.font.height()) / 2;

        let text_x = TERMINAL_BAR_MARGIN + 10;

        let field_right =
            action_left - 8;

        let field_width = field_right
            .saturating_sub(text_x)
            .max(1);

        let clip = Rect::new(
            text_x,
            y,
            field_width as u32,
            TERMINAL_BAR_HEIGHT as u32,
        );

        self.canvas.set_clip_rect(Some(clip));

        let input = terminal.input();
        let cursor = terminal.cursor()
            .min(input.len());

        let prefix = input.get(..cursor)
            .unwrap_or(input);

        let cursor_width = self.font
            .size_of(prefix)
            .map(|(width, _)| width as i32)
            .unwrap_or_else(|_| {
                prefix.chars().count() as i32
                    * self.char_width
            });

        let input_width = self.font
            .size_of(input)
            .map(|(width, _)| width as i32)
            .unwrap_or_else(|_| {
                input.chars().count() as i32
                    * self.char_width
            });

        let scroll_x =
            Self::search_query_scroll_x(
                cursor_width,
                input_width,
                field_width,
                4,
            );

        self.render_dpi_text(
            input,
            (text_x - scroll_x) as f32,
            text_y as f32,
            if terminal.is_running() {
                Color::RGB(145, 145, 145)
            } else {
                Color::RGB(235, 235, 235)
            },
        )?;

        if !terminal.is_running() && !terminal.output_focused()
            && (!self.split_mode || self.terminal_pane == self.active_pane) {
            self.canvas.set_draw_color(
                Color::RGB(245, 245, 245),
            );
            self.canvas.draw_line(
                Point::new(
                    text_x + cursor_width
                        - scroll_x,
                    y + 9,
                ),
                Point::new(
                    text_x + cursor_width
                        - scroll_x,
                    y + TERMINAL_BAR_HEIGHT - 9,
                ),
            ).map_err(|error| {
                error.to_string()
            })?;
        }

        self.canvas.set_clip_rect(None);

        Ok(())
    }

    fn render_terminal_button(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        label: &str,
        emphasized: bool,
    ) -> Result<(), String> {
        self.canvas.set_draw_color(
            if emphasized {
                Color::RGB(105, 60, 60)
            } else {
                Color::RGB(55, 65, 70)
            },
        );

        self.canvas.fill_rect(Rect::new(
            x + 2,
            y + 5,
            width.saturating_sub(4) as u32,
            TERMINAL_BAR_HEIGHT
                .saturating_sub(10) as u32,
        )).map_err(|error| {
            error.to_string()
        })?;

        let label_width = self.font
            .size_of(label)
            .map(|(width, _)| width as i32)
            .unwrap_or_else(|_| {
                label.chars().count() as i32
                    * self.char_width
            });

        self.render_dpi_text(
            label,
            (x + (width - label_width) / 2)
                as f32,
            (y + (TERMINAL_BAR_HEIGHT
                - self.font.height()) / 2)
                as f32,
            Color::RGB(225, 225, 225),
        )?;

        Ok(())
    }

    // ----------------------------------------------------------------------
    // Layout
    // ----------------------------------------------------------------------

    fn text_left(
        &self,
        table: &PieceTable,
    ) -> Result<i32, String> {
        let lines =
            table.cached_line_count();

        let gutter =
            LineNumbers::width(
                &self.font,
                lines,
            )?;

        Ok(20 + gutter)
    }

    fn text_clip_rect(
        &self,
        table: &PieceTable,
    ) -> Result<Rect, String> {
        let text_left =
            self.text_left(table)?;

        Ok(Rect::new(
            text_left,
            TITLE_BAR_HEIGHT,
            (
                self.window_width
                    - text_left
            ).max(0) as u32,
            (
                self.window_height
                    - TITLE_BAR_HEIGHT
                    - self.bottom_inset
            ).max(0) as u32,
        ))
    }

    // ----------------------------------------------------------------------
    // Text
    // ----------------------------------------------------------------------

    fn render_text(
        &mut self,
        table: &mut PieceTable,
    ) -> Result<(), String> {
        let text_left = self.text_left(table)?;
        let first = self.scroll_line;
        let last = first.saturating_add(self.visible_line_count()).min(table.cached_line_count());
        let (left, right) = self.visible_visual_columns(text_left);
        const NORMAL: Color = Color::RGB(220, 220, 220);
        for line in first..last {
            let window = table.line_window(line, left, right, self.tab_width).map_err(|e| e.to_string())?;
            if window.text.is_empty() { continue; }
            let y = TITLE_BAR_HEIGHT + 8 + (line - first) as i32 * self.font.height();
            let matches = if let Some(syntax) = &self.syntax {
                if window.first_byte < crate::syntax_core::MAX_HIGHLIGHT_BYTES {
                    let prefix = table.line_prefix(line, crate::syntax_core::MAX_HIGHLIGHT_BYTES)
                        .map_err(|e| e.to_string())?;
                    syntax.matches_line(&prefix)
                } else { Vec::new() }
            } else { Vec::new() };
            let mut byte = 0;
            let mut visual = window.first_visual;
            for hit in matches {
                let start = hit.start.saturating_sub(window.first_byte).min(window.text.len());
                let end = hit.end.saturating_sub(window.first_byte).min(window.text.len());
                if end <= byte { continue; }
                if start > byte {
                    visual = self.render_text_chunk(&window.text[byte..start],
                        self.x_for_visual_column(visual, text_left), visual, y, NORMAL)?;
                }
                visual = self.render_text_chunk(&window.text[start.max(byte)..end],
                    self.x_for_visual_column(visual, text_left), visual, y, hit.color)?;
                byte = end;
            }
            if byte < window.text.len() {
                self.render_text_chunk(&window.text[byte..], self.x_for_visual_column(visual, text_left),
                    visual, y, NORMAL)?;
            }
        }
        Ok(())
    }

    fn visible_visual_columns(&self, text_left: i32) -> (usize, usize) {
        let cell = self.char_width.max(1) as usize;
        let left = (self.scroll_x.max(0) as usize / cell).saturating_sub(1);
        let right = (self.scroll_x.max(0) as usize)
            .saturating_add(self.active_content_width().saturating_sub(text_left).max(0) as usize)
            / cell + 2;
        (left, right)
    }

    fn clipped_column_x(&self, table: &mut PieceTable, line: usize, column: usize, text_left: i32)
        -> Result<i32, String>
    {
        let (left, right) = self.visible_visual_columns(text_left);
        let window = table.line_window(line, left, right, self.tab_width).map_err(|e| e.to_string())?;
        let count = window.text.chars().count();
        let visual = if column < window.first_column { left }
            else if column > window.first_column.saturating_add(count) { right }
            else {
                window.text.chars().take(column - window.first_column).fold(window.first_visual,
                    |v, c| v.saturating_add(Self::visual_advance(c, v, self.tab_width)))
            };
        Ok(self.x_for_visual_column(visual, text_left))
    }

    fn render_text_chunk(
        &mut self,
        text: &str,
        start_x: i32,
        start_visual_column: usize,
        y: i32,
        color: Color,
    ) -> Result<usize, String> {
        self.render_text_chunk_with_right(
            text,
            start_x,
            start_visual_column,
            y,
            color,
            self.active_content_width(),
        )
    }

    fn render_terminal_text_chunk(
        &mut self,
        text: &str,
        start_x: i32,
        start_visual_column: usize,
        y: i32,
        color: Color,
    ) -> Result<usize, String> {
        self.render_text_chunk_with_right(
            text,
            start_x,
            start_visual_column,
            y,
            color,
            self.window_width,
        )
    }

    fn render_text_chunk_with_right(
        &mut self,
        text: &str,
        start_x: i32,
        start_visual_column: usize,
        y: i32,
        color: Color,
        visible_right: i32,
    ) -> Result<usize, String> {
        if text.is_empty() {
            return Ok(start_visual_column);
        }

        let mut current_x =
            start_x;

        let mut visual_column =
            start_visual_column;

        let mut segment_start =
            0usize;

        for (byte_index, character) in text.char_indices() {
            if character != '\t' {
                continue;
            }

            if byte_index > segment_start {
                let segment =
                    &text[
                        segment_start
                            ..byte_index
                    ];

                self.render_plain_text(
                    segment,
                    current_x,
                    y,
                    color,
                    MAX_TEXT_TEXTURE_WIDTH,
                    Some((0.0, visible_right as f32)),
                )?;

                let segment_width =
                    self.font
                        .size_of(segment)
                        .map(|(width, _)| {
                            i32::try_from(width)
                                .unwrap_or(i32::MAX)
                        })
                        .unwrap_or_else(|_| {
                            i32::try_from(
                                segment.chars().count(),
                            )
                            .unwrap_or(i32::MAX)
                            .saturating_mul(
                                self.char_width.max(1),
                            )
                        });

                current_x =
                    current_x.saturating_add(
                        segment_width,
                    );

                visual_column =
                    Self::visual_column_after_text(
                        segment,
                        visual_column,
                        self.tab_width,
                    );
            }

            let next_visual_column =
                visual_column.saturating_add(
                    Self::visual_advance(
                        character,
                        visual_column,
                        self.tab_width,
                    ),
                );

            let advance =
                next_visual_column
                    .saturating_sub(visual_column);

            let advance_pixels =
                i32::try_from(advance)
                    .unwrap_or(i32::MAX)
                    .saturating_mul(
                        self.char_width.max(1),
                    );

            current_x =
                current_x.saturating_add(
                    advance_pixels,
                );

            visual_column =
                next_visual_column;

            segment_start =
                byte_index
                    + character.len_utf8();
        }

        if segment_start < text.len() {
            let segment =
                &text[
                    segment_start..
                ];

            self.render_plain_text(
                segment,
                current_x,
                y,
                color,
                MAX_TEXT_TEXTURE_WIDTH,
                Some((0.0, visible_right as f32)),
            )?;

            visual_column =
                Self::visual_column_after_text(
                    segment,
                    visual_column,
                    self.tab_width,
                );
        }

        Ok(visual_column)
    }

    fn render_plain_text(
        &mut self,
        text: &str,
        start_x: i32,
        y: i32,
        color: Color,
        max_texture_width: i32,
        visible_bounds: Option<(f32, f32)>,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }

        // The texture limit is a physical-pixel limit. At 200% DPI each
        // glyph is roughly twice as wide, so reduce chunk length before
        // asking SDL_ttf to allocate the high-resolution surface.
        let raster_char_width =
            (
                self.char_width.max(1) as f32
                    * self.dpi_text
                        .raster_scale()
            )
                .ceil()
                .max(1.0) as i32;

        let max_chars =
            (
                max_texture_width
                    / raster_char_width
            )
            .max(1) as usize;

        let mut byte_offset =
            0usize;

        let mut current_x =
            start_x as f32;

        while byte_offset < text.len() {
            if visible_bounds
                .map(|(_, right)| {
                    current_x >= right
                })
                .unwrap_or(false)
            {
                break;
            }

            let chunk_end =
                text[byte_offset..]
                    .char_indices()
                    .nth(max_chars)
                    .map(|(offset, _)| {
                        byte_offset
                            .saturating_add(offset)
                    })
                    .unwrap_or(text.len());

            let chunk =
                &text[
                    byte_offset
                        ..chunk_end
                ];

            let (chunk_width, chunk_height) =
                self.logical_text_size(
                    chunk,
                );

            let chunk_right =
                current_x
                    + chunk_width;

            let is_visible =
                visible_bounds
                    .map(|(left, right)| {
                        current_x < right
                            && chunk_right > left
                    })
                    .unwrap_or(true);

            if is_visible {
                self.render_dpi_text_with_size(
                    chunk,
                    current_x,
                    y as f32,
                    color,
                    chunk_width,
                    chunk_height,
                )?;
            }

            current_x =
                chunk_right;

            byte_offset =
                chunk_end;
        }

        Ok(())
    }

    // ----------------------------------------------------------------------
    // Selection
    // ----------------------------------------------------------------------

    fn render_selection(
        &mut self,
        table: &mut PieceTable,
    ) -> Result<(), String> {
        self.render_cursor_selection(table, &table.cursor.clone())?;
        for index in 0..table.secondary_cursors.len() {
            let cursor = table.secondary_cursors[index].clone();
            self.render_cursor_selection(table, &cursor)?;
        }
        Ok(())
    }

    fn render_cursor_selection(
        &mut self,
        table: &mut PieceTable,
        cursor: &crate::piece_table::Cursor,
    ) -> Result<(), String> {
        if cursor.position == cursor.anchor {
            return Ok(());
        }

        let (
            start_line,
            start_column,
            end_line,
            end_column,
        ) =
            if cursor.position
                <= cursor.anchor
            {
                (
                    cursor.line,
                    cursor.column,
                    cursor.anchor_line,
                    cursor.anchor_column,
                )
            } else {
                (
                    cursor.anchor_line,
                    cursor.anchor_column,
                    cursor.line,
                    cursor.column,
                )
            };

        let visible =
            self.visible_line_count();

        let first =
            self.scroll_line;

        let last =
            first + visible;

        self.canvas.set_draw_color(
            Color::RGB(
                70,
                100,
                160,
            ),
        );

        let text_left =
            self.text_left(table)?;

        for line in start_line.max(first)..end_line.saturating_add(1).min(last) {

            let length = table.line_length(line).map_err(|e| e.to_string())?;

            let from =
                if line == start_line {
                    start_column
                } else {
                    0
                };

            let to =
                if line == end_line {
                    end_column
                } else {
                    length
                };

            let x =
                self.clipped_column_x(table, line, from, text_left)?;

            let y =
                TITLE_BAR_HEIGHT
                    + 8
                    + (
                        line - first
                    ) as i32
                        * self.font.height();

            if to <= from {
                continue;
            }

            let end_x =
                self.clipped_column_x(table, line, to, text_left)?;

            let width =
                (end_x - x)
                    .max(1) as u32;

            self.canvas
                .fill_rect(Rect::new(
                    x,
                    y,
                    width,
                    self.font.height() as u32,
                ))
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    // ----------------------------------------------------------------------
    // Search match
    // ----------------------------------------------------------------------

    fn render_search_match(
        &mut self,
        table: &mut PieceTable,
        search_ui: &SearchUi,
    ) -> Result<(), String> {
        let current =
            match search_ui.current_match() {
                Some(current) => current,
                None => return Ok(()),
            };

        let (
            start_line,
            start_column,
        ) =
            table
                .line_column_at(
                    current.start,
                )
                .map_err(|e| e.to_string())?;

        let (
            end_line,
            end_column,
        ) =
            table
                .line_column_at(
                    current.end,
                )
                .map_err(|e| e.to_string())?;

        let first =
            self.scroll_line;

        let last =
            first
                + self.visible_line_count();

        let text_left =
            self.text_left(table)?;

        self.canvas.set_draw_color(
            Color::RGB(
                110,
                85,
                20,
            ),
        );

        for line in start_line..=end_line {
            if line < first
                || line >= last
            {
                continue;
            }

            let line_length = table.line_length(line).map_err(|e| e.to_string())?;

            let from =
                if line == start_line {
                    start_column
                } else {
                    0
                };

            let to =
                if line == end_line {
                    end_column
                } else {
                    line_length
                };

            if to <= from {
                if current.start == current.end
                    && line == start_line
                {
                    let x =
                        self.clipped_column_x(table, line, from, text_left)?;

                    let y =
                        TITLE_BAR_HEIGHT
                            + 8
                            + (
                                line - first
                            ) as i32
                                * self.font.height();

                    self.canvas
                        .fill_rect(Rect::new(
                            x,
                            y,
                            2,
                            self.font.height() as u32,
                        ))
                        .map_err(|e| e.to_string())?;
                }

                continue;
            }

            let x =
                self.clipped_column_x(table, line, from, text_left)?;

            let end_x =
                self.clipped_column_x(table, line, to, text_left)?;

            let width =
                (end_x - x)
                    .max(1) as u32;

            let y =
                TITLE_BAR_HEIGHT
                    + 8
                    + (
                        line - first
                    ) as i32
                        * self.font.height();

            self.canvas
                .fill_rect(Rect::new(
                    x,
                    y,
                    width,
                    self.font.height() as u32,
                ))
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    // ----------------------------------------------------------------------
    // Cursor
    // ----------------------------------------------------------------------

    fn render_cursor(
        &mut self,
        table: &mut PieceTable,
    ) -> Result<(), String> {
        self.render_cursor_at(table, self.cursor_line, self.cursor_column)?;
        for index in 0..table.secondary_cursors.len() {
            let cursor = &table.secondary_cursors[index];
            let (line, column) = (cursor.line, cursor.column);
            self.render_cursor_at(table, line, column)?;
        }
        Ok(())
    }

    fn render_cursor_at(
        &mut self,
        table: &mut PieceTable,
        line: usize,
        column: usize,
    ) -> Result<(), String> {
        let visible =
            self.visible_line_count();

        if line < self.scroll_line
            || line >= self.scroll_line + visible
        {
            return Ok(());
        }

        let text_left = self.text_left(table)?;
        let x = self.clipped_column_x(table, line, column, text_left)?;

        let y =
            TITLE_BAR_HEIGHT
                + 8
                + (
                    line - self.scroll_line
                ) as i32
                    * self.font.height();

        self.canvas.set_draw_color(
            Color::RGB(
                255,
                255,
                255,
            ),
        );

        self.canvas
            .draw_line(
                Point::new(
                    x,
                    y,
                ),
                Point::new(
                    x,
                    y + self.font.height(),
                ),
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    // ----------------------------------------------------------------------
    // Command bar
    // ----------------------------------------------------------------------

    pub fn command_bar_hit_at(
        &self,
        command_bar: &CommandBar,
        x: i32,
        y: i32,
    ) -> CommandBarHit {
        command_bar.hit_test(
            self.window_width,
            self.window_height,
            x,
            y,
        )
    }

    pub fn command_bar_cursor_at(
        &self,
        command_bar: &CommandBar,
        x: i32,
    ) -> usize {
        let text = command_bar.input();
        let text_x =
            COMMAND_BAR_MARGIN + 12;

        if x <= text_x {
            return 1.min(text.len());
        }

        let input_width =
            self.command_font.size_of(text)
                .map(|(width, _)| width as i32)
                .unwrap_or_else(|_| {
                    text.chars().count() as i32
                        * self.command_char_width
                });

        let prefix =
            text.get(..command_bar.cursor())
                .unwrap_or(text);

        let cursor_width =
            self.command_font.size_of(prefix)
                .map(|(width, _)| width as i32)
                .unwrap_or_else(|_| {
                    prefix.chars().count() as i32
                        * self.command_char_width
                });

        let field_width =
            (self.window_width
                - COMMAND_BAR_MARGIN * 2
                - COMMAND_RUN_WIDTH
                - 24)
                .max(1);

        let scroll_x =
            Self::search_query_scroll_x(
                cursor_width,
                input_width,
                field_width,
                4,
            );

        let target =
            x - text_x + scroll_x;

        let mut previous_width = 0i32;

        for (index, character) in
            text.char_indices()
        {
            let end =
                index + character.len_utf8();

            let width =
                self.command_font
                    .size_of(&text[..end])
                    .map(|(width, _)| {
                        width as i32
                    })
                    .unwrap_or_else(|_| {
                        text[..end]
                            .chars()
                            .count() as i32
                            * self.command_char_width
                    });

            if target
                < previous_width
                    + (width - previous_width) / 2
            {
                return index.max(1);
            }

            previous_width = width;
        }

        text.len()
    }

    fn render_command_bar(
        &mut self,
        command_bar: &CommandBar,
        search_ui: &SearchUi,
    ) -> Result<(), String> {
        if !command_bar.is_active() {
            return Ok(());
        }

        // Reuse the DPI-aware, bounded text renderer with the panel's fonts.
        // Restore editor metrics even if drawing returns an error.
        std::mem::swap(&mut self.font, &mut self.command_font);
        std::mem::swap(&mut self.raster_font, &mut self.command_raster_font);
        std::mem::swap(&mut self.char_width, &mut self.command_char_width);
        let result = self.render_command_bar_contents(command_bar, search_ui);
        std::mem::swap(&mut self.font, &mut self.command_font);
        std::mem::swap(&mut self.raster_font, &mut self.command_raster_font);
        std::mem::swap(&mut self.char_width, &mut self.command_char_width);
        self.canvas.set_clip_rect(None);
        result
    }

    fn render_command_bar_contents(
        &mut self,
        command_bar: &CommandBar,
        search_ui: &SearchUi,
    ) -> Result<(), String> {
        if !command_bar.is_active() {
            return Ok(());
        }

        let count =
            command_bar.suggestion_count();

        let panel_height =
            command_bar.panel_height();

        let x = COMMAND_BAR_MARGIN;
        let y =
            self.window_height
                - COMMAND_BAR_MARGIN
                - panel_height;

        let width =
            (self.window_width
                - COMMAND_BAR_MARGIN * 2)
                .max(1);

        self.canvas.set_draw_color(
            Color::RGB(38, 38, 38),
        );

        self.canvas
            .fill_rect(Rect::new(
                x,
                y,
                width as u32,
                panel_height as u32,
            ))
            .map_err(|error| {
                error.to_string()
            })?;

        self.canvas.set_draw_color(
            Color::RGB(100, 100, 100),
        );

        self.canvas
            .draw_rect(Rect::new(
                x,
                y,
                width as u32,
                panel_height as u32,
            ))
            .map_err(|error| {
                error.to_string()
            })?;

        for index in 0..count {
            let Some(suggestion) =
                command_bar.suggestion(index)
            else {
                continue;
            };

            let row_y = y + command_bar.suggestion_row_offset(index);
            let row_height = command_bar.suggestion_row_height(index);

            if command_bar.is_info() {
                if suggestion.label.is_empty() { continue; }
                let color = if command_bar.info_is_documentation(index) {
                    self.syntax.as_ref()
                        .and_then(|syntax| syntax.rules.iter().find(|rule| rule.name.eq_ignore_ascii_case("comment")))
                        .map(|rule| rule.color)
                        .unwrap_or(Color::RGB(106, 153, 85))
                } else { Color::RGB(235, 235, 235) };
                let clip = Rect::new(x + 12, row_y, (width - 24).max(1) as u32, row_height as u32);
                let text_width = self.font.size_of(suggestion.label).map(|(w, _)| w as i32).unwrap_or(0);
                self.canvas.set_clip_rect(Some(clip));
                self.render_clipped_single_line_text(
                    suggestion.label, x + 12,
                    row_y + (row_height - self.font.height()) / 2,
                    clip, text_width, color,
                )?;
                self.canvas.set_clip_rect(None);
                continue;
            }

            if index == command_bar.selected() {
                self.canvas.set_draw_color(
                    Color::RGB(55, 75, 95),
                );

                self.canvas
                    .fill_rect(Rect::new(
                        x + 1,
                        row_y + 1,
                        width.saturating_sub(2)
                            as u32,
                        COMMAND_SUGGESTION_HEIGHT
                            .saturating_sub(1)
                            as u32,
                    ))
                    .map_err(|error| {
                        error.to_string()
                    })?;
            }

            let marker =
                if suggestion.active {
                    "●"
                } else {
                    "›"
                };

            let text_y =
                row_y
                    + (COMMAND_SUGGESTION_HEIGHT
                        - self.font.height()) / 2;

            self.render_dpi_text(
                marker,
                (x + 10) as f32,
                text_y as f32,
                if suggestion.active {
                    Color::RGB(125, 205, 155)
                } else {
                    Color::RGB(145, 175, 205)
                },
            )?;

            self.render_dpi_text(
                suggestion.label,
                (x + 34) as f32,
                text_y as f32,
                Color::RGB(235, 235, 235),
            )?;

            if width >= 520 {
                let description_width =
                    self.font
                        .size_of(
                            suggestion.description
                        )
                        .map(|(width, _)| {
                            width as i32
                        })
                        .unwrap_or_else(|_| {
                            suggestion.description
                                .chars()
                                .count() as i32
                                * self.char_width
                        });

                let description_x =
                    x + width
                        - description_width
                        - 12;

                self.render_dpi_text(
                    suggestion.description,
                    description_x as f32,
                    text_y as f32,
                    Color::RGB(165, 165, 165),
                )?;
            }
        }

        let input_y = y + command_bar.suggestions_height();

        self.canvas.set_draw_color(
            Color::RGB(105, 155, 205),
        );

        self.canvas
            .draw_rect(Rect::new(
                x + 5,
                input_y + 5,
                width.saturating_sub(10)
                    as u32,
                COMMAND_INPUT_HEIGHT
                    .saturating_sub(10)
                    as u32,
            ))
            .map_err(|error| {
                error.to_string()
            })?;

        let run_x =
            x + width - COMMAND_RUN_WIDTH;

        self.canvas.set_draw_color(
            Color::RGB(55, 75, 95),
        );

        self.canvas
            .fill_rect(Rect::new(
                run_x,
                input_y + 5,
                COMMAND_RUN_WIDTH
                    .saturating_sub(5)
                    as u32,
                COMMAND_INPUT_HEIGHT
                    .saturating_sub(10)
                    as u32,
            ))
            .map_err(|error| {
                error.to_string()
            })?;

        let run_label = "Run  ↵";
        let run_label_width =
            self.font.size_of(run_label)
                .map(|(width, _)| width as i32)
                .unwrap_or_else(|_| {
                    run_label.chars().count()
                        as i32
                        * self.char_width
                });

        let run_text_y =
            input_y
                + (COMMAND_INPUT_HEIGHT
                    - self.font.height()) / 2;

        self.render_dpi_text(
            run_label,
            (run_x
                + (COMMAND_RUN_WIDTH
                    - run_label_width) / 2)
                as f32,
            run_text_y as f32,
            Color::RGB(225, 235, 245),
        )?;

        let input = command_bar.input();
        let cursor =
            command_bar.cursor()
                .min(input.len());

        let prefix =
            input.get(..cursor)
                .unwrap_or(input);

        let cursor_width =
            self.font.size_of(prefix)
                .map(|(width, _)| width as i32)
                .unwrap_or_else(|_| {
                    prefix.chars().count() as i32
                        * self.char_width
                });

        let input_width =
            self.font.size_of(input)
                .map(|(width, _)| width as i32)
                .unwrap_or_else(|_| {
                    input.chars().count() as i32
                        * self.char_width
                });

        let status =
            command_bar.status()
                .map(str::to_owned)
                .or_else(|| {
                    if search_ui.pattern_error()
                        .is_some()
                    {
                        Some(
                            "invalid regex".to_string()
                        )
                    } else if search_ui.search_failed() {
                        Some("no match".to_string())
                    } else {
                        search_ui.last_replace_count()
                            .map(|count| {
                                format!(
                                    "{count} replaced"
                                )
                            })
                    }
                });

        let status = status.or_else(|| command_bar.navigation_hint());

        let status_width =
            status.as_deref()
                .map(|status| {
                    self.font.size_of(status)
                        .map(|(width, _)| {
                            width as i32
                        })
                        .unwrap_or_else(|_| {
                            status.chars().count()
                                as i32
                                * self.char_width
                        })
                })
                .unwrap_or(0);

        let text_x = x + 12;
        let status_gap = 16;
        let right = run_x - 12;
        // Formatter diagnostics can be much longer than the command field.
        // Reserve at most half the available width and render through the
        // existing bounded texture path, keeping the input visible.
        let status_space = status_width.min(((right - text_x) / 2).max(0));

        let text_right =
            if status.is_some()
                && width >= 420
            {
                right
                    - status_space
                    - status_gap
            } else {
                right
            };

        let field_width =
            (text_right - text_x)
                .max(1);

        let scroll_x =
            Self::search_query_scroll_x(
                cursor_width,
                input_width,
                field_width,
                4,
            );

        let text_y =
            input_y
                + (COMMAND_INPUT_HEIGHT
                    - self.font.height()) / 2;

        let clip = Rect::new(
            text_x,
            input_y,
            field_width as u32,
            COMMAND_INPUT_HEIGHT as u32,
        );

        self.canvas.set_clip_rect(Some(clip));

        self.render_clipped_single_line_text(
            input,
            text_x - scroll_x,
            text_y,
            clip,
            input_width,
            Color::RGB(235, 235, 235),
        )?;

        self.canvas.set_draw_color(
            Color::RGB(255, 255, 255),
        );

        let caret_x =
            text_x
                + cursor_width
                - scroll_x;

        self.canvas
            .draw_line(
                Point::new(
                    caret_x,
                    input_y + 8,
                ),
                Point::new(
                    caret_x,
                    input_y
                        + COMMAND_INPUT_HEIGHT
                        - 8,
                ),
            )
            .map_err(|error| {
                error.to_string()
            })?;

        self.canvas.set_clip_rect(None);

        if let Some(status) = status
            .as_deref()
            .filter(|_| width >= 420)
        {
            let status_clip = Rect::new(right - status_space, input_y, status_space.max(1) as u32, COMMAND_INPUT_HEIGHT as u32);
            self.canvas.set_clip_rect(Some(status_clip));
            self.render_clipped_single_line_text(
                status,
                right - status_space,
                text_y,
                status_clip,
                status_width,
                if command_bar.status().is_some()
                    || search_ui.pattern_error()
                        .is_some()
                    || search_ui.search_failed()
                {
                    Color::RGB(220, 155, 155)
                } else {
                    Color::RGB(150, 210, 165)
                },
            )?;
            self.canvas.set_clip_rect(None);
        }

        Ok(())
    }

    // ----------------------------------------------------------------------
    // Legacy search bar geometry helpers
    // ----------------------------------------------------------------------

    #[allow(dead_code)]
    fn render_search_bar(
        &mut self,
        search_ui: &SearchUi,
    ) -> Result<(), String> {
        if !search_ui.is_active() {
            return Ok(());
        }

        let bar_height =
            Self::search_bar_height(search_ui);

        const BAR_MARGIN: i32 = SEARCH_BAR_MARGIN;
        const INNER_MARGIN: i32 = 10;
        const QUERY_GAP: i32 = 8;
        const STATUS_GAP: i32 = 16;

        let x =
            BAR_MARGIN;

        let y =
            self.window_height
                - bar_height
                - BAR_MARGIN;

        let width =
            (
                self.window_width
                    - BAR_MARGIN * 2
            )
            .max(1);

        self.canvas.set_draw_color(
            Color::RGB(
                45,
                45,
                45,
            ),
        );

        self.canvas
            .fill_rect(Rect::new(
                x,
                y,
                width as u32,
                bar_height as u32,
            ))
            .map_err(|e| e.to_string())?;

        self.canvas.set_draw_color(
            Color::RGB(
                100,
                100,
                100,
            ),
        );

        self.canvas
            .draw_rect(Rect::new(
                x,
                y,
                width as u32,
                bar_height as u32,
            ))
            .map_err(|e| e.to_string())?;

        let text_y =
            y
                + (
                    SEARCH_ROW_HEIGHT
                        - self.font.height()
                ) / 2;

        let label =
            "Find:";

        let label_width =
            self.font
                .size_of(label)
                .map(|(width, _)| width as i32)
                .unwrap_or(
                    label.chars().count() as i32
                        * self.char_width,
                );

        let label_x =
            x + INNER_MARGIN;

        let query_x =
            label_x
                + label_width
                + QUERY_GAP;

        self.render_dpi_text(
            label,
            label_x as f32,
            text_y as f32,
            Color::RGB(
                230,
                230,
                230,
            ),
        )?;

        let compact_layout =
            self.window_width < 640;

        let minimal_layout =
            self.window_width < 360;

        let status: Option<String> =
            if search_ui.pattern_error().is_some() {
                Some(
                    if minimal_layout {
                        "!".to_owned()
                    } else if compact_layout {
                        "invalid".to_owned()
                    } else {
                        "— invalid regex".to_owned()
                    }
                )
            } else if search_ui.search_failed() {
                Some(
                    if minimal_layout {
                        "0".to_owned()
                    } else if compact_layout {
                        "no match".to_owned()
                    } else {
                        "— no match".to_owned()
                    }
                )
            } else {
                search_ui.last_replace_count()
                    .map(|count| {
                        if minimal_layout {
                            count.to_string()
                        } else if compact_layout {
                            format!("{count} replaced")
                        } else {
                            format!("— {count} replaced")
                        }
                    })
            };

        let status_width =
            match status.as_deref() {
                Some(text) =>
                    self.font
                        .size_of(text)
                        .map(|(width, _)| width as i32)
                        .unwrap_or(
                            text.chars().count() as i32
                                * self.char_width,
                        ),

                None => 0,
            };

        let mode_key =
            if search_ui.is_replace_mode() {
                "Alt+M"
            } else {
                "Tab"
            };

        let mode =
            if minimal_layout {
                search_ui
                    .compact_mode_label()
                    .to_string()
            } else if compact_layout {
                format!(
                    "{} · {}",
                    search_ui.compact_mode_label(),
                    mode_key,
                )
            } else {
                format!(
                    "{} · {}",
                    search_ui.mode_label(),
                    mode_key,
                )
            };

        let mode_width =
            self.font
                .size_of(&mode)
                .map(|(width, _)| width as i32)
                .unwrap_or(
                    mode.chars().count() as i32
                        * self.char_width,
                );

        let right_edge =
            self.window_width
                - BAR_MARGIN
                - INNER_MARGIN;

        let available_width =
            right_edge
                .saturating_sub(query_x)
                .max(0);

        let (show_mode, show_status) =
            Self::search_bar_visibility(
                available_width,
                self.char_width,
                mode_width,
                status_width,
                STATUS_GAP,
                status.is_some(),
            );

        let mode_x =
            right_edge
                .saturating_sub(mode_width);

        let status_right =
            if show_mode {
                mode_x.saturating_sub(STATUS_GAP)
            } else {
                right_edge
            };

        let status_x =
            status_right
                .saturating_sub(status_width);

        let query_right =
            if show_status {
                status_x.saturating_sub(STATUS_GAP)
            } else if show_mode {
                mode_x.saturating_sub(STATUS_GAP)
            } else {
                right_edge
            };

        let query_field_width =
            (
                query_right
                    - query_x
            )
            .max(1);

        self.canvas.set_draw_color(
            if search_ui.focused_field()
                == SearchField::Query
            {
                Color::RGB(105, 155, 205)
            } else {
                Color::RGB(70, 70, 70)
            },
        );

        self.canvas
            .draw_rect(Rect::new(
                query_x.saturating_sub(4),
                y + 5,
                query_field_width
                    .saturating_add(8) as u32,
                (SEARCH_ROW_HEIGHT - 10) as u32,
            ))
            .map_err(|e| e.to_string())?;

        let query_clip =
            Rect::new(
                query_x,
                y,
                query_field_width as u32,
                SEARCH_ROW_HEIGHT as u32,
            );

        let query =
            search_ui.query();

        let cursor =
            search_ui.query_cursor()
                .min(query.len());

        // SearchUi stores the cursor as a UTF-8 byte offset.
        let prefix =
            query.get(..cursor)
                .unwrap_or(query);

        let cursor_width =
            if prefix.is_empty() {
                0
            } else {
                self.font
                    .size_of(prefix)
                    .map(|(width, _)| width as i32)
                    .unwrap_or(
                        prefix.chars().count() as i32
                            * self.char_width,
                    )
            };

        let query_width =
            if query.is_empty() {
                0
            } else {
                self.font
                    .size_of(query)
                    .map(|(width, _)| width as i32)
                    .unwrap_or(
                        query.chars().count() as i32
                            * self.char_width,
                    )
            };

        let padding =
            4;

        let query_scroll_x =
            Self::search_query_scroll_x(
                cursor_width,
                query_width,
                query_field_width,
                padding,
            );

        self.canvas.set_clip_rect(
            Some(query_clip),
        );

        if !query.is_empty() {
            self.render_clipped_single_line_text(
                query,
                query_x
                    - query_scroll_x,
                text_y,
                query_clip,
                query_width,
                Color::RGB(
                    230,
                    230,
                    230,
                ),
            )?;
        }

        let caret_x =
            query_x
                + cursor_width
                - query_scroll_x;

        if search_ui.focused_field()
            == SearchField::Query
        {
            self.canvas.set_draw_color(
                Color::RGB(
                    255,
                    255,
                    255,
                ),
            );

            self.canvas
                .draw_line(
                    Point::new(
                        caret_x,
                        y + 7,
                    ),
                    Point::new(
                        caret_x,
                        y
                            + SEARCH_ROW_HEIGHT
                            - 7,
                    ),
                )
                .map_err(|e| e.to_string())?;
        }

        self.canvas.set_clip_rect(None);

        if let Some(status) =
            status.as_deref()
                .filter(|_| show_status)
        {
            let status_color =
                if search_ui.pattern_error()
                    .is_some()
                    || search_ui.search_failed()
                {
                    Color::RGB(220, 150, 150)
                } else {
                    Color::RGB(150, 210, 165)
                };

            self.render_dpi_text(
                status,
                status_x as f32,
                text_y as f32,
                status_color,
            )?;
        }

        if show_mode {
            self.render_dpi_text(
                &mode,
                mode_x as f32,
                text_y as f32,
                Color::RGB(
                    155,
                    195,
                    235,
                ),
            )?;
        }

        if search_ui.is_replace_mode() {
            self.render_replacement_search_row(
                search_ui,
                x,
                y + SEARCH_ROW_HEIGHT,
                width,
            )?;
        }

        Ok(())
    }

    fn render_replacement_search_row(
        &mut self,
        search_ui: &SearchUi,
        bar_x: i32,
        row_y: i32,
        bar_width: i32,
    ) -> Result<(), String> {
        const INNER_MARGIN: i32 = 10;
        const FIELD_GAP: i32 = 8;
        const HINT_GAP: i32 = 16;

        self.canvas.set_draw_color(
            Color::RGB(65, 65, 65),
        );

        self.canvas
            .draw_line(
                Point::new(
                    bar_x + 1,
                    row_y,
                ),
                Point::new(
                    bar_x
                        + bar_width
                        - 2,
                    row_y,
                ),
            )
            .map_err(|e| e.to_string())?;

        let compact_layout =
            self.window_width < 640;

        let minimal_layout =
            self.window_width < 360;

        let label =
            if minimal_layout {
                "R:"
            } else {
                "Replace:"
            };

        let hint =
            if minimal_layout {
                "R / A"
            } else if compact_layout {
                "Alt+R · Alt+A"
            } else {
                "Alt+R replace · Alt+A all"
            };

        let text_y =
            row_y
                + (
                    SEARCH_ROW_HEIGHT
                        - self.font.height()
                ) / 2;

        let label_x =
            bar_x + INNER_MARGIN;

        let label_width =
            self.font
                .size_of(label)
                .map(|(width, _)| width as i32)
                .unwrap_or(
                    label.chars().count() as i32
                        * self.char_width,
                );

        let field_x =
            label_x
                + label_width
                + FIELD_GAP;

        let right_edge =
            bar_x
                + bar_width
                - INNER_MARGIN;

        let hint_width =
            self.font
                .size_of(hint)
                .map(|(width, _)| width as i32)
                .unwrap_or(
                    hint.chars().count() as i32
                        * self.char_width,
                );

        let available_width =
            right_edge
                .saturating_sub(field_x)
                .max(0);

        let show_hint =
            self.char_width
                .saturating_add(HINT_GAP)
                .saturating_add(hint_width)
                <= available_width;

        let hint_x =
            right_edge
                .saturating_sub(hint_width);

        let field_right =
            if show_hint {
                hint_x.saturating_sub(HINT_GAP)
            } else {
                right_edge
            };

        let field_width =
            field_right
                .saturating_sub(field_x)
                .max(1);

        self.render_dpi_text(
            label,
            label_x as f32,
            text_y as f32,
            Color::RGB(
                230,
                230,
                230,
            ),
        )?;

        self.canvas.set_draw_color(
            if search_ui.focused_field()
                == SearchField::Replacement
            {
                Color::RGB(105, 155, 205)
            } else {
                Color::RGB(70, 70, 70)
            },
        );

        self.canvas
            .draw_rect(Rect::new(
                field_x.saturating_sub(4),
                row_y + 5,
                field_width
                    .saturating_add(8) as u32,
                (SEARCH_ROW_HEIGHT - 10) as u32,
            ))
            .map_err(|e| e.to_string())?;

        let replacement =
            search_ui.replacement();

        let cursor =
            search_ui.replacement_cursor()
                .min(replacement.len());

        let prefix =
            replacement.get(..cursor)
                .unwrap_or(replacement);

        let cursor_width =
            if prefix.is_empty() {
                0
            } else {
                self.font
                    .size_of(prefix)
                    .map(|(width, _)| width as i32)
                    .unwrap_or(
                        prefix.chars().count() as i32
                            * self.char_width,
                    )
            };

        let replacement_width =
            if replacement.is_empty() {
                0
            } else {
                self.font
                    .size_of(replacement)
                    .map(|(width, _)| width as i32)
                    .unwrap_or(
                        replacement.chars().count() as i32
                            * self.char_width,
                    )
            };

        let scroll_x =
            Self::search_query_scroll_x(
                cursor_width,
                replacement_width,
                field_width,
                4,
            );

        let replacement_clip =
            Rect::new(
                field_x,
                row_y,
                field_width as u32,
                SEARCH_ROW_HEIGHT as u32,
            );

        self.canvas.set_clip_rect(
            Some(replacement_clip),
        );

        if !replacement.is_empty() {
            self.render_clipped_single_line_text(
                replacement,
                field_x
                    - scroll_x,
                text_y,
                replacement_clip,
                replacement_width,
                Color::RGB(
                    230,
                    230,
                    230,
                ),
            )?;
        }

        if search_ui.focused_field()
            == SearchField::Replacement
        {
            let caret_x =
                field_x
                    + cursor_width
                    - scroll_x;

            self.canvas.set_draw_color(
                Color::RGB(255, 255, 255),
            );

            self.canvas
                .draw_line(
                    Point::new(
                        caret_x,
                        row_y + 7,
                    ),
                    Point::new(
                        caret_x,
                        row_y
                            + SEARCH_ROW_HEIGHT
                            - 7,
                    ),
                )
                .map_err(|e| e.to_string())?;
        }

        self.canvas.set_clip_rect(None);

        if show_hint {
            self.render_dpi_text(
                hint,
                hint_x as f32,
                text_y as f32,
                Color::RGB(
                    155,
                    195,
                    235,
                ),
            )?;
        }

        Ok(())
    }
}

fn wrap_terminal_status(
    status: &str,
    width: i32,
    measure: impl Fn(&str) -> i32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for character in status.chars() {
        if character == '\n' {
            lines.push(std::mem::take(&mut line));
            continue;
        }
        let previous_len = line.len();
        line.push(character);
        if previous_len > 0 && measure(&line) > width.max(1) {
            line.truncate(previous_len);
            lines.push(std::mem::take(&mut line));
            line.push(character);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod terminal_status_tests {
    use super::wrap_terminal_status;

    #[test]
    fn wraps_long_errors_without_losing_unicode_text() {
        let status = "Could not open 東京/árvíz.txt: Both panes have unsaved changes.";
        let lines = wrap_terminal_status(status, 12, |text| text.chars().count() as i32);
        assert_eq!(lines.concat(), status);
        assert!(lines.iter().all(|line| line.chars().count() <= 12));
        assert!(lines.len() > 1);
    }

    #[test]
    fn status_handles_empty_text_newlines_and_tiny_windows() {
        assert!(wrap_terminal_status("", 10, |_| 0).is_empty());
        assert_eq!(wrap_terminal_status("first\nsecond", 80, |text| text.len() as i32), ["first", "second"]);
        assert_eq!(wrap_terminal_status("🙂é", 0, |_| 8), ["🙂", "é"]);
    }
}

#[cfg(test)]
fn fixed_split_pane_bounds(window_width: i32, pane: usize) -> (i32, i32) {
    split_pane_bounds(window_width, pane, 0.5)
}

fn split_pane_bounds(window_width: i32, pane: usize, ratio: f64) -> (i32, i32) {
    let window_width = window_width.max(1);
    let minimum = 120.min(window_width / 2);
    let divider = ((window_width as f64 * ratio) as i32).clamp(minimum, window_width - minimum);

    if pane == 0 {
        (0, divider.max(1))
    } else {
        (
            divider,
            window_width
                .saturating_sub(divider)
                .max(1),
        )
    }
}

#[cfg(test)]
mod cursor_hit_tests {
    use super::{
        CursorHitTestLayout,
        Renderer,
        fixed_split_pane_bounds,
    };
    use crate::piece_table::PieceTable;

    fn table_with_text(
        text: &str,
    ) -> PieceTable {
        let mut table =
            PieceTable::empty().unwrap();

        table.insert(0, text).unwrap();

        table
    }

    fn layout() -> CursorHitTestLayout {
        CursorHitTestLayout {
            text_left: 100,
            text_top: 44,
            right: 500,
            bottom: 300,
            line_height: 20,
            visible_lines: 12,
            scroll_line: 0,
            scroll_x: 0,
            char_width: 10,
            tab_width: 4,
        }
    }

    #[test]
    fn fixed_split_assigns_every_horizontal_pixel() {
        assert_eq!(
            fixed_split_pane_bounds(801, 0),
            (0, 400),
        );
        assert_eq!(
            fixed_split_pane_bounds(801, 1),
            (400, 401),
        );
    }

    #[test]
    fn ascii_clicks_snap_to_nearest_caret() {
        let column_at = |x| {
            Renderer::logical_column_for_document_x(
                "abc",
                x,
                10,
                4,
            )
        };

        assert_eq!(column_at(-20), 0);
        assert_eq!(column_at(0), 0);
        assert_eq!(column_at(4), 0);
        assert_eq!(column_at(5), 1);
        assert_eq!(column_at(14), 1);
        assert_eq!(column_at(15), 2);
        assert_eq!(column_at(25), 3);
        assert_eq!(column_at(200), 3);
    }

    #[test]
    fn odd_width_characters_snap_at_the_true_midpoint() {
        assert_eq!(
            Renderer::logical_column_for_document_x(
                "a",
                5,
                11,
                4,
            ),
            0,
        );

        assert_eq!(
            Renderer::logical_column_for_document_x(
                "a",
                6,
                11,
                4,
            ),
            1,
        );
    }

    #[test]
    fn tab_clicks_snap_to_reachable_carets() {
        let column_at = |x| {
            Renderer::logical_column_for_document_x(
                "a\tb",
                x,
                10,
                4,
            )
        };

        assert_eq!(column_at(9), 1);
        assert_eq!(column_at(10), 1);
        assert_eq!(column_at(24), 1);
        assert_eq!(column_at(25), 2);
        assert_eq!(column_at(39), 2);
        assert_eq!(column_at(40), 2);
        assert_eq!(column_at(45), 3);
    }

    #[test]
    fn tab_clicks_honor_non_default_width() {
        assert_eq!(
            Renderer::logical_column_for_document_x(
                "a\tb",
                44,
                10,
                8,
            ),
            1,
        );

        assert_eq!(
            Renderer::logical_column_for_document_x(
                "a\tb",
                45,
                10,
                8,
            ),
            2,
        );
    }

    #[test]
    fn split_text_chunks_keep_line_relative_tab_stops() {
        let whole =
            Renderer::visual_column_after_text(
                "let\tx",
                0,
                4,
            );

        let after_first_chunk =
            Renderer::visual_column_after_text(
                "let",
                0,
                4,
            );

        let after_second_chunk =
            Renderer::visual_column_after_text(
                "\tx",
                after_first_chunk,
                4,
            );

        assert_eq!(whole, 5);
        assert_eq!(after_second_chunk, whole);
    }

    #[test]
    fn unicode_clicks_use_character_columns() {
        let column_at = |x| {
            Renderer::logical_column_for_document_x(
                "aé🙂b",
                x,
                10,
                4,
            )
        };

        assert_eq!(column_at(5), 1);
        assert_eq!(column_at(15), 2);
        assert_eq!(column_at(25), 3);
        assert_eq!(column_at(35), 4);
        assert_eq!(column_at(100), 4);
    }

    #[test]
    fn vertical_clicks_only_map_to_complete_visible_rows() {
        assert_eq!(
            Renderer::visible_row_for_y(
                43,
                44,
                20,
                3,
            ),
            None,
        );

        assert_eq!(
            Renderer::visible_row_for_y(
                44,
                44,
                20,
                3,
            ),
            Some(0),
        );

        assert_eq!(
            Renderer::visible_row_for_y(
                63,
                44,
                20,
                3,
            ),
            Some(0),
        );

        assert_eq!(
            Renderer::visible_row_for_y(
                64,
                44,
                20,
                3,
            ),
            Some(1),
        );

        assert_eq!(
            Renderer::visible_row_for_y(
                103,
                44,
                20,
                3,
            ),
            Some(2),
        );

        assert_eq!(
            Renderer::visible_row_for_y(
                104,
                44,
                20,
                3,
            ),
            None,
        );
    }

    #[test]
    fn hit_test_applies_horizontal_and_vertical_scroll() {
        let mut table =
            table_with_text(
                "zero\none\ntwo\nthree",
            );

        let mut scrolled = layout();
        scrolled.scroll_line = 1;
        scrolled.scroll_x = 20;

        let target =
            Renderer::cursor_target_for_layout(
                &mut table,
                scrolled,
                100,
                64,
            )
            .unwrap();

        assert_eq!(target, Some((2, 2)));
    }

    #[test]
    fn hit_test_rejects_gutter_title_and_outside_edges() {
        let mut table =
            table_with_text("abc");

        assert_eq!(
            Renderer::cursor_target_for_layout(
                &mut table,
                layout(),
                99,
                44,
            )
            .unwrap(),
            None,
        );

        assert_eq!(
            Renderer::cursor_target_for_layout(
                &mut table,
                layout(),
                100,
                43,
            )
            .unwrap(),
            None,
        );

        assert_eq!(
            Renderer::cursor_target_for_layout(
                &mut table,
                layout(),
                500,
                44,
            )
            .unwrap(),
            None,
        );

        assert_eq!(
            Renderer::cursor_target_for_layout(
                &mut table,
                layout(),
                100,
                300,
            )
            .unwrap(),
            None,
        );
    }

    #[test]
    fn hit_test_clamps_rows_below_document_to_eof() {
        let cases = [
            ("abc\ndef", (1, 3)),
            ("abc\n", (1, 0)),
            ("", (0, 0)),
        ];

        for (text, expected) in cases {
            let mut table =
                table_with_text(text);

            let target =
                Renderer::cursor_target_for_layout(
                    &mut table,
                    layout(),
                    100,
                    144,
                )
                .unwrap();

            assert_eq!(target, Some(expected));
        }
    }

    #[test]
    fn unicode_hit_target_becomes_a_valid_byte_position() {
        let mut table =
            table_with_text("aé中🙂z");

        let (line, column) =
            Renderer::cursor_target_for_layout(
                &mut table,
                layout(),
                130,
                44,
            )
            .unwrap()
            .unwrap();

        table
            .move_cursor_to_line_column(
                line,
                column,
            )
            .unwrap();

        assert_eq!(table.cursor.position, 6);
        assert_eq!(table.cursor.column, 3);
    }

    #[test]
    fn click_past_non_final_line_stops_before_newline() {
        let mut table =
            table_with_text("abc\ndef");

        let (line, column) =
            Renderer::cursor_target_for_layout(
                &mut table,
                layout(),
                499,
                44,
            )
            .unwrap()
            .unwrap();

        table
            .move_cursor_to_line_column(
                line,
                column,
            )
            .unwrap();

        assert_eq!(table.cursor.position, 3);
        assert_eq!(table.cursor.line, 0);
        assert_eq!(table.cursor.column, 3);
    }

    #[test]
    fn click_on_empty_middle_line_stays_on_that_line() {
        let mut table =
            table_with_text("a\n\nb");

        let target =
            Renderer::cursor_target_for_layout(
                &mut table,
                layout(),
                100,
                64,
            )
            .unwrap();

        assert_eq!(target, Some((1, 0)));
    }

    #[test]
    fn search_scroll_keeps_end_caret_inside_the_field() {
        let scroll =
            Renderer::search_query_scroll_x(
                200,
                200,
                100,
                4,
            );

        assert_eq!(scroll, 104);
        assert_eq!(200 - scroll, 96);
    }

    #[test]
    fn short_search_query_does_not_scroll() {
        assert_eq!(
            Renderer::search_query_scroll_x(
                40,
                40,
                100,
                4,
            ),
            0,
        );
    }

    #[test]
    fn collapsed_search_field_still_keeps_the_caret_in_bounds() {
        let scroll =
            Renderer::search_query_scroll_x(
                200,
                200,
                1,
                4,
            );

        assert_eq!(200 - scroll, 0);
    }

    #[test]
    fn search_bar_shows_mode_and_status_when_both_fit() {
        assert_eq!(
            Renderer::search_bar_visibility(
                150,
                20,
                40,
                20,
                8,
                true,
            ),
            (true, true),
        );
    }

    #[test]
    fn narrow_search_bar_prioritizes_status() {
        assert_eq!(
            Renderer::search_bar_visibility(
                60,
                20,
                40,
                20,
                8,
                true,
            ),
            (false, true),
        );
    }

    #[test]
    fn extremely_narrow_search_bar_hides_trailing_labels() {
        assert_eq!(
            Renderer::search_bar_visibility(
                30,
                20,
                40,
                20,
                8,
                true,
            ),
            (false, false),
        );
    }
}

#[cfg(test)]
mod terminal_selection_render_tests {
    use super::*;

    #[test]
    #[ignore = "Pixel regression; run with SDL_VIDEODRIVER=dummy in a separate process"]
    fn title_bar_stays_fixed_when_editor_font_changes() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("title zoom", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let textures = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &textures, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        renderer.set_mode_label(Some("VIM INSERT"));
        let pixels = |renderer: &Renderer<'_>| {
            renderer.canvas.read_pixels(Rect::new(0, 0, 800, TITLE_BAR_HEIGHT as u32)).unwrap()
                .convert_format(sdl3::pixels::PixelFormat::RGBA32).unwrap()
                .with_lock(|bytes| bytes.to_vec())
        };
        renderer.render_title_bar().unwrap();
        let expected = pixels(&renderer);
        let initial_editor_height = renderer.font.height();
        for size in [8.0, 48.0, 24.0, 18.0] {
            renderer.set_font_size(size).unwrap();
            let editor_height = renderer.font.height();
            renderer.render_title_bar().unwrap();
            assert_eq!(pixels(&renderer), expected, "title bar changed at editor size {size}");
            assert_eq!(renderer.font.height(), editor_height, "editor font must be restored");
            if size != 18.0 { assert_ne!(editor_height, initial_editor_height); }
        }
    }

    #[test]
    #[ignore = "Pixel regression; run with SDL_VIDEODRIVER=dummy in a separate process"]
    fn split_document_uses_the_full_pane_width_beside_terminal() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("split document width", 1001, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(1001, 1.0)).unwrap();
        renderer.set_file_path(Some(std::path::Path::new("example.rs")));
        let mut document = PieceTable::empty().unwrap();
        document.insert(0, &format!("// {}\nfn main() {{\n\tprintln!(\"{}\");\n}}\n",
            "long comment with Unicode café 東京 ".repeat(10), "visible code ".repeat(10)).repeat(40)).unwrap();
        document.move_cursor_to_line_column(0, 0).unwrap();
        renderer.update_cursor(&document);
        let mut other = PieceTable::empty().unwrap();
        let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        terminal.output_mut().insert(0, "project listing\n").unwrap();
        let search = SearchUi::new();
        let bar = CommandBar::new();
        let pixels = |renderer: &Renderer<'_>, left, width| {
            renderer.canvas.read_pixels(Rect::new(left + 2, TITLE_BAR_HEIGHT + 3,
                (width - 4) as u32, (600 - TITLE_BAR_HEIGHT - 3) as u32)).unwrap()
                .convert_format(sdl3::pixels::PixelFormat::RGBA32).unwrap()
                .with_lock(|bytes| bytes.to_vec())
        };
        for window_width in [800, 1001] {
            for pane in [0, 1] {
                let (left, width) = fixed_split_pane_bounds(window_width, pane);
                for (scroll, scroll_line) in [(0, 0), (93, 0), (0, 50), (93, 50)] {
                    // A normal editor at the same width is the pixel reference.
                    renderer.set_split_mode(false);
                    renderer.window_width = width;
                    renderer.scroll_x = scroll;
                    renderer.set_scroll_line(&mut document, scroll_line);
                    renderer.render(&mut document, &search, &bar).unwrap();
                    let expected = pixels(&renderer, 0, width);
                    let expected_cursor = renderer.cursor_target_at(&mut document,
                        width - 25, TITLE_BAR_HEIGHT + 10).unwrap();
                    renderer.window_width = window_width;
                    renderer.set_split_mode(true);
                    renderer.set_active_pane(1 - pane);
                    renderer.place_terminal_in_active_pane();
                    renderer.set_active_pane(pane);
                    renderer.render_split_terminal(&mut document, &mut other, &mut terminal,
                        &search, &bar).unwrap();
                    assert!(pixels(&renderer, left, width) == expected,
                        "document pane {pane} at window width {window_width}, scroll {scroll} must use its full width");
                    assert_eq!(renderer.cursor_target_at(&mut document,
                        left + width - 25, TITLE_BAR_HEIGHT + 10).unwrap(), expected_cursor);
                    // The ordinary two-document split shares this rendering path.
                    renderer.render_split(&mut document, &mut other, &search, &bar).unwrap();
                    assert!(pixels(&renderer, left, width) == expected);
                }
            }
        }
    }

    #[test]
    #[ignore = "SDL split terminal regression; run with SDL_VIDEODRIVER=dummy in a separate process"]
    fn split_terminal_layout_and_targets_follow_either_pane() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video
            .window("split terminal", 1001, 600)
            .hidden()
            .build()
            .unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || {
            ttf.load_font_from_iostream(
                sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(),
                18.0,
            )
            .unwrap()
        };
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(
            canvas,
            &texture_creator,
            font(),
            font(),
            18.0,
            (font(), font()),
            crate::window::WindowHitTestState::new(1001, 1.0),
        )
        .unwrap();
        let mut active = PieceTable::empty().unwrap();
        let mut other = PieceTable::empty().unwrap();
        other
            .insert(0, &"The other document remains visible.\n".repeat(40))
            .unwrap();
        let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        terminal
            .output_mut()
            .insert(0, &"aé 東京 words ".repeat(30))
            .unwrap();
        terminal.insert_text("echo hello");
        let search = SearchUi::new();
        let mut bar = CommandBar::new();
        renderer.set_split_mode(true);
        for width in [1000, 1001, 800] {
            renderer.window_width = width;
            for pane in [0, 1] {
                renderer.set_active_pane(pane);
                renderer.place_terminal_in_active_pane();
                renderer
                    .render_split(&mut active, &mut other, &search, &bar)
                    .unwrap();
                let (other_left, other_width) = renderer.pane_bounds(1 - pane);
                let clip = Rect::new(
                    other_left + 2,
                    TITLE_BAR_HEIGHT + 3,
                    (other_width - 4) as u32,
                    (600 - TITLE_BAR_HEIGHT - 3) as u32,
                );
                let pixels = |renderer: &Renderer<'_>| {
                    renderer
                        .canvas
                        .read_pixels(clip)
                        .unwrap()
                        .convert_format(sdl3::pixels::PixelFormat::RGBA32)
                        .unwrap()
                        .with_lock(|bytes| bytes.to_vec())
                };
                let expected = pixels(&renderer);
                renderer
                    .render_split_terminal(&mut active, &mut other, &mut terminal, &search, &bar)
                    .unwrap();
                assert_eq!(
                    pixels(&renderer),
                    expected,
                    "terminal must not change the adjacent document"
                );
                assert_eq!(renderer.bottom_inset, 0);
                let (left, pane_width) = renderer.pane_bounds(pane);
                assert_eq!(
                    renderer.terminal_columns(),
                    ((pane_width - 24) / renderer.char_width) as usize
                );
                let rows = renderer.terminal_layout.len();
                let x = left + 12 + renderer.terminal_text_width("aé");
                assert_eq!(
                    renderer
                        .terminal_output_offset_at(&mut terminal, x, TITLE_BAR_HEIGHT + 8)
                        .unwrap(),
                    3
                );
                renderer
                    .navigate_terminal_output(&mut terminal, OutputCommand::None)
                    .unwrap();
                assert_eq!(
                    renderer.terminal_layout.len(),
                    rows,
                    "input must preserve pane-width wrapping"
                );
                assert_eq!(
                    renderer.terminal_hit_at(other_left + 20, TITLE_BAR_HEIGHT + 10),
                    TerminalHit::Outside
                );
                let y = 600 - TERMINAL_BAR_MARGIN - TERMINAL_BAR_HEIGHT + 4;
                assert_eq!(renderer.terminal_hit_at(left + 20, y), TerminalHit::Input);
                assert_eq!(
                    renderer.terminal_hit_at(left + pane_width - TERMINAL_BAR_MARGIN - 4, y),
                    TerminalHit::Editor
                );
                assert_eq!(
                    renderer.terminal_hit_at(
                        left + pane_width - TERMINAL_BAR_MARGIN - TERMINAL_EDITOR_WIDTH - 4,
                        y
                    ),
                    TerminalHit::Clear
                );
                assert_eq!(
                    renderer.terminal_hit_at(
                        left + pane_width
                            - TERMINAL_BAR_MARGIN
                            - TERMINAL_EDITOR_WIDTH
                            - TERMINAL_CLEAR_WIDTH
                            - 4,
                        y
                    ),
                    TerminalHit::StopOrRunAgain
                );
                assert_eq!(
                    renderer.terminal_cursor_at(&terminal, left + TERMINAL_BAR_MARGIN + 10),
                    0
                );
                assert!(
                    renderer.terminal_cursor_at(&terminal, left + TERMINAL_BAR_MARGIN + 35) > 0
                );
                let misses = renderer.terminal_text_cache.misses;
                renderer
                    .render_split_terminal(&mut active, &mut other, &mut terminal, &search, &bar)
                    .unwrap();
                assert_eq!(renderer.terminal_text_cache.misses, misses);
            }
        }
        bar.open(":find document");
        renderer
            .render_split_terminal(&mut active, &mut other, &mut terminal, &search, &bar)
            .unwrap();
        assert_eq!(renderer.bottom_inset, bar.reserved_height());
        let (left, width) = renderer.terminal_bounds();
        let button_x = left + width - TERMINAL_BAR_MARGIN - 4;
        let button_y = 600 - bar.reserved_height() - TERMINAL_BAR_MARGIN - TERMINAL_BAR_HEIGHT + 4;
        assert_eq!(
            renderer.terminal_hit_at(button_x, button_y),
            TerminalHit::Editor
        );
        assert_eq!(
            renderer.terminal_hit_at(button_x, 599),
            TerminalHit::Outside
        );
    }

    #[test]
    #[ignore = "SDL rendering regression; run with SDL_VIDEODRIVER=dummy --ignored --test-threads=1"]
    fn unity_csharp_bom_and_windows_line_endings_render_without_error() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("C# encoding regression", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        renderer.set_file_path(Some(std::path::Path::new("Assets/Scripts/Player.cs")));
        for prefix in ["", "\u{feff}"] {
            let text = format!("{prefix}using UnityEngine;\r\n\r\npublic class Player : MonoBehaviour\r\n{{\r\n    // café\r\n}}\r\n");
            let mut table = PieceTable::empty().unwrap();
            table.insert(0, &text).unwrap();
            renderer.invalidate_scroll_cache();
            renderer.render(&mut table, &SearchUi::new(), &CommandBar::new())
                .unwrap_or_else(|error| panic!("C# prefix {prefix:?}: {error}"));
            assert_eq!(table.text().unwrap(), text, "rendering must preserve the file bytes");
        }
    }

    #[test]
    #[ignore = "Pixel regression; SDL_VIDEODRIVER=dummy, --ignored --test-threads=1"]
    fn editor_viewport_preserves_scrolled_unicode_tabs_and_clicks() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("viewport pixels", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        let text = "abé\tcd λ\txyz ".repeat(100);
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, &text).unwrap();
        table.ensure_line_cached(0).unwrap();
        let left = renderer.text_left(&table).unwrap();
        let clip = Rect::new(left, TITLE_BAR_HEIGHT, (800 - left) as u32, 60);
        let pixels = |renderer: &Renderer<'_>| {
            let surface = renderer.canvas.read_pixels(clip).unwrap()
                .convert_format(sdl3::pixels::PixelFormat::RGBA32).unwrap();
            surface.with_lock(|bytes| bytes.to_vec())
        };
        for scroll in [0, 7, 93, 500, 1700] {
            renderer.scroll_x = scroll;
            renderer.canvas.set_clip_rect(None);
            renderer.canvas.set_draw_color(Color::RGB(30, 30, 30));
            renderer.canvas.clear();
            renderer.canvas.set_clip_rect(Some(clip));
            renderer.render_text_chunk(&text, left - scroll, 0, TITLE_BAR_HEIGHT + 8,
                Color::RGB(220, 220, 220)).unwrap();
            let expected = pixels(&renderer);
            renderer.canvas.set_clip_rect(None);
            renderer.canvas.set_draw_color(Color::RGB(30, 30, 30));
            renderer.canvas.clear();
            renderer.canvas.set_clip_rect(Some(clip));
            renderer.render_text(&mut table).unwrap();
            assert_eq!(pixels(&renderer), expected, "scroll {scroll}");
            let layout = CursorHitTestLayout {
                text_left: left, text_top: TITLE_BAR_HEIGHT + 8, right: 800, bottom: 600,
                line_height: renderer.font.height(), visible_lines: 10, scroll_line: 0,
                scroll_x: scroll, char_width: renderer.char_width, tab_width: renderer.tab_width,
            };
            for x in (left..800).step_by(7) {
                let expected = Renderer::logical_column_for_document_x(&text, x - left + scroll,
                    renderer.char_width, renderer.tab_width);
                let actual = Renderer::cursor_target_for_layout(&mut table, layout, x, TITLE_BAR_HEIGHT + 9).unwrap();
                assert_eq!(actual, Some((0, expected)));
            }
        }
    }

    #[test]
    #[ignore = "Long-line benchmark; SDL_VIDEODRIVER=dummy, --ignored --test-threads=1"]
    fn long_line_navigation_probe() {
        use std::time::Instant;
        let path = std::env::temp_dir().join(format!("potyi-long-lines-{}.txt", std::process::id()));
        let mut content = "x".repeat(1024 * 1024);
        content.push('\n');
        std::fs::write(&path, content.repeat(3)).unwrap();
        let mut table = PieceTable::open(path.to_str().unwrap()).unwrap();
        let start = Instant::now();
        table.ensure_line_cached(3).unwrap();
        eprintln!("LONG_LINE initial index: {:?}", start.elapsed());
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("long line probe", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        let start = Instant::now();
        renderer.render_text(&mut table).unwrap();
        eprintln!("LONG_LINE first text frame: {:?}", start.elapsed());
        let start = Instant::now();
        for _ in 0..5 {
            table.cursor_down().unwrap();
            renderer.ensure_cursor_visible(&mut table);
            renderer.render_text(&mut table).unwrap();
            table.cursor_up().unwrap();
            renderer.ensure_cursor_visible(&mut table);
            renderer.render_text(&mut table).unwrap();
        }
        assert_eq!(table.cursor.line, 0);
        eprintln!("LONG_LINE 10 moves and text frames: {:?}", start.elapsed());
        drop(table);
        std::fs::remove_file(path).unwrap();
    }


    #[test]
    #[ignore = "Run with SDL_VIDEODRIVER=dummy and --ignored --test-threads=1"]
    fn compact_hover_panel_renders_with_small_paragraph_gaps() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("hover spacing", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        let mut bar = CommandBar::new();
        bar.open(":hover");
        renderer.set_file_path(Some(std::path::Path::new("example.rs")));
        bar.show_hover(&crate::lsp::hover_content(&serde_json::json!({"contents": {
            "kind": "plaintext",
            "value": "potyi_lsp_demo\n\nfn greeting() -> &'static str\n\n\nReturns a greeting to test Pötyi hover information."
        }})));
        assert_eq!(bar.panel_height(), 108);
        assert!(renderer.command_font.height() <= bar.suggestion_row_height(0));
        renderer.canvas.set_draw_color(Color::RGB(30, 30, 30));
        renderer.canvas.clear();
        renderer.render_command_bar(&bar, &SearchUi::new()).unwrap();
        let top = 600 - bar.reserved_height();
        let has_color = |renderer: &Renderer<'_>, index, color: [u8; 3]| {
            let surface = renderer.canvas.read_pixels(Rect::new(20,
                top + bar.suggestion_row_offset(index), 760, bar.suggestion_row_height(index) as u32))
                .unwrap().convert_format(sdl3::pixels::PixelFormat::RGBA32).unwrap();
            surface.with_lock(|pixels| pixels.chunks_exact(4).any(|pixel| pixel[..3] == color))
        };
        assert!(has_color(&renderer, 4, [106, 153, 85]), "documentation should use Rust comment green");
        assert!(has_color(&renderer, 2, [235, 235, 235]), "signature should keep its normal color");
        if let Some(path) = std::env::var_os("POTYI_HOVER_PREVIEW") {
            renderer.canvas.read_pixels(Rect::new(0, 600 - bar.reserved_height() - 8, 800,
                (bar.reserved_height() + 8) as u32)).unwrap().save_bmp(path).unwrap();
        }
        for rule in &mut renderer.syntax.as_mut().unwrap().rules {
            if rule.name == "comment" { rule.color = Color::RGB(180, 100, 220); }
        }
        renderer.render_command_bar(&bar, &SearchUi::new()).unwrap();
        assert!(has_color(&renderer, 4, [180, 100, 220]), "custom comment colors must apply to hover documentation");
    }

    #[test]
    #[ignore = "Run with SDL_VIDEODRIVER=dummy and --ignored --test-threads=1"]
    fn occurrence_selections_carets_and_vim_transition_render_correctly() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("occurrence rendering", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        let mut config = crate::config::EditorConfig::default();
        config.keybinding_mode = crate::config::KeybindingMode::Conventional;
        let mut editor = crate::Editor::new(config.clone()).unwrap();
        editor.document.insert(0, "cat\ncat").unwrap();
        editor.document.move_cursor(0).unwrap();
        editor.select_next_occurrence().unwrap();
        editor.select_next_occurrence().unwrap();
        renderer.update_cursor(&editor.document);
        renderer.canvas.set_draw_color(Color::RGB(0, 0, 0));
        renderer.canvas.clear();
        renderer.render_selection(&mut editor.document).unwrap();
        let left = renderer.text_left(&mut editor.document).unwrap();
        let pixel = |renderer: &Renderer<'_>, x, y| {
            let surface = renderer.canvas.read_pixels(Rect::new(x, y, 1, 1)).unwrap()
                .convert_format(sdl3::pixels::PixelFormat::RGBA32).unwrap();
            surface.with_lock(|bytes| [bytes[0], bytes[1], bytes[2]])
        };
        for line in 0..2 {
            assert_eq!(pixel(&renderer, left + 1, TITLE_BAR_HEIGHT + 9 + line * renderer.font.height()),
                [70, 100, 160], "both selections must be highlighted");
        }
        editor.insert("dog").unwrap();
        renderer.update_cursor(&editor.document);
        renderer.canvas.set_draw_color(Color::RGB(0, 0, 0));
        renderer.canvas.clear();
        renderer.render_cursor(&mut editor.document).unwrap();
        let x = renderer.visual_x_for_column("dog", 3, left);
        for line in 0..2 {
            assert_eq!(pixel(&renderer, x, TITLE_BAR_HEIGHT + 9 + line * renderer.font.height()),
                [255, 255, 255], "both insertion carets must be drawn");
        }
        let mut other = crate::Editor::new(config).unwrap();
        let mut vim = crate::VimController::new();
        let mut other_vim = crate::VimController::new();
        let mut enabled = false;
        crate::apply_keybinding_mode(crate::config::KeybindingMode::Vim, &mut enabled,
            &mut editor, &mut other, &mut vim, &mut other_vim, &mut renderer);
        assert!(enabled);
        assert!(editor.document.secondary_cursors.is_empty());
        assert!(!editor.document.has_selection());
        assert_eq!(editor.document.text().unwrap(), "dog\ndog");
        editor.config.keybinding_mode = crate::config::KeybindingMode::Vim;
        editor.undo().unwrap();
        assert_eq!(editor.document.text().unwrap(), "cat\ncat");
        assert!(editor.document.secondary_cursors.is_empty());
        assert!(!editor.document.has_selection());
    }

    #[test]
    #[cfg(not(windows))]
    #[ignore = "Latency probe; run with SDL_VIDEODRIVER=dummy --ignored --nocapture --test-threads=1"]
    fn terminal_command_latency_probe() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let events = sdl.event().unwrap();
        crate::terminal::register_test_events(&events);
        let mut pump = sdl.event_pump().unwrap();
        let window = video.window("terminal latency", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        let mut terminal = Terminal::new(std::env::current_dir().unwrap()).unwrap();
        terminal.set_events(events.clone());
        for command in ["printf terminal-ready", "grep -n 'fn main' src/main.rs", "printf 'ready\\n' | grep -n ready"] {
            for _ in pump.poll_iter() {}
            terminal.clear().unwrap();
            terminal.insert_text(command);
            let start = std::time::Instant::now();
            terminal.submit(&events).unwrap();
            let initial_length = terminal.output_mut().len();
            renderer.render_terminal(&mut terminal).unwrap();
            let mut first_output = None;
            while terminal.is_running() {
                let changed = terminal.poll_background().unwrap();
                if changed {
                    renderer.render_terminal(&mut terminal).unwrap();
                    if terminal.output_mut().len() > initial_length && first_output.is_none() {
                        first_output = Some(start.elapsed());
                    }
                }
                assert!(start.elapsed() < std::time::Duration::from_secs(20), "command timed out");
                if !terminal.is_running() { break; }
                let timeout = if terminal.has_pending_work() { std::time::Duration::ZERO } else { std::time::Duration::from_secs(1) };
                if let Some(event) = pump.wait_event_timeout(timeout)
                    && let Some(event) = event.as_user_event_type::<crate::terminal::TerminalEvent>()
                { terminal.handle_event(event).unwrap(); }
            }
            eprintln!("LATENCY {command:?}: first drawn output {:?}, complete {:?}", first_output.unwrap(), start.elapsed());
            let name = if command.starts_with("grep") { "terminal_grep_first_frame" }
                else if command.contains('|') { "terminal_pipe_first_frame" } else { "terminal_printf_first_frame" };
            crate::benchmarks::record(name, first_output.unwrap(), 0);
        }
    }

    #[test]
    #[ignore = "Performance probe; run with SDL_VIDEODRIVER=dummy --ignored --nocapture --test-threads=1"]
    fn terminal_performance_probe() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("terminal performance", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        let text = (0..100).map(|n| format!("{n}: ordinary terminal output with Unicode 東京 and words\n")).collect::<String>();
        terminal.output_mut().insert(0, &text).unwrap();
        renderer.render_terminal(&mut terminal).unwrap();
        let initial_misses = renderer.terminal_text_cache.misses;
        let start = std::time::Instant::now();
        for _ in 0..120 { renderer.render_terminal(&mut terminal).unwrap(); }
        let elapsed = start.elapsed();
        eprintln!("PERF 120 unchanged frames: {elapsed:?}");
        crate::benchmarks::record("terminal_120_cached_frames", elapsed, 0);
        assert_eq!(renderer.terminal_text_cache.misses, initial_misses, "unchanged frames must reuse glyph textures");
        assert!(renderer.terminal_text_cache.bytes() <= crate::terminal_text_cache::MAX_BYTES);
        terminal.clear().unwrap();
        let chunk = "long unfinished output ".repeat(700);
        let start = std::time::Instant::now();
        for _ in 0..128 {
            let end = terminal.output_mut().len();
            terminal.output_mut().insert(end, &chunk).unwrap();
            renderer.update_terminal_layout(&mut terminal).unwrap();
        }
        let elapsed = start.elapsed();
        eprintln!("PERF unfinished-line append/layout {} bytes: {elapsed:?}", chunk.len() * 128);
        crate::benchmarks::record("terminal_append_layout_2mb", elapsed, chunk.len() * 128);
        renderer.set_font_size(20.0).unwrap();
        assert_eq!(renderer.terminal_text_cache.bytes(), 0, "font changes invalidate cached glyphs");
        // Deliberately exceed the cache budget, without a timing assertion.
        for n in 0..4 {
            let text = format!("large cached row {n}");
            let color = Color::RGB(255, 255, 255);
            let key = TextCache::key(&text, color);
            let texture = texture_creator.create_texture_static(None, 1024, 1024).unwrap();
            renderer.terminal_text_cache.insert(key, &text, color, texture);
            assert!(renderer.terminal_text_cache.bytes() <= crate::terminal_text_cache::MAX_BYTES);
            assert!(renderer.terminal_text_cache.get(key, &text, color).is_some());
        }
        assert!(renderer.terminal_text_cache.get(TextCache::key("large cached row 0", Color::RGB(255,255,255)),
            "large cached row 0", Color::RGB(255,255,255)).is_none(), "old textures must be evicted");
        renderer.render(terminal.output_mut(), &SearchUi::new(), &CommandBar::new()).unwrap();
        assert_eq!(renderer.terminal_text_cache.bytes(), 0, "returning to the editor releases cached textures");

    }

    #[test]
    #[ignore = "Run with SDL_VIDEODRIVER=dummy and --ignored --test-threads=1"]
    fn terminal_selection_handles_wrapping_unicode_and_streaming() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("terminal selection test", 800, 600).hidden().build().unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(), 18.0,
        ).unwrap();
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(canvas, &texture_creator, font(), font(), 18.0,
            (font(), font()), crate::window::WindowHitTestState::new(800, 1.0)).unwrap();
        let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        let text = format!("aé🙂\n\n{}\nend", "wrapped words ".repeat(200));
        terminal.output_mut().insert(0, &text).unwrap();
        renderer.render_terminal(&mut terminal).unwrap();
        terminal.move_output_cursor(0, false).unwrap();
        renderer.navigate_terminal_output(&mut terminal, OutputCommand::None).unwrap();
        renderer.render_terminal(&mut terminal).unwrap();
        let x = 12 + renderer.terminal_text_width("aé");
        assert_eq!(renderer.terminal_output_offset_at(&mut terminal, x, TITLE_BAR_HEIGHT + 8).unwrap(), 3);
        terminal.move_output_cursor(3, false).unwrap();
        renderer.navigate_terminal_output(&mut terminal, OutputCommand::Rows(1, true)).unwrap();
        assert_eq!(terminal.output_cursor(), 8); // empty logical line
        assert_eq!(terminal.output_selection(), 3..8);
        let desired_x = terminal.output_desired_x();
        renderer.navigate_terminal_output(&mut terminal, OutputCommand::Rows(1, true)).unwrap();
        assert_eq!(terminal.output_desired_x(), desired_x);
        let row = renderer.terminal_layout.row_at(terminal.output_cursor());
        renderer.navigate_terminal_output(&mut terminal, OutputCommand::Rows(1, true)).unwrap();
        assert_eq!(renderer.terminal_layout.row_at(terminal.output_cursor()), row + 1);
        let selection = terminal.output_selection();
        renderer.render_terminal(&mut terminal).unwrap();
        let visible_start = renderer.terminal_layout.visible_rows(renderer.visible_line_count(), terminal.scroll_back()).start;
        let end = terminal.output_mut().len();
        terminal.output_mut().insert(end, &"streaming\n".repeat(100)).unwrap();
        renderer.render_terminal(&mut terminal).unwrap();
        assert_eq!(terminal.output_selection(), selection);
        assert_eq!(renderer.terminal_layout.visible_rows(renderer.visible_line_count(), terminal.scroll_back()).start, visible_start);
        terminal.clear().unwrap();
        terminal.move_output_cursor(0, false).unwrap();
        renderer.render_terminal(&mut terminal).unwrap();
    }
    #[test]
    #[ignore = "requires SDL and a Git checkout; run with SDL_VIDEODRIVER=dummy and --test-threads=1"]
    fn git_commit_mouse_target_and_back_restore_terminal_layout() {
        let sdl = sdl3::init().unwrap(); let events=sdl.event().unwrap();
        crate::terminal::register_test_events(&events); let mut pump=sdl.event_pump().unwrap();
        let video=sdl.video().unwrap(); let window=video.window("Git navigation",800,600).hidden().build().unwrap();
        let ttf=sdl3::ttf::init().unwrap();
        let font=|| ttf.load_font_from_iostream(sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(),18.0).unwrap();
        let canvas=window.into_canvas(); let texture_creator=canvas.texture_creator();
        let mut renderer=Renderer::new(canvas,&texture_creator,font(),font(),18.0,(font(),font()),crate::window::WindowHitTestState::new(800,1.0)).unwrap();
        let mut terminal=Terminal::new(std::env::current_dir().unwrap()).unwrap(); terminal.set_events(events.clone()); terminal.clear().unwrap();
        let drain=|terminal: &mut Terminal,pump: &mut sdl3::EventPump| {
            let start=std::time::Instant::now();
            while terminal.is_running() {
                terminal.poll_background().unwrap(); assert!(start.elapsed()<std::time::Duration::from_secs(15));
                if let Some(event)=pump.wait_event_timeout(std::time::Duration::from_millis(20))
                    && let Some(event)=event.as_user_event_type::<crate::terminal::TerminalEvent>() {terminal.handle_event(event).unwrap();}
            }
        };
        terminal.insert_text("git log -2 --oneline"); terminal.submit(&events).unwrap(); drain(&mut terminal,&mut pump);
        renderer.render_terminal(&mut terminal).unwrap();
        let original=terminal.output_text().unwrap();
        let entry=terminal.entries_in(0..original.len()).iter().find(|e| e.kind==EntryKind::Commit).unwrap().clone();
        let row=renderer.terminal_layout.row_at(entry.range.start);
        let rows=renderer.terminal_layout.visible_rows(renderer.visible_terminal_line_count(),terminal.scroll_back());
        let y=TITLE_BAR_HEIGHT+8+(row-rows.start) as i32*renderer.terminal_line_height()+2;
        let x=16;
        let action=renderer.terminal_action_at(&mut terminal,x,y).unwrap();
        assert_eq!(action,entry.action());
        let mut active = PieceTable::empty().unwrap();
        let mut other = PieceTable::empty().unwrap();
        for pane in [0, 1] {
            renderer.set_split_mode(true);
            renderer.set_active_pane(pane);
            renderer.place_terminal_in_active_pane();
            renderer.render_split_terminal(&mut active, &mut other, &mut terminal,
                &SearchUi::new(), &CommandBar::new()).unwrap();
            let row = renderer.terminal_layout.row_at(entry.range.start);
            let rows = renderer.terminal_layout.visible_rows(renderer.visible_terminal_line_count(), terminal.scroll_back());
            let y = TITLE_BAR_HEIGHT + 8 + (row - rows.start) as i32 * renderer.terminal_line_height() + 2;
            let x = renderer.pane_bounds(pane).0 + 16;
            assert_eq!(renderer.terminal_action_at(&mut terminal, x, y).unwrap(), entry.action());
        }
        renderer.set_split_mode(false);
        renderer.render_terminal(&mut terminal).unwrap();
        terminal.begin_output_drag(entry.range.start,x,y,action,false).unwrap();
        let TerminalAction::Commit(commit)=terminal.finish_output_drag().unwrap() else {panic!()};
        let scroll=terminal.scroll_back();
        terminal.open_commit(commit).unwrap(); renderer.render_terminal(&mut terminal).unwrap();
        drain(&mut terminal,&mut pump); renderer.render_terminal(&mut terminal).unwrap();
        assert!(terminal.can_go_back());
        assert_eq!(renderer.terminal_layout.visible_rows(renderer.visible_terminal_line_count(),terminal.scroll_back()).start,0,"diff should start at its header");
        if let Ok(path)=std::env::var("POTYI_GIT_NAV_SCREENSHOT") {
            if let Some(hunk)=terminal.output_text().unwrap().find("@@") {
                terminal.move_output_cursor(hunk,false).unwrap();
                renderer.navigate_terminal_output(&mut terminal,OutputCommand::None).unwrap();
                renderer.render_terminal(&mut terminal).unwrap();
            }
            renderer.canvas.read_pixels(Rect::new(0,0,800,600)).unwrap().save_bmp(path).unwrap();
        }
        let back_x=800-TERMINAL_BAR_MARGIN-TERMINAL_EDITOR_WIDTH-TERMINAL_CLEAR_WIDTH+4;
        assert_eq!(renderer.terminal_hit_at(back_x,600-TERMINAL_BAR_MARGIN-TERMINAL_BAR_HEIGHT+4),TerminalHit::Clear);
        terminal.go_back().unwrap(); renderer.render_terminal(&mut terminal).unwrap();
        assert_eq!(terminal.output_text().unwrap(),original); assert_eq!(terminal.scroll_back(),scroll);
        assert!(!terminal.can_go_back()); assert_eq!(terminal.action_at_output_offset(entry.range.start).unwrap(),entry.action());
    }

}
