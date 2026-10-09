// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! SDL-free helper executable for the production backend checks.
#![allow(dead_code)]
pub(crate) type Wake = std::sync::Arc<dyn Fn() + Send + Sync>;
#[path = "../../../../src/experimental_terminal/bridge.rs"]
mod bridge;
fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let result = if arguments.first().is_some_and(|a| a == "--term-request") {
        bridge::client(arguments.into_iter().skip(1).collect())
    } else {
        Err(std::io::Error::other("Expected --term-request"))
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
