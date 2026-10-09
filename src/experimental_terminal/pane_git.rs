// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Move-owned snapshots restore Back without rerunning or copying the base view.
use super::super::git_detail::{Commit, GitDetail, Request};
use super::*;

pub(super) struct Saved {
    browser: Browser,
    transcript: SharedTranscript,
    layout: Layout,
    prompt: Prompt,
    selection: OutputSelection,
    range: Option<output_selection::Range>,
    pinned: Option<PinnedTail>,
    fragments: std::sync::Arc<Vec<Fragment>>,
    view: Option<vt100::Parser>,
    paint: Selection,
    browsing: bool,
    offset: usize,
    follow: bool,
    base_id: u64,
    epoch: u64,
    command: Option<String>,
    output_directory: PathBuf,
    relative_safe: bool,
    pending_command: Option<String>,
    pending_launch_other: bool,
}
impl Pane {
    pub(crate) fn can_go_back(&self) -> bool {
        self.saved.is_some()
    }
    pub(crate) fn go_back(&mut self) {
        let Some(saved) = self.saved.take() else {
            return;
        };
        self.copy_job = None;
        self.navigation_job = None;
        self.diagnostic_job = None;
        self.navigation_queue.clear();
        self.visual_navigation = None;
        self.output_press = None;
        self.press = None;
        self.reveal = None;
        self.scroll_anchor = None;
        self.browser = saved.browser;
        self.transcript = saved.transcript;
        self.layout = saved.layout;
        self.prompt = saved.prompt;
        self.output_selection = saved.selection;
        self.output_range = saved.range;
        self.pinned_tail = saved.pinned;
        self.fragments = saved.fragments;
        self.view = saved.view;
        self.selection = saved.paint;
        self.browsing = saved.browsing;
        self.browse_offset = saved.offset;
        self.browse_follow = saved.follow;
        self.browse_base_id = saved.base_id;
        self.browse_epoch = saved.epoch;
        self.last_command = saved.command;
        self.output_directory = saved.output_directory;
        self.relative_links_safe = saved.relative_safe;
        self.pending_command = saved.pending_command;
        self.pending_launch_other = saved.pending_launch_other;
        self.detail_commit = None;
        (self.wake)();
    }
    pub(super) fn open_commit(&mut self, commit: Commit) -> io::Result<()> {
        if self.session.running() || self.browser.running() || self.pending_command.is_some() {
            return Err(io::Error::other(
                "Wait for the command to finish, or stop it before opening a commit",
            ));
        }
        if self.saved.is_some() {
            let request = Request::for_commit(&commit)?;
            self.clear();
            self.browser.execute_git_request(request)?;
            self.detail_commit = Some(commit);
            self.focus_commit_output()?;
            return Ok(());
        }
        let detail = GitDetail::open(commit.clone(), self.wake.clone())?;
        let (browser, transcript) = detail.into_parts();
        let layout = Layout::new(transcript.clone(), self.wake.clone())?;
        self.navigation_job = None;
        self.navigation_queue.clear();
        self.visual_navigation = None;
        self.copy_job = None;
        self.diagnostic_job = None;
        self.output_press = None;
        self.press = None;
        self.reveal = None;
        self.scroll_anchor = None;
        self.saved = Some(Box::new(Saved {
            browser: std::mem::replace(&mut self.browser, browser),
            transcript: std::mem::replace(&mut self.transcript, transcript),
            layout: std::mem::replace(&mut self.layout, layout),
            prompt: std::mem::take(&mut self.prompt),
            selection: std::mem::take(&mut self.output_selection),
            range: self.output_range.take(),
            pinned: self.pinned_tail.take(),
            fragments: std::mem::take(&mut self.fragments),
            view: self.view.take(),
            paint: std::mem::take(&mut self.selection),
            browsing: self.browsing,
            offset: self.browse_offset,
            follow: self.browse_follow,
            base_id: self.browse_base_id,
            epoch: self.browse_epoch,
            command: self.last_command.take(),
            output_directory: std::mem::replace(
                &mut self.output_directory,
                commit.directory().into(),
            ),
            relative_safe: self.relative_links_safe,
            pending_command: self.pending_command.take(),
            pending_launch_other: std::mem::take(&mut self.pending_launch_other),
        }));
        self.browsing = true;
        self.browse_offset = 0;
        self.browse_follow = false;
        self.browse_base_id = 0;
        self.browse_epoch = 0;
        self.relative_links_safe = true;
        self.detail_commit = Some(commit);
        self.focus_commit_output()?;
        (self.wake)();
        Ok(())
    }
    pub(super) fn repeat_commit(&mut self) -> io::Result<()> {
        if let Some(commit) = self.detail_commit.clone() {
            let request = Request::for_commit(&commit)?;
            self.clear();
            self.browser.execute_git_request(request)?;
            self.focus_commit_output()?;
        }
        Ok(())
    }

    fn focus_commit_output(&mut self) -> io::Result<()> {
        let mut source = Source {
            store: &self.transcript,
            session: &self.session,
        };
        let bounds = output_selection::TextSource::bounds(&source);
        self.browse_epoch = bounds.epoch;
        self.browse_base_id = bounds.first;
        self.browse_offset = 0;
        self.browse_follow = false;
        self.output_selection = OutputSelection::default();
        self.output_selection
            .move_logical(&mut source, output_selection::Motion::First, false)?;
        self.output_range = None;
        self.reveal = None;
        self.scroll_anchor = None;
        Ok(())
    }
}
