// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Small command bridge, built and shipped beside the editor for every target.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

pub(crate) type Wake = std::sync::Arc<dyn Fn() + Send + Sync>;
// Keep authentication, framing, limits and the Windows console reader identical
// to the fallback helper entry point in the editor executable.
#[allow(dead_code)]
#[path = "../experimental_terminal/bridge.rs"]
mod bridge;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let result = if arguments.next().as_deref() == Some("--term-request") {
        bridge::client(arguments.collect())
    } else {
        Err(std::io::Error::other("Expected --term-request"))
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
