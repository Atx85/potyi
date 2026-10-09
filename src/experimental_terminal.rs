// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in embedded terminal, independent of the original command panel.
pub(crate) mod bridge;
pub(crate) mod browser;
#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod comparison;
mod copy_job;
mod history;
mod input;
mod lifecycle;
pub(crate) mod links;
mod navigation_job;
mod output_selection;
mod output_source;
pub(crate) mod painting;
pub(crate) mod pane;
mod prompt;
mod session;
mod transcript;
#[cfg(test)]
mod window;
pub(crate) use session::Wake;
pub(crate) fn run(command: Option<String>) -> Result<(), String> {
    crate::app::run_experimental(command)
}

mod layout;

#[cfg(windows)]
mod windows_command;

mod git_detail;

mod diagnostic_links;

mod output_colors;

mod literal_output;
