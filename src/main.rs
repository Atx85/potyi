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


#![cfg_attr(
    all(windows, not(debug_assertions)),
    windows_subsystem = "windows"
)]

static FONT_DATA: &[u8] = include_bytes!("../fonts/DejaVuSansMono.ttf");

mod app;
mod editor;
mod dpi_text;
mod clipboard;
mod command_bar;
mod embedded_config;
mod formatting;
mod lsp;
mod lsp_ui;
mod lsp_setup;
mod workspace_edit;
#[cfg(test)]
mod benchmarks;
mod keybindings;
mod line_numbers;
mod piece_table;
mod renderer;
mod search;
mod search_ui;
mod startup;
mod window;
mod syntax;
mod syntax_core;
mod config;
mod multi_cursor;
mod terminal;
mod terminal_layout;
mod terminal_text_cache;
mod vim;
mod emacs;

// Shared internal types retained at the crate root for existing feature modules.
use app::commands::CommandOutcome;
use app::navigation::parse_location;
#[cfg(test)]
use app::navigation::apply_keybinding_mode;
#[cfg(test)]
use sdl3::keyboard::{Keycode, Mod};
use editor::{CursorState, Editor, HistoryEntry, HistoryKind, TextChange, file_is_open_in};
use piece_table::PieceTableSnapshot;
use search::Searcher;
#[cfg(test)]
use piece_table::PieceTable;
#[cfg(test)]
use vim::VimController;

fn main() -> Result<(), String> {
    app::run()
}

#[cfg(test)]
mod tests;
