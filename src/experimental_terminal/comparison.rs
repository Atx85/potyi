// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Exercise both backends without changing the legacy implementation.
use super::session::Session;
use crate::terminal::{Terminal, TerminalEvent};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

fn legacy(command: &str) -> (String, Duration) {
    let sdl = sdl3::init().unwrap();
    let events = sdl.event().unwrap();
    crate::terminal::register_test_events(&events);
    let mut pump = sdl.event_pump().unwrap();
    let mut terminal = Terminal::new(std::env::current_dir().unwrap()).unwrap();
    terminal.set_events(events);
    terminal.clear().unwrap();
    let start = Instant::now();
    terminal.run_command(command).unwrap();
    while terminal.is_running() {
        for event in pump.poll_iter() {
            if let Some(event) = event.as_user_event_type::<TerminalEvent>() {
                terminal.handle_event(event).unwrap();
            }
        }
        terminal.poll_background().unwrap();
        assert!(start.elapsed() < Duration::from_secs(15));
        thread::sleep(Duration::from_millis(2));
    }
    (terminal.output_text().unwrap(), start.elapsed())
}

fn wait(session: &mut Session, expected: &str) {
    let start = Instant::now();
    loop {
        session.poll();
        let (_, cols) = session.screen().size();
        if session
            .screen()
            .rows(0, cols)
            .any(|row| row.trim() == expected)
        {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "{:?}",
            session.screen().contents()
        );
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
#[ignore = "uses SDL custom events; run with SDL_VIDEODRIVER=dummy and one test thread"]
fn experimental_terminal_comparison_both_backends_run_the_same_shell_command() {
    #[cfg(unix)]
    let command = "printf 'COM%s_READY\\n' PARE";
    #[cfg(windows)]
    let command = "echo COMPARE_READY";
    let (output, legacy_elapsed) = legacy(command);
    assert!(output.lines().any(|line| line == "COMPARE_READY"));
    let start = Instant::now();
    let mut session =
        Session::open(&std::env::current_dir().unwrap(), 24, 100, Arc::new(|| {})).unwrap();
    session.send(format!("{command}\r").into_bytes()).unwrap();
    wait(&mut session, "COMPARE_READY");
    println!(
        "same command: old completion {legacy_elapsed:?}; new shell startup through output {:?}",
        start.elapsed()
    );
}

#[cfg(unix)]
#[test]
#[ignore = "uses SDL custom events; run with SDL_VIDEODRIVER=dummy and one test thread"]
fn experimental_terminal_comparison_new_backend_receives_input_where_old_stdin_is_closed() {
    let (output, _) =
        legacy("read answer && printf 'ANS:%s:END\\n' \"$answer\" || printf 'INPUT_%s\\n' CLOSED");
    assert!(output.lines().any(|line| line == "INPUT_CLOSED"));
    let mut session =
        Session::open(&std::env::current_dir().unwrap(), 24, 100, Arc::new(|| {})).unwrap();
    session
        .send(b"read answer; printf 'ANS:%s:END\\n' \"$answer\"\r".to_vec())
        .unwrap();
    session.send(b"received\r".to_vec()).unwrap();
    wait(&mut session, "ANS:received:END");
}
