// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Narrow borrows for editing, history, and text routing.
use super::*;

pub(super) struct EditContext<'a, 'font> {
    pub editor: &'a mut Editor,
    pub renderer: &'a mut Renderer<'font>,
    pub vim: &'a mut VimController,
    pub search_ui: &'a mut SearchUi,
    pub command_bar: &'a mut CommandBar,
    pub clipboard: &'a sdl3::clipboard::ClipboardUtil,
    pub dirty: &'a mut bool,
    pub vim_enabled: &'a mut bool,
}

impl<'a, 'font> From<InputContext<'a, 'font>> for EditContext<'a, 'font> {
    fn from(context: InputContext<'a, 'font>) -> Self {
        Self {
            editor: context.editor,
            renderer: context.renderer,
            vim: context.vim,
            search_ui: context.search_ui,
            command_bar: context.command_bar,
            clipboard: context.clipboard,
            dirty: context.dirty,
            vim_enabled: context.vim_enabled,
        }
    }
}

pub(super) struct HistoryContext<'a, 'font> {
    pub editor: &'a mut Editor,
    pub other_editor: &'a mut Editor,
    pub renderer: &'a mut Renderer<'font>,
    pub vim: &'a mut VimController,
    pub other_vim: &'a mut VimController,
    pub lsp_ui: &'a mut lsp_ui::LspUi,
    pub search_ui: &'a mut SearchUi,
    pub command_bar: &'a mut CommandBar,
    pub dirty: &'a mut bool,
    pub vim_enabled: &'a mut bool,
}

impl<'a, 'font> From<InputContext<'a, 'font>> for HistoryContext<'a, 'font> {
    fn from(context: InputContext<'a, 'font>) -> Self {
        Self {
            editor: context.editor,
            other_editor: context.other_editor,
            renderer: context.renderer,
            vim: context.vim,
            other_vim: context.other_vim,
            lsp_ui: context.lsp_ui,
            search_ui: context.search_ui,
            command_bar: context.command_bar,
            dirty: context.dirty,
            vim_enabled: context.vim_enabled,
        }
    }
}

pub(super) struct TextContext<'a, 'font> {
    pub pane_keys: &'a mut PaneKeys,
    pub editor: &'a mut Editor,
    pub other_editor: &'a mut Editor,
    pub renderer: &'a mut Renderer<'font>,
    pub terminal: &'a mut Terminal,
    pub vim: &'a mut VimController,
    pub emacs: &'a mut emacs::Controller,
    pub lsp_ui: &'a mut lsp_ui::LspUi,
    pub search_ui: &'a mut SearchUi,
    pub command_bar: &'a mut CommandBar,
    pub dirty: &'a mut bool,
    pub vim_enabled: &'a mut bool,
}

impl<'a, 'font> From<InputContext<'a, 'font>> for TextContext<'a, 'font> {
    fn from(context: InputContext<'a, 'font>) -> Self {
        Self {
            pane_keys: context.pane_keys,
            editor: context.editor,
            other_editor: context.other_editor,
            renderer: context.renderer,
            terminal: context.terminal,
            vim: context.vim,
            emacs: context.emacs,
            lsp_ui: context.lsp_ui,
            search_ui: context.search_ui,
            command_bar: context.command_bar,
            dirty: context.dirty,
            vim_enabled: context.vim_enabled,
        }
    }
}
