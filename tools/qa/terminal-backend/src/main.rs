// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
#![allow(dead_code)]
pub(crate) type Wake = std::sync::Arc<dyn Fn() + Send + Sync>;
#[path = "../../../../src/experimental_terminal/bridge.rs"]
mod bridge;
fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--term-request") {
        if let Err(error) = bridge::client(args.collect()) {
            eprintln!("{error}");
            std::process::exit(1);
        }
    } else {
        std::process::exit(2);
    }
}
