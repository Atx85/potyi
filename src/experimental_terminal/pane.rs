// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    bridge::Bridge,
    browser::{BrowseAction, Browser, Kind, Row},
    copy_job::CopyJob,
    input,
    layout::{Fragment, Layout, Metrics},
    links::{self, FileLink},
    navigation_job::{NavigationJob, Work},
    output_selection::{self, Selection as OutputSelection},
    output_source::{PinnedTail, Source},
    painting::{self, GlyphCache, Point, Selection},
    prompt::Prompt,
    session::{Session, Wake},
    transcript::{Anchor, RecordData, SharedTranscript, Transcript},
};
use sdl3::{
    event::Event,
    keyboard::Keycode,
    mouse::MouseButton,
    rect::Rect,
    render::{Canvas, TextureCreator},
    ttf::{Font, FontStyle},
    video::{Window, WindowContext},
};
use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
};

#[path = "pane_git.rs"]
mod git;
#[path = "pane_output.rs"]
mod output;

#[derive(Debug)]
pub(crate) struct Ready;
pub(crate) enum Action {
    Handled,
    Editor,
    Open(FileLink),
    CommandBar,
    ClosePane,
}
// The 256KiB visible projection holds fewer than this many nonempty native
// filename segments. Retain ink rectangles only, never paths or text copies.
const MAX_ZERO_NATIVE_HITS: usize = 2048;
struct ZeroNativeHit {
    row: u16,
    segment: usize,
    start: Anchor,
    source: u64,
    left: i32,
    width: u32,
}
pub(crate) struct Pane {
    wake: Wake,
    output_selection: OutputSelection,
    copy_job: Option<CopyJob>,
    clipboard_mode: Option<bool>,
    saved: Option<Box<git::Saved>>,
    detail_commit: Option<super::git_detail::Commit>,
    diagnostic_job: Option<(super::diagnostic_links::Job, bool, Option<Anchor>)>,
    layout: Layout,
    fragments: std::sync::Arc<Vec<Fragment>>,
    zero_native_hits: Vec<ZeroNativeHit>,
    output_range: Option<output_selection::Range>,
    pinned_tail: Option<PinnedTail>,
    navigation_job: Option<NavigationJob>,
    navigation_queue: VecDeque<output::Pending>,
    visual_navigation: Option<(output_selection::Request, Anchor, u16)>,
    reveal: Option<Anchor>,
    scroll_anchor: Option<Anchor>,
    output_press: Option<output::Press>,
    session: Session,
    bridge: Bridge,
    browser: Browser,
    transcript: SharedTranscript,
    view: Option<vt100::Parser>,
    prompt: Prompt,
    browsing: bool,
    browse_offset: usize,
    browse_follow: bool,
    browse_base_id: u64,
    browse_epoch: u64,
    queued: VecDeque<FileLink>,
    last_command: Option<String>,
    output_directory: PathBuf,
    relative_links_safe: bool,
    selection: Selection,
    press: Option<(Point, bool)>,
    wheel: f32,
    area: Rect,
    cell: (i32, i32),
    pending_command: Option<String>,
    pending_launch_other: bool,
    pub(crate) pane: usize,
    pub(crate) visible: bool,
}
impl Pane {
    pub(crate) fn new(
        directory: &Path,
        rows: u16,
        cols: u16,
        wake: Wake,
        pane: usize,
    ) -> io::Result<Self> {
        let bridge = Bridge::new(wake.clone())?;
        #[allow(unused_mut)]
        let mut environment = bridge.environment()?;
        #[cfg(test)]
        {
            let client = std::env::var_os("POTYI_TERM_TEST_CLIENT")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::current_exe()
                        .ok()?
                        .parent()?
                        .parent()
                        .map(|p| p.join(format!("potyi{}", std::env::consts::EXE_SUFFIX)))
                });
            if let Some(client) = client.filter(|p| p.is_file()) {
                environment[0].1 = client.to_string_lossy().into_owned();
            }
        }
        let transcript = Transcript::shared()?;
        let mut browser =
            Browser::open_empty_with_transcript(directory, wake.clone(), transcript.clone())?;
        browser.seed_banner()?;
        let session = Session::open_with_transcript(
            directory,
            rows,
            cols,
            wake.clone(),
            &environment,
            transcript.clone(),
        )?;
        let output_directory = browser.directory().to_path_buf();
        let layout = Layout::new(transcript.clone(), wake.clone())?;
        Ok(Self {
            wake,
            output_selection: OutputSelection::default(),
            copy_job: None,
            clipboard_mode: None,
            saved: None,
            detail_commit: None,
            diagnostic_job: None,
            layout,
            fragments: std::sync::Arc::new(Vec::new()),
            zero_native_hits: Vec::new(),
            output_range: None,
            pinned_tail: None,
            navigation_job: None,
            navigation_queue: VecDeque::new(),
            visual_navigation: None,
            reveal: None,
            scroll_anchor: None,
            output_press: None,
            session,
            bridge,
            browser,
            transcript,
            view: None,
            prompt: Prompt::default(),
            browsing: true,
            browse_offset: 0,
            browse_follow: true,
            browse_base_id: 0,
            browse_epoch: 0,
            queued: VecDeque::new(),
            last_command: None,
            output_directory,
            relative_links_safe: true,
            selection: Selection::default(),
            press: None,
            wheel: 0.0,
            area: Rect::new(0, 0, 1, 1),
            cell: (1, 1),
            pending_command: None,
            pending_launch_other: false,
            pane,
            visible: true,
        })
    }
    pub(crate) fn reopen(&mut self) {
        self.visible = true;
        self.focus_prompt();
        self.browse_follow = true;
        self.scroll_anchor = None;
        self.reveal = None;
        self.browser.clear_status();
        self.session.status = None;
    }
    pub(crate) fn clipboard(
        &mut self,
        paste: bool,
        all: bool,
        clipboard: &sdl3::clipboard::ClipboardUtil,
    ) {
        if paste {
            let result = clipboard
                .clipboard_text()
                .map_err(|e| e.to_string())
                .and_then(|text| {
                    self.focus_prompt();
                    if self.session.running()
                        && self.session.supports_input()
                        && self.session.live_screen().alternate_screen()
                    {
                        self.session.paste(&text).map_err(|e| e.to_string())
                    } else {
                        self.prompt.insert(&text);
                        Ok(())
                    }
                });
            if let Err(error) = result {
                self.set_status(format!("Clipboard paste failed: {error}"));
            }
        } else if self.view.is_some() {
            self.clipboard_copy(all);
        } else {
            let selected = self.selection.text(self.output_screen());
            let text = if all || selected.is_empty() {
                self.output_screen().contents()
            } else {
                selected
            };
            match clipboard.set_clipboard_text(&text) {
                Ok(()) => self.clipboard_mode = Some(false),
                Err(error) => self.set_status(format!("Clipboard copy failed: {error}")),
            }
        }
    }
    pub(crate) fn closed(&self) -> bool {
        self.session.closed
    }
    pub(crate) fn dragging(&self) -> bool {
        self.selection.dragging || self.output_press.is_some()
    }
    #[cfg(test)]
    pub(crate) fn contents(&self) -> String {
        let mut store = self.transcript.lock().unwrap();
        let mut text = String::new();
        for index in 0..store.len() {
            let Ok(record) = store.read(index) else { break };
            match record.data {
                RecordData::Native(bytes) => {
                    if let Ok(row) = super::browser::decode_native(&bytes) {
                        text.push_str(&row.text);
                        text.push('\n');
                    }
                }
                RecordData::Terminal {
                    cols,
                    wrapped,
                    formatted,
                } => {
                    if cols == 0 {
                        if let Ok(value) = super::output_colors::plain_text(&formatted) {
                            text.push_str(value);
                        }
                    } else {
                        let mut parser = vt100::Parser::new(1, cols.clamp(1, 256), 0);
                        parser.process(&formatted);
                        text.push_str(&parser.screen().contents());
                    }
                    if !wrapped {
                        text.push('\n');
                    }
                }
                RecordData::Header(value) | RecordData::Result { text: value, .. } => {
                    text.push_str(&value);
                    text.push('\n');
                }
            }
        }
        if self.session.running() {
            if let Some((tail, _)) = self.session.plain_tail() {
                text.push_str(tail);
            } else {
                text.push_str(&self.session.live_screen().contents());
            }
        }
        text
    }
    #[cfg(test)]
    pub(crate) fn directory(&self) -> &Path {
        self.browser.directory()
    }
    #[cfg(test)]
    pub(crate) fn ready(&self) -> bool {
        self.session.ready()
    }
    pub(crate) fn prompt_text(&self) -> &str {
        self.prompt.text()
    }
    #[cfg(test)]
    pub(crate) fn browser_rows(&mut self) -> Vec<Row> {
        let mut store = self.transcript.lock().unwrap();
        let mut seen = std::collections::HashSet::new();
        self.fragments
            .iter()
            .flat_map(|fragment| fragment.segments.iter())
            .filter_map(|segment| {
                if !seen.insert(segment.record_id) {
                    return None;
                }
                let record = store.read_id(segment.record_id).ok()?;
                if !matches!(record.data, RecordData::Native(_)) {
                    return None;
                }
                self.browser.decorate_record(&record).ok()
            })
            .collect()
    }
    pub(crate) fn pending(&self) -> bool {
        self.session.work_pending
            || self.browser.pending()
            || self.visible
                && (self.layout.pending()
                    || !self.navigation_queue.is_empty()
                    || self.visual_navigation.is_some()
                    || self.reveal.is_some()
                    || self.scroll_anchor.is_some())
    }
    pub(crate) fn command_running(&self) -> bool {
        self.session.running()
    }
    pub(crate) fn running(&self) -> bool {
        self.session.running() || self.browser.working() || self.pending_command.is_some()
    }
    pub(crate) fn set_prompt_cursor(&mut self, byte: usize) {
        self.focus_prompt();
        self.prompt.set_cursor(byte);
    }
    pub(crate) fn prompt_cursor(&self) -> usize {
        self.prompt.cursor()
    }
    pub(crate) fn status(&self) -> Option<&str> {
        if self.browsing {
            self.browser.status()
        } else {
            self.session.status.as_deref()
        }
    }
    pub(crate) fn output_focus_hint(&self) -> Option<&'static str> {
        self.output_selection
            .focused()
            .then(|| match self.output_selection.visual() {
                output_selection::Visual::Off => "Output · select and copy · Esc: command input",
                output_selection::Visual::Character => {
                    "Output · VISUAL · y: copy · Esc: command input"
                }
                output_selection::Visual::Line => {
                    "Output · VISUAL LINE · y: copy · Esc: command input"
                }
            })
    }
    pub(crate) fn prompt_view(&self, columns: usize) -> (String, usize) {
        if self.output_selection.focused() || self.navigation_job.is_some() {
            return (
                "Output selected · Esc returns to the command".into(),
                usize::MAX,
            );
        }
        if self.session.running() && self.session.supports_input() {
            return (
                "Running · keyboard input goes to the program".into(),
                usize::MAX,
            );
        }
        let columns = columns.clamp(1, 1000);
        let before: Vec<_> = self.prompt.text()[..self.prompt.cursor()]
            .chars()
            .rev()
            .take(columns.saturating_sub(3))
            .collect();
        let mut text = String::from("> ");
        text.extend(before.into_iter().rev());
        let cursor = text.chars().count();
        text.extend(
            self.prompt.text()[self.prompt.cursor()..]
                .chars()
                .take(columns.saturating_sub(cursor)),
        );
        (text.replace(['\n', '\t'], " "), cursor)
    }
    pub(crate) fn poll(&mut self) -> bool {
        let mut changed = self.session.poll() | self.browser.poll();
        if self.browser.take_failure() {
            self.pending_command = None;
        }
        while let Ok(request) = self.bridge.requests.try_recv() {
            changed = true;
            if request.operation == "complete" {
                if let (Some(id), Some(status)) = (request.command_id, request.exit_status) {
                    self.session.command_finished(id, status, request.directory);
                }
                continue;
            }
            if request.operation == "cwd" {
                if request.directory != self.output_directory {
                    self.relative_links_safe = false;
                    self.session.invalidate_relative_links();
                }
                // The native directory changes only after the managed command
                // completes. A helper never injects cd into a running child's stdin.
                continue;
            }
            if request.directory != self.output_directory {
                self.session.invalidate_relative_links();
            }
            let exact = if request.path == "~"
                || request.path.starts_with("~/")
                || request.path.starts_with("~\\")
            {
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(PathBuf::from)
                    .unwrap_or(request.directory.clone())
                    .join(request.path.get(2..).unwrap_or(""))
            } else {
                request.directory.join(&request.path)
            };
            let (value, line, column) = if exact.exists() {
                (&*request.path, None, None)
            } else {
                crate::parse_location(&request.path)
                    .map(|(p, l, c)| (p, Some(l), c))
                    .unwrap_or((&request.path, None, None))
            };
            if value.is_empty() {
                self.set_status("Usage: edit/view PATH[:LINE[:COLUMN]]".into());
                continue;
            }
            let path = if value == "~" || value.starts_with("~/") || value.starts_with("~\\") {
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(PathBuf::from)
                    .unwrap_or(request.directory.clone())
                    .join(value.get(2..).unwrap_or(""))
            } else {
                request.directory.join(value)
            };
            if self.queued.len() < 32 {
                self.queued.push_back(FileLink {
                    path,
                    line,
                    column,
                    byte_column: false,
                    read_only: request.operation == "view",
                    other_pane: true,
                });
            }
        }
        if let Some(result) = self
            .diagnostic_job
            .as_mut()
            .and_then(|(job, _, _)| job.poll())
        {
            let (_, other, keyboard) = self.diagnostic_job.take().unwrap();
            match result {
                Ok(Some(mut link)) if self.queued.len() < 32 => {
                    link.other_pane = other;
                    self.queued.push_back(link);
                }
                Ok(None)
                    if keyboard.is_some()
                        && keyboard == self.output_selection.cursor()
                        && self.navigation_job.is_none()
                        && self.navigation_queue.is_empty()
                        && self.visual_navigation.is_none() =>
                {
                    self.focus_prompt()
                }
                Ok(_) => {}
                Err(error) => self.set_status(error.to_string()),
            }
            changed = true;
        }
        if let Some(completion) = self.session.take_completion() {
            if completion.directory != self.output_directory {
                self.relative_links_safe = false;
            }
            if completion.directory.is_dir() && completion.directory != self.browser.directory() {
                if let Err(error) = self.browser.enter(&completion.directory) {
                    self.set_status(error.to_string());
                }
                self.prompt.reset_completion();
            }
            self.session.status = None;
            changed = true;
        }
        if changed {
            if let Some((text, source)) = self.session.plain_tail() {
                if let Err(error) = self.layout.set_plain_tail(text, source) {
                    self.set_status(error.to_string());
                }
            } else if let Some((screen, start, count)) = self.session.tail_screen() {
                if !screen.alternate_screen() {
                    if let Err(error) = self.layout.set_tail(
                        screen,
                        start,
                        count,
                        self.session.tail_source().unwrap_or(0),
                    ) {
                        self.set_status(error.to_string());
                    }
                }
            } else if let Err(error) = self.layout.set_tail(self.session.live_screen(), 0, 0, 0) {
                self.set_status(error.to_string());
            }
        }
        match self.layout.poll() {
            Ok(ready) => changed |= ready,
            Err(error) => self.set_status(error.to_string()),
        }
        changed |= self.poll_navigation();
        if self.session.ready()
            && !self.browser.running()
            && let Some(command) = self.pending_command.take()
        {
            let other = std::mem::take(&mut self.pending_launch_other);
            if let Err(error) = self.launch_command(&command, other) {
                self.set_status(error.to_string());
            }
            changed = true;
        }
        changed
    }
    pub(crate) fn request(&mut self) -> Option<FileLink> {
        self.queued.pop_front()
    }
    pub(crate) fn enter_directory(&mut self, directory: &Path) -> io::Result<()> {
        if self.saved.is_some() {
            self.go_back();
        }
        if self.session.running() {
            return Err(io::Error::other(
                "Stop the running command before changing directory",
            ));
        }
        self.pending_command = None;
        self.pending_launch_other = false;
        self.browser.enter(directory)?;
        self.browsing = true;
        self.browse_offset = 0;
        self.browse_follow = true;
        self.prompt.reset_completion();
        Ok(())
    }
    pub(crate) fn launch_command(&mut self, command: &str, other_pane: bool) -> io::Result<()> {
        let queued = self.queued.len();
        self.send_command(command)?;
        if self.pending_command.as_deref() == Some(command) {
            self.pending_launch_other = other_pane;
        } else if self.queued.len() > queued {
            self.queued.back_mut().unwrap().other_pane = other_pane;
        }
        Ok(())
    }
    pub(crate) fn send_command(&mut self, command: &str) -> io::Result<()> {
        if self.saved.is_some() && command.trim() == "back" {
            self.go_back();
            return Ok(());
        }
        if self.saved.is_some() && command.trim() != "clear" {
            if self.browser.running() {
                return Err(io::Error::other("A command is already running"));
            }
            self.go_back();
        }
        if command.len() > super::session::MAX_PASTE_BYTES {
            return Err(io::Error::other("Command exceeds 64 KiB"));
        }
        if self.session.running() {
            return Err(io::Error::other("A command is already running"));
        }
        if self.pending_command.is_some() {
            return Err(io::Error::other(
                "A command is already waiting for the terminal",
            ));
        }
        if self.browser.running() {
            self.pending_command = Some(command.into());
            return Ok(());
        }
        self.prompt.remember_command(command);
        if command.trim() == "clear" {
            self.clear();
            self.last_command = Some(command.into());
            return Ok(());
        }
        let before = self.transcript.lock().unwrap().len();
        let action = match self.browser.execute(command) {
            Ok(action) => action,
            Err(error) => {
                if self.transcript.lock().unwrap().len() == before {
                    let _ = self.browser.echo_command(command);
                }
                return Err(error);
            }
        };
        if matches!(action, BrowseAction::Open { .. } | BrowseAction::Editor) {
            self.browser.echo_command(command)?;
        }
        self.last_command = Some(command.into());
        self.prompt.reset_completion();
        match action {
            BrowseAction::NotHandled => {
                if !self.session.ready() {
                    self.pending_command = Some(command.into());
                    return Ok(());
                }
                self.browser.stop();
                if self.browser.directory() != self.output_directory {
                    self.relative_links_safe = false;
                }
                self.output_directory = self.browser.directory().to_path_buf();
                self.relative_links_safe = true;
                self.session
                    .start_command(command, self.browser.directory())?;
                self.session.status = Some("Running…".into());
                self.browsing = false;
            }
            BrowseAction::Changed => {
                self.browsing = true;
                self.browse_offset = 0;
                self.browse_follow = true;
            }
            BrowseAction::Open {
                path,
                read_only,
                line,
                column,
            } => {
                if self.queued.len() < 32 {
                    self.queued.push_back(FileLink {
                        path,
                        read_only,
                        line,
                        column,
                        byte_column: false,
                        other_pane: false,
                    });
                }
            }
            BrowseAction::Editor => self.visible = false,
        }
        Ok(())
    }
    pub(crate) fn set_status(&mut self, status: String) {
        if self.browsing {
            self.browser.set_status(status);
        } else {
            self.session.status = Some(status);
        }
    }
    pub(crate) fn stop(&mut self) {
        let waiting = self.pending_command.take().is_some();
        if self.session.running() {
            if let Err(e) = self.session.send(vec![3]) {
                self.set_status(e.to_string());
            }
        } else if self.browser.working() {
            self.browser.stop();
        } else if self.saved.is_some() {
            if let Err(error) = self.repeat_commit() {
                self.set_status(error.to_string());
            }
        } else if !waiting && let Some(command) = self.last_command.clone() {
            self.prompt.clear();
            if let Err(error) = self.send_command(&command) {
                self.set_status(error.to_string());
            }
        }
    }
    pub(crate) fn clear(&mut self) {
        self.copy_job = None;
        self.diagnostic_job = None;
        self.focus_prompt();
        self.fragments = std::sync::Arc::new(Vec::new());
        self.view = None;
        self.pending_command = None;
        self.pending_launch_other = false;
        self.selection.clear();
        if let Err(error) = self.transcript.lock().unwrap().clear() {
            self.session.status = Some(error.to_string());
        }
        if self.saved.is_none() {
            self.session.clear();
        }
        self.output_directory = self.browser.directory().to_path_buf();
        self.relative_links_safe = true;
        self.browser.cancel_for_clear();
        self.browse_offset = 0;
        self.browse_follow = true;
    }
    fn start_copy(&mut self, all: bool, yank: bool) -> Result<(), String> {
        let source = Source {
            store: &self.transcript,
            session: &self.session,
        };
        let job = CopyJob::start_pinned(
            &source,
            &self.output_selection,
            all,
            yank,
            self.wake.clone(),
            self.pinned_tail.as_ref(),
        )
        .map_err(|error| error.to_string())?;
        self.copy_job = Some(job);
        self.set_status("Preparing output copy…".into());
        Ok(())
    }
    pub(crate) fn take_clipboard_mode(&mut self) -> Option<bool> {
        self.clipboard_mode.take()
    }
    pub(crate) fn poll_copy(&mut self, clipboard: &sdl3::clipboard::ClipboardUtil) -> bool {
        let Some(result) = self.copy_job.as_ref().and_then(CopyJob::poll) else {
            return false;
        };
        let job = self.copy_job.take().unwrap();
        if job.epoch != self.transcript.lock().unwrap().epoch() {
            return false;
        }
        match result {
            Ok(copied) if copied.text.is_empty() => self.set_status("Nothing selected".into()),
            Ok(copied) => match clipboard.set_clipboard_text(&copied.text) {
                Ok(()) => {
                    self.clipboard_mode =
                        Some(!job.all && job.selection.2 == output_selection::Visual::Line);
                    if job.yank
                        && job.selection
                            == (
                                self.output_selection.anchor(),
                                self.output_selection.cursor(),
                                self.output_selection.visual(),
                            )
                    {
                        if let Some(selection) = copied.after_yank {
                            self.output_selection = selection;
                            self.output_range = None;
                        }
                    }
                    self.set_status(
                        if job.all {
                            "Output copied"
                        } else {
                            "Selection copied"
                        }
                        .into(),
                    );
                }
                Err(error) => self.set_status(format!("Clipboard copy failed: {error}")),
            },
            Err(error) => self.set_status(error),
        }
        true
    }
    pub(crate) fn draw<'a>(
        &mut self,
        canvas: &mut Canvas<Window>,
        creator: &'a TextureCreator<WindowContext>,
        font: &mut Font<'_>,
        cache: &mut GlyphCache<'a>,
        cell: (i32, i32),
        scale: f32,
        focused: bool,
        area: Rect,
        tab_width: usize,
        font_key: u32,
        measure: &mut dyn FnMut(char) -> i32,
    ) -> Result<(), String> {
        self.area = area;
        self.cell = cell;
        self.layout.set_font_columns(true);
        let rows = ((area.height() as i32 - painting::MARGIN - painting::FOOTER) / cell.1)
            .clamp(1, u16::MAX as i32) as u16;
        let cols = ((area.width() as i32 - painting::MARGIN * 2) / cell.0).clamp(1, u16::MAX as i32)
            as u16;
        self.browser.set_columns(usize::from(cols));
        if self
            .session
            .resize(
                rows.min(super::session::MAX_ROWS),
                cols.min(super::session::MAX_COLS),
            )
            .map_err(|e| e.to_string())?
        {
            self.selection.clear();
        }
        if self
            .session
            .tail_screen()
            .is_some_and(|(screen, _, _)| screen.alternate_screen())
        {
            self.view = None;
        } else {
            self.refresh_output(rows, cols, tab_width, font_key, measure)
                .map_err(|e| e.to_string())?;
        }
        self.layout.set_font_columns(true);
        self.zero_native_hits.clear();
        if self.view.is_some() {
            font.set_style(FontStyle::NORMAL);
            for (row, fragment) in self.fragments.iter().enumerate() {
                for (index, segment) in fragment.segments.iter().enumerate() {
                    if self.zero_native_hits.len() == MAX_ZERO_NATIVE_HITS {
                        break;
                    }
                    let text = &fragment.text[segment.range.clone()];
                    if !segment.native
                        || text.is_empty()
                        || fragment.text_columns(text) != 0
                        || segment.range.end > 4096
                    {
                        continue;
                    }
                    // Use whole-row prefixes: a mark after packed-row padding
                    // can combine into that padding and have no legacy hit span.
                    let left = font
                        .size_of(&fragment.text[..segment.range.start])
                        .map_err(|e| e.to_string())?
                        .0;
                    let right = font
                        .size_of(&fragment.text[..segment.range.end])
                        .map_err(|e| e.to_string())?
                        .0;
                    let left = (left as f32 / scale).round() as i32;
                    let right = (right as f32 / scale).round() as i32;
                    if right > left {
                        self.zero_native_hits.push(ZeroNativeHit {
                            row: row as u16,
                            segment: index,
                            start: segment.start,
                            source: segment.source,
                            left,
                            width: (right - left) as u32,
                        });
                    }
                }
            }
            let cursor = self
                .output_selection
                .focused()
                .then(|| {
                    self.output_selection
                        .cursor()
                        .and_then(|anchor| self.anchor_point(anchor).map(|point| (point, anchor)))
                })
                .flatten();
            return painting::draw_fragments(
                canvas,
                creator,
                font,
                cache,
                &self.fragments,
                &self.selection,
                self.output_range,
                cursor,
                cell.0,
                cell.1,
                scale,
                focused,
                area,
            );
        }
        painting::draw_area(
            canvas,
            creator,
            font,
            cache,
            self.output_screen(),
            &self.selection,
            self.status(),
            cell.0,
            cell.1,
            scale,
            focused,
            self.view.is_none(),
            area,
        )
    }
    fn output_screen(&self) -> &vt100::Screen {
        self.view
            .as_ref()
            .map_or(self.session.live_screen(), |view| view.screen())
    }
    fn point(&self, x: f32, y: f32) -> Point {
        let (rows, cols) = self.output_screen().size();
        Point {
            row: ((y as i32 - self.area.y() - painting::MARGIN).max(0) / self.cell.1)
                .min(rows as i32 - 1) as u16,
            col: ((x as i32 - self.area.x() - painting::MARGIN).max(0) / self.cell.0)
                .min(cols as i32) as u16,
        }
    }
    fn link(&self, point: Point) -> Option<FileLink> {
        let screen = self.output_screen();
        let mut text = String::new();
        let mut column = 0;
        for col in 0..screen.size().1 {
            let cell = screen.cell(point.row, col)?;
            if cell.is_wide_continuation() {
                continue;
            }
            let contents = cell.contents();
            if col < point.col {
                column += if contents.is_empty() {
                    1
                } else {
                    contents.chars().count()
                };
            }
            text.push_str(if contents.is_empty() { " " } else { contents });
        }
        links::at(text.trim_end(), column, &self.output_directory).and_then(|(range, link)| {
            // The browser stores exact paths. Raw terminal rows lack per-command
            // provenance, so only absolute links remain safe after cwd changes.
            let value = text[range].trim_matches(['\'', '"', '(', ')', '[', ']', ',', ';']);
            Path::new(value).is_absolute().then_some(link)
        })
    }
    fn scroll(&mut self, rows: isize) {
        self.browse_follow = false;
        self.scroll_anchor = None;
        self.reveal = None;
        self.browse_offset = self.browse_offset.saturating_add_signed(-rows);
    }
    pub(crate) fn event(
        &mut self,
        event: &Event,
        modifiers: sdl3::keyboard::Mod,
        clipboard: &sdl3::clipboard::ClipboardUtil,
        vim: bool,
    ) -> Option<Action> {
        if let Event::KeyDown {
            keycode: Some(key),
            keymod,
            repeat: true,
            ..
        } = event
        {
            if matches!(
                *key,
                Keycode::Return | Keycode::KpEnter | Keycode::Tab | Keycode::F6
            ) || input::control(*keymod) && matches!(*key, Keycode::C | Keycode::L)
                || (input::command(*keymod) || input::control(*keymod) && input::shift(*keymod))
                    && matches!(*key, Keycode::C | Keycode::V)
                || input::alt(*keymod) && *key == Keycode::Left
            {
                return Some(Action::Handled);
            }
        }
        if let Event::KeyDown {
            keycode: Some(Keycode::Left),
            keymod,
            repeat: false,
            ..
        } = event
        {
            if input::alt(*keymod) && self.saved.is_some() {
                self.go_back();
                return Some(Action::Handled);
            }
        }
        if let Event::KeyDown {
            keycode: Some(Keycode::L),
            keymod,
            ..
        } = event
        {
            if input::control(*keymod) {
                self.clear();
                return Some(Action::Handled);
            }
        }
        if let Some(action) = self.output_event(event, modifiers, clipboard, vim) {
            return action;
        }
        if matches!(
            event,
            Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            }
        ) {
            return Some(Action::Editor);
        }
        let result: Result<(), String> = match event {
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if *key == Keycode::Grave && input::control(*keymod) {
                    return Some(Action::Editor);
                }
                if *key == Keycode::P && input::control(*keymod) {
                    return Some(Action::CommandBar);
                }
                let copy = *key == Keycode::C
                    && (input::command(*keymod)
                        || (input::control(*keymod) && input::shift(*keymod)));
                let paste = *key == Keycode::V
                    && (input::command(*keymod)
                        || (input::control(*keymod) && input::shift(*keymod)));
                if copy || paste {
                    self.clipboard(paste, input::shift(*keymod), clipboard);
                    Ok(())
                } else if !self.session.running()
                    || !self.session.supports_input()
                    || !self.session.live_screen().alternate_screen()
                {
                    match *key {
                        Keycode::Escape => return Some(Action::Editor),
                        Keycode::Return | Keycode::KpEnter => {
                            if self.session.running() {
                                self.set_status("A command is already running. Stop it before submitting another command.".into());
                                return Some(Action::Handled);
                            }
                            if let Some(command) = self.prompt.take_command() {
                                if command == ":exit" {
                                    return Some(Action::ClosePane);
                                }
                                if let Err(error) = self.send_command(&command) {
                                    self.prompt.insert(&command);
                                    self.set_status(error.to_string());
                                }
                            }
                        }
                        Keycode::Backspace => self.prompt.backspace(),
                        Keycode::Delete => self.prompt.delete(),
                        Keycode::Left => self.prompt.move_left(),
                        Keycode::Right => self.prompt.move_right(),
                        Keycode::Home => self.prompt.home(),
                        Keycode::End => self.prompt.end(),
                        Keycode::Up => self.prompt.history_previous(),
                        Keycode::Down => self.prompt.history_next(),
                        Keycode::Tab => {
                            let candidates: Vec<_> = self
                                .browser
                                .completion()
                                .iter()
                                .map(|c| (c.path.clone(), c.directory))
                                .collect();
                            if let Some(status) = self.prompt.complete(
                                input::shift(*keymod),
                                self.browser.directory(),
                                &candidates,
                            ) {
                                self.set_status(status);
                            }
                        }
                        Keycode::C if input::control(*keymod) => {
                            if self.running() {
                                self.pending_command = None;
                                if self.session.running() {
                                    if let Err(error) = self.session.send(vec![3]) {
                                        self.set_status(error.to_string());
                                    }
                                } else {
                                    self.browser.stop();
                                }
                            }
                        }
                        Keycode::L if input::control(*keymod) => self.clear(),
                        Keycode::PageUp => self.scroll(20),
                        Keycode::PageDown => self.scroll(-20),
                        _ => {}
                    }
                    Ok(())
                } else if input::shift(*keymod)
                    && matches!(*key, Keycode::PageUp | Keycode::PageDown)
                {
                    self.scroll(if *key == Keycode::PageUp { 20 } else { -20 });
                    self.selection.clear();
                    Ok(())
                } else if input::command(*keymod) {
                    Ok(())
                } else {
                    input::key_bytes(
                        *key,
                        *keymod,
                        self.session.live_screen().application_cursor(),
                    )
                    .map_or(Ok(()), |bytes| {
                        self.session.send(bytes).map_err(|e| e.to_string())
                    })
                }
            }
            Event::TextInput { text, .. } => {
                if (!input::control(modifiers) || input::alt_graph(modifiers))
                    && !input::command(modifiers)
                {
                    if !self.session.running()
                        || !self.session.supports_input()
                        || !self.session.live_screen().alternate_screen()
                    {
                        self.prompt.insert(text);
                        return Some(Action::Handled);
                    }
                    let bytes = if input::alt(modifiers) && !input::alt_graph(modifiers) {
                        [b"\x1b".as_slice(), text.as_bytes()].concat()
                    } else {
                        text.as_bytes().to_vec()
                    };
                    self.selection.clear();
                    self.session.send(bytes).map_err(|e| e.to_string())
                } else {
                    Ok(())
                }
            }
            Event::MouseWheel { y, direction, .. } => {
                let delta = *y
                    * if *direction == sdl3::mouse::MouseWheelDirection::Flipped {
                        -3.0
                    } else {
                        3.0
                    };
                if delta.is_finite() {
                    if delta != 0.0 && delta.signum() != self.wheel.signum() {
                        self.wheel = 0.0;
                    }
                    self.wheel += delta;
                    let rows = self.wheel.trunc() as isize;
                    self.wheel -= rows as f32;
                    if rows != 0 {
                        self.scroll(rows);
                    }
                }
                self.selection.clear();
                Ok(())
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                clicks,
                ..
            } => {
                let point = self.point(*x, *y);
                self.press = Some((
                    point,
                    input::command(modifiers) || input::control(modifiers),
                ));
                self.selection = Selection {
                    anchor: Some(point),
                    end: Some(point),
                    dragging: true,
                    spans: Vec::new(),
                };
                if *clicks >= 2 {
                    self.selection.anchor = Some(Point {
                        row: point.row,
                        col: 0,
                    });
                    self.selection.end = Some(Point {
                        row: point.row,
                        col: self.output_screen().size().1,
                    });
                }
                Ok(())
            }
            Event::MouseMotion { x, y, .. } if self.selection.dragging => {
                self.selection.end = Some(self.point(*x, *y));
                Ok(())
            }
            Event::MouseButtonUp {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                self.selection.dragging = false;
                if let Some((start, other)) = self.press.take()
                    && start == self.point(*x, *y)
                    && self.selection.anchor == self.selection.end
                    && let Some(mut link) = self.link(start)
                {
                    link.other_pane = other;
                    return Some(Action::Open(link));
                }
                Ok(())
            }
            Event::Window {
                win_event: sdl3::event::WindowEvent::FocusLost,
                ..
            } => {
                self.selection.dragging = false;
                self.press = None;
                return None;
            }
            _ => return None,
        };
        if let Err(error) = result {
            self.set_status(error);
        }
        Some(Action::Handled)
    }
}
