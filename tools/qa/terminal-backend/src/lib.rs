// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! SDL-free harness for native PTY checks and cross-target compile checks.
#![allow(dead_code)]
#[path = "../../../../src/experimental_terminal/session.rs"]
mod session;

pub(crate) use session::Wake;
#[path = "../../../../src/experimental_terminal/bridge.rs"]
mod bridge;
#[path = "../../../../src/experimental_terminal/links.rs"]
mod links;

#[path = "../../../../src/experimental_terminal/history.rs"]
mod history;

#[path = "../../../../src/experimental_terminal/lifecycle.rs"]
mod lifecycle;
#[path = "../../../../src/experimental_terminal/prompt.rs"]
mod prompt;
#[path = "../../../../src/experimental_terminal/browser.rs"]
mod browser;

#[path = "../../../../src/experimental_terminal/transcript.rs"]
mod transcript;

#[path = "../../../../src/experimental_terminal/output_selection.rs"]
mod output_selection;
#[path = "../../../../src/experimental_terminal/output_source.rs"]
mod output_source;

#[path = "../../../../src/experimental_terminal/layout.rs"]
mod layout;
#[path = "../../../../src/experimental_terminal/navigation_job.rs"]
mod navigation_job;
#[path = "../../../../src/experimental_terminal/copy_job.rs"]
mod copy_job;

#[cfg(windows)]
#[path = "../../../../src/experimental_terminal/windows_command.rs"]
mod windows_command;

#[path = "../../../../src/experimental_terminal/git_detail.rs"]
mod git_detail;

#[path = "../../../../src/experimental_terminal/diagnostic_links.rs"]
mod diagnostic_links;

#[path = "../../../../src/experimental_terminal/output_colors.rs"]
mod output_colors;

#[path = "../../../../src/experimental_terminal/literal_output.rs"]
mod literal_output;
