// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Keyboard priority lives here; each stage reports whether it consumed the key.
use super::*;
mod command_bar;
mod conventional;
mod emacs_mode;
mod history;
mod panes;
mod shortcuts;
mod terminal;
mod vim_mode;
pub(crate) use panes::PaneKeys;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyFlow {
    Pass,
    Handled,
    Quit,
}

impl KeyFlow {
    fn consumed(self) -> Option<EventFlow> {
        match self {
            Self::Pass => None,
            Self::Handled => Some(EventFlow::Continue),
            Self::Quit => Some(EventFlow::Quit),
        }
    }
}

impl From<EventFlow> for KeyFlow {
    fn from(flow: EventFlow) -> Self {
        match flow {
            EventFlow::Continue => Self::Handled,
            EventFlow::Quit => Self::Quit,
        }
    }
}

type KeyResult = Result<KeyFlow, String>;

pub(super) fn key(
    mut context: InputContext<'_, '_>,
    key: Keycode,
    keymod: Mod,
    repeat: bool,
) -> Result<EventFlow, String> {
    context.emacs.begin_key();
    if let Some(flow) = panes::handle(context.reborrow(), key, keymod, repeat)?.consumed() {
        return Ok(flow);
    }
    if super::super::experimental::key(context.reborrow(), key, keymod, repeat) {
        return Ok(EventFlow::Continue);
    }
    if let Some(flow) =
        shortcuts::terminal_toggle(context.reborrow(), key, keymod, repeat)?.consumed()
    {
        return Ok(flow);
    }
    if let Some(flow) = terminal::handle(context.reborrow(), key, keymod, repeat)?.consumed() {
        return Ok(flow);
    }

    // Completion precedes Emacs and command-bar handling, as in the original loop.
    if context.lsp_ui.completion_key(
        key,
        repeat,
        context.editor,
        context.other_editor,
        context.command_bar,
    ) {
        *context.dirty = true;
        return Ok(EventFlow::Continue);
    }
    if let Some(flow) = emacs_mode::handle(context.reborrow(), key, keymod, repeat)?.consumed() {
        return Ok(flow);
    }
    if let Some(flow) = shortcuts::open_command(context.reborrow(), key, keymod, repeat)?.consumed()
    {
        return Ok(flow);
    }

    let bound_command = context.key_bindings.command_for(key, keymod, repeat);
    if let Some(flow) =
        command_bar::handle(context.reborrow(), key, keymod, repeat, bound_command)?.consumed()
    {
        return Ok(flow);
    }
    if let Some(flow) = history::handle(
        context.reborrow().into(),
        key,
        keymod,
        repeat,
        bound_command,
    )?
    .consumed()
    {
        return Ok(flow);
    }
    if let Some(flow) =
        conventional::escape(context.editor, context.dirty, *context.vim_enabled, key)?.consumed()
    {
        return Ok(flow);
    }
    if let Some(flow) = vim_mode::handle(
        context.reborrow().into(),
        key,
        keymod,
        repeat,
        bound_command,
    )?
    .consumed()
    {
        return Ok(flow);
    }
    // The last handler is the fallback for keys not consumed by any stage.
    conventional::handle(context.into(), key, keymod, repeat, bound_command)
}
