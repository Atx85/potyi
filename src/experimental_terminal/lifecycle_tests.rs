// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn experimental_terminal_lifecycle_cancel_removes_packet_and_rejects_late_completion() {
    let mut lifecycle = Lifecycle::new(Shell::Posix, "", true).unwrap();
    lifecycle
        .prepare("printf CANCELLED", &std::env::temp_dir())
        .unwrap();
    let cancelled = lifecycle.current_id().unwrap();
    assert!(
        lifecycle
            .directory
            .join(format!("command-{cancelled}"))
            .exists()
    );
    assert!(
        lifecycle
            .directory
            .join(format!("directory-{cancelled}"))
            .exists()
    );
    lifecycle.cancel_start();
    assert!(!lifecycle.running());
    assert!(
        !lifecycle
            .directory
            .join(format!("command-{cancelled}"))
            .exists()
    );
    assert!(
        !lifecycle
            .directory
            .join(format!("directory-{cancelled}"))
            .exists()
    );
    lifecycle
        .prepare("printf NEXT", &std::env::temp_dir())
        .unwrap();
    let next = lifecycle.current_id().unwrap();
    assert!(next > cancelled);
    assert!(!lifecycle.finish(cancelled, 0, std::env::temp_dir()));
    assert!(!lifecycle.observe_end(cancelled));
    assert!(lifecycle.running());
    assert!(lifecycle.take_completion().is_none());
    assert!(lifecycle.finish(next, 23, std::env::temp_dir()));
    assert!(!lifecycle.release_after_seal());
    assert!(lifecycle.observe_end(next));
    assert!(lifecycle.release_after_seal());
    assert_eq!(lifecycle.take_completion().unwrap().status, 23);
}

#[test]
fn experimental_terminal_production_packet_cleanup_waits_for_metadata_and_output_seal() {
    for metadata_first in [true, false] {
        let mut lifecycle = Lifecycle::new(Shell::Posix, "", true).unwrap();
        lifecycle
            .prepare("printf HELLO", &std::env::temp_dir())
            .unwrap();
        let id = lifecycle.current_id().unwrap();
        let packet = lifecycle.directory.join(format!("command-{id}"));
        let directory_packet = lifecycle.directory.join(format!("directory-{id}"));
        let assert_retained = || {
            assert!(packet.is_file());
            assert!(directory_packet.is_file());
        };
        assert_retained();
        assert!(!lifecycle.finish(id + 1, 0, std::env::temp_dir()));
        assert!(!lifecycle.observe_end(id + 1));
        assert!(!lifecycle.release_after_seal());
        if metadata_first {
            assert!(lifecycle.finish(id, 5, std::env::temp_dir()));
        } else {
            assert!(lifecycle.observe_end(id));
        }
        assert!(!lifecycle.release_after_seal());
        assert_retained();
        if metadata_first {
            assert!(lifecycle.observe_end(id));
        } else {
            assert!(lifecycle.finish(id, 5, std::env::temp_dir()));
        }
        assert_retained();
        assert!(lifecycle.release_after_seal());
        assert!(!packet.exists());
        assert!(!directory_packet.exists());
        assert_eq!(lifecycle.take_completion().unwrap().status, 5);
        assert!(!lifecycle.release_after_seal());
    }
}

#[test]
fn experimental_terminal_production_closed_driver_removes_unfinished_packet() {
    let mut lifecycle = Lifecycle::new(Shell::Posix, "", true).unwrap();
    lifecycle
        .prepare("printf UNFINISHED", &std::env::temp_dir())
        .unwrap();
    let id = lifecycle.current_id().unwrap();
    let packet = lifecycle.directory.join(format!("command-{id}"));
    let directory_packet = lifecycle.directory.join(format!("directory-{id}"));
    assert!(packet.is_file());
    assert!(directory_packet.is_file());
    lifecycle.abort_after_closed();
    assert!(!packet.exists());
    assert!(!directory_packet.exists());
    assert!(!lifecycle.running());
    assert!(lifecycle.take_completion().is_none());
    assert!(!lifecycle.finish(id, 0, std::env::temp_dir()));
    assert!(!lifecycle.observe_end(id));
}

#[test]
fn experimental_terminal_production_cleanup_failure_bounds_retained_packets() {
    let mut lifecycle = Lifecycle::new(Shell::Posix, "", true).unwrap();
    lifecycle
        .prepare("printf ORIGINAL", &std::env::temp_dir())
        .unwrap();
    let id = lifecycle.current_id().unwrap();
    let packet = lifecycle.directory.join(format!("command-{id}"));
    // Replacing the file with a directory makes remove_file fail on all
    // platforms without depending on root privileges or permission bits.
    fs::remove_file(&packet).unwrap();
    fs::create_dir(&packet).unwrap();
    assert!(lifecycle.finish(id, 9, std::env::temp_dir()));
    assert!(lifecycle.observe_end(id));
    assert!(lifecycle.release_after_seal());
    assert_eq!(lifecycle.take_completion().unwrap().status, 9);
    assert_eq!(lifecycle.cleanup_pending, Some(id));
    assert!(
        lifecycle
            .prepare("printf NEXT", &std::env::temp_dir())
            .is_err()
    );
    assert_eq!(lifecycle.next_id, id);
    assert!(!lifecycle.running());
    assert!(
        !lifecycle
            .directory
            .join(format!("command-{}", id + 1))
            .exists()
    );
    assert!(
        !lifecycle
            .directory
            .join(format!("directory-{}", id + 1))
            .exists()
    );
    fs::remove_dir(&packet).unwrap();
    lifecycle
        .prepare("printf NEXT", &std::env::temp_dir())
        .unwrap();
    assert_eq!(lifecycle.cleanup_pending, None);
    assert_eq!(lifecycle.current_id(), Some(id + 1));
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
    };

    struct DriverFixture {
        lifecycle: Lifecycle,
        directory: PathBuf,
        helper: PathBuf,
    }
    struct DriverOutput {
        statuses: Vec<i32>,
        text: String,
        stderr: String,
    }
    impl DriverOutput {
        fn command_stdout(&self) -> String {
            let mut parts = self
                .text
                .strip_prefix("\x1b[2J\x1b[H\x1b]777;potyi-ready\x07")
                .expect("driver ready marker")
                .split("TEST_COMPLETE:");
            let mut output = parts.next().unwrap().to_owned();
            for part in parts {
                output.push_str(part.split_once('\n').unwrap().1);
            }
            output
        }
    }
    impl DriverFixture {
        fn new(kind: Shell, production: bool) -> Self {
            let lifecycle = Lifecycle::new(kind, "", production).unwrap();
            let directory = lifecycle.directory.join("cwd é ' with spaces");
            fs::create_dir(&directory).unwrap();
            let helper = lifecycle.directory.join("completion-helper");
            fs::write(&helper, "#!/bin/sh\n[ \"$4\" = complete ] || exit 91\nprintf 'TEST_COMPLETE:%s:%s\\n' \"$5\" \"$6\"\n").unwrap();
            fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
            Self {
                lifecycle,
                directory,
                helper,
            }
        }
        fn run(
            &mut self,
            shell: &Path,
            commands: &[&str],
            child_shell: Option<&Path>,
        ) -> DriverOutput {
            let mut packets = String::new();
            let mut previous_packet = None;
            for command in commands {
                self.lifecycle.prepare(command, &self.directory).unwrap();
                let id = self.lifecycle.current_id().unwrap();
                let packet = if self.lifecycle.shell == Shell::Fish {
                    format!("{}:{id}\0", self.lifecycle.nonce)
                } else {
                    format!("{}:{id}\n", self.lifecycle.nonce)
                };
                packets.push_str(&packet);
                // Replay the same packet while it can still exist on disk.
                // Neither the original rename path nor the production id
                // guard may run it a second time.
                packets.push_str(&packet);
                // Replay an older retained packet after a newer one completed,
                // modelling failed app cleanup as well as immediate duplicates.
                if let Some(previous) = previous_packet.replace(packet) {
                    packets.push_str(&previous);
                }
                // Prebuild all fixture packets without pretending the app
                // sealed their output: production now removes packets at that
                // real boundary. This pipe fixture drives the shell directly.
                self.lifecycle.running = None;
            }
            let mut command = Command::new(shell);
            match shell.file_name().and_then(|name| name.to_str()).unwrap() {
                "bash" => {
                    command.args(["--noprofile", "--norc", "-c"]);
                }
                "zsh" => {
                    command.args(["-f", "-c"]);
                }
                "fish" => {
                    command.args(["--no-config", "-c"]);
                }
                _ => {
                    command.arg("-c");
                }
            }
            let mut child = command
                .arg(self.lifecycle.launch())
                .current_dir(&self.directory)
                .env("POTYI_DRIVER_SHELL", child_shell.unwrap_or(shell))
                .env("POTYI_DRIVER_DIRECTORY", &self.lifecycle.directory)
                .env("POTYI_DRIVER_NONCE", &self.lifecycle.nonce)
                .env(
                    "POTYI_COMMAND_STDIN_NULL",
                    if self.lifecycle.production { "1" } else { "0" },
                )
                .env("POTYI_TERM_CLIENT", &self.helper)
                .env("POTYI_TERM_ADDRESS", "unused-test-address")
                .env("POTYI_TERM_TOKEN", "unused-test-token")
                .env("POTYI_CHILD_BASH_ENV_SET", "0")
                .env("POTYI_CHILD_ENV_SET", "0")
                .env("HOME", &self.directory)
                .env("ZDOTDIR", &self.directory)
                .env("XDG_CONFIG_HOME", &self.directory)
                .env_remove("POTYI_COMMAND_SHELL")
                .env_remove("POTYI_MUTATION")
                .env_remove("BASH_ENV")
                .env_remove("ENV")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(packets.as_bytes())
                .unwrap();
            let output = child.wait_with_output().unwrap();
            let text = String::from_utf8(output.stdout).unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            assert!(
                output.status.success(),
                "driver exited for {}: {text}\n{stderr}",
                shell.display()
            );
            let statuses = text
                .split("TEST_COMPLETE:")
                .skip(1)
                .map(|completion| {
                    completion
                        .split_once(':')
                        .unwrap()
                        .1
                        .lines()
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap()
                })
                .collect::<Vec<i32>>();
            assert_eq!(
                statuses.len(),
                commands.len(),
                "missing completion for {}: {text}\n{stderr}",
                shell.display()
            );
            DriverOutput {
                statuses,
                text,
                stderr,
            }
        }
    }
    fn available_shells() -> Vec<(PathBuf, Shell)> {
        let mut shells = vec![(PathBuf::from("/bin/sh"), Shell::Posix)];
        for name in ["bash", "zsh", "dash", "ksh", "mksh", "ash", "fish"] {
            for prefix in ["/bin", "/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"] {
                let path = Path::new(prefix).join(name);
                if path.is_file() {
                    shells.push((
                        path,
                        if name == "fish" {
                            Shell::Fish
                        } else {
                            Shell::Posix
                        },
                    ));
                    break;
                }
            }
        }
        shells
    }

    fn fresh_c(shell: &Path, directory: &Path, command: &str) -> std::process::Output {
        Command::new(shell)
            .arg("-c")
            .arg(command)
            .current_dir(directory)
            .env("HOME", directory)
            .env("ZDOTDIR", directory)
            .env("XDG_CONFIG_HOME", directory)
            .env_remove("BASH_ENV")
            .env_remove("ENV")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    #[test]
    fn experimental_terminal_production_syntax_exit_and_exec_failures_keep_dispatcher_usable() {
        for (shell, kind) in available_shells() {
            let mut fixture = DriverFixture::new(kind, true);
            let commands = [
                "printf 'unterminated",
                "exit 17",
                "exec /potyi-no-such-command",
                "printf 'RECOVERED\\n'",
            ];
            let references: Vec<_> = commands
                .iter()
                .map(|command| fresh_c(&shell, &fixture.directory, command))
                .collect();
            let output = fixture.run(&shell, &commands, None);
            // Some ksh versions accept an unmatched final quote under -c.
            // Match that shell's actual status and output, including nonzero
            // syntax statuses on shells that reject this command.
            assert_eq!(
                output.statuses,
                references
                    .iter()
                    .map(|reference| reference.status.code().unwrap())
                    .collect::<Vec<_>>(),
                "fresh -c statuses differ for {}: {}",
                shell.display(),
                output.stderr
            );
            assert_eq!(
                output.command_stdout().as_bytes(),
                references
                    .iter()
                    .flat_map(|reference| reference.stdout.iter().copied())
                    .collect::<Vec<_>>(),
                "fresh -c output differs for {}: {}",
                shell.display(),
                output.stderr
            );
            assert_eq!(
                output.statuses[1],
                17,
                "{}: {}",
                shell.display(),
                output.stderr
            );
            assert_ne!(
                output.statuses[2],
                0,
                "{}: {}",
                shell.display(),
                output.stderr
            );
            assert_eq!(
                output.statuses[3],
                0,
                "{}: {}",
                shell.display(),
                output.stderr
            );
            assert!(output.text.contains("RECOVERED"));
            assert!(!output.stderr.is_empty());
        }
    }

    #[test]
    fn experimental_terminal_production_keeps_fresh_state_cwd_and_eof_stdin() {
        for (shell, kind) in available_shells() {
            let mut fixture = DriverFixture::new(kind, true);
            let commands = if kind == Shell::Fish {
                [
                    "set -gx POTYI_MUTATION changed; builtin cd /; exit 7",
                    "if set -q POTYI_MUTATION; echo BAD_STATE; else; echo FRESH_STATE; end; pwd; if read supplied; echo BAD_STDIN; else; echo EOF_STDIN; end",
                ]
            } else {
                [
                    "export POTYI_MUTATION=changed; cd /; exit 7",
                    "printf 'FRESH_STATE:%s\\n' \"${POTYI_MUTATION-unset}\"; pwd; if read supplied; then printf 'BAD_STDIN\\n'; else printf 'EOF_STDIN\\n'; fi",
                ]
            };
            let output = fixture.run(&shell, &commands, None);
            assert_eq!(
                output.statuses,
                [7, 0],
                "{}: {}",
                shell.display(),
                output.stderr
            );
            assert!(output.text.contains("FRESH_STATE"));
            assert!(output.text.contains("EOF_STDIN"));
            assert!(!output.text.contains("BAD_STATE"));
            assert!(!output.text.contains("FRESH_STATE:changed"));
            assert!(!output.text.contains("BAD_STDIN"));
            assert!(output.text.contains(fixture.directory.to_str().unwrap()));
        }
    }

    #[test]
    fn experimental_terminal_production_late_syntax_failure_matches_fresh_c_child() {
        for (shell, kind) in available_shells() {
            let mut fixture = DriverFixture::new(kind, true);
            let broken = "printf 'BEFORE_LATE_ERROR\\n'\nprintf 'unterminated";
            let next = "printf 'AFTER_LATE_ERROR\\n'";
            let reference = fresh_c(&shell, &fixture.directory, broken);
            let next_reference = fresh_c(&shell, &fixture.directory, next);
            let output = fixture.run(&shell, &[broken, next], None);
            assert_eq!(
                output.statuses[0],
                reference.status.code().unwrap(),
                "late syntax status differs from original -c for {}: {}",
                shell.display(),
                output.stderr
            );
            assert_eq!(
                output.statuses[1],
                0,
                "{}: {}",
                shell.display(),
                output.stderr
            );
            assert_eq!(
                output.command_stdout().as_bytes(),
                [reference.stdout, next_reference.stdout].concat(),
                "late syntax output differs from original -c for {}: {}",
                shell.display(),
                output.stderr
            );
            assert!(output.text.contains("AFTER_LATE_ERROR"));
        }
    }

    #[test]
    fn experimental_terminal_production_preserves_exact_command_text_for_fresh_c() {
        for (shell, kind) in available_shells() {
            if kind != Shell::Posix {
                continue;
            }
            for command in [
                "printf '%s' 'space 東京'",
                "printf '%s' trailing\\",
                "printf '%s' slash\\\n",
                "printf 'newline\\n'\n\n",
                "printf '%s' comment; # final comment",
                "printf 'BEFORE_LATE_ERROR\\n'\nprintf 'unterminated",
                "printf '%s' 'quoted\n\n'\n\n",
                "return 7; printf 'AFTER_RETURN\\n'",
            ] {
                let mut fixture = DriverFixture::new(kind, true);
                let reference = fresh_c(&shell, &fixture.directory, command);
                let output = fixture.run(&shell, &[command], None);
                assert_eq!(
                    output.statuses,
                    [reference.status.code().unwrap()],
                    "status differs for {} and {command:?}: {}",
                    shell.display(),
                    output.stderr
                );
                assert_eq!(
                    output.command_stdout().as_bytes(),
                    reference.stdout,
                    "output differs for {} and {command:?}: {}",
                    shell.display(),
                    output.stderr
                );
            }
        }
    }

    #[test]
    fn experimental_terminal_raw_preflight_rejects_invalid_body_before_side_effects() {
        for (shell, kind) in available_shells() {
            let mut fixture = DriverFixture::new(kind, false);
            let output = fixture.run(
                &shell,
                &[
                    "printf 'MUST_NOT_RUN\\n'\nprintf 'unterminated",
                    "printf 'AFTER_ERROR\\n'",
                ],
                None,
            );
            assert_ne!(output.statuses[0], 0);
            assert_eq!(output.statuses[1], 0);
            assert!(!output.text.contains("MUST_NOT_RUN"));
            assert!(output.text.contains("AFTER_ERROR"));
        }
    }

    #[test]
    fn experimental_terminal_production_rejects_replayed_ids_above_shell_signed_range() {
        for (shell, kind) in available_shells() {
            if kind != Shell::Posix {
                continue;
            }
            let mut fixture = DriverFixture::new(kind, true);
            fixture.lifecycle.next_id = i64::MAX as u64;
            let output = fixture.run(&shell, &["printf ONE", "printf TWO", "printf THREE"], None);
            assert_eq!(output.statuses, [0, 0, 0]);
            assert_eq!(output.command_stdout(), "ONETWOTHREE");
        }
    }

    #[test]
    fn experimental_terminal_production_child_launch_failure_keeps_next_command_available() {
        let mut fixture = DriverFixture::new(Shell::Posix, true);
        let wrapper = fixture.lifecycle.directory.join("child-shell");
        let flag = fixture.lifecycle.directory.join("first-launch");
        let quoted_flag = quote_posix(flag.to_str().unwrap());
        // Model a child failing before it parses the body. A check-only launch
        // would consume this first failure and hide which process owns it.
        fs::write(&wrapper, format!("#!/bin/sh\nif [ ! -e {quoted_flag} ]; then : > {quoted_flag}; exit 126; fi\nexec /bin/sh \"$@\"\n")).unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
        let output = fixture.run(
            Path::new("/bin/sh"),
            &["printf MUST_NOT_RUN", "printf 'NEXT_CHILD_READY\\n'"],
            Some(&wrapper),
        );
        assert_eq!(output.statuses, [126, 0]);
        assert!(!output.text.contains("MUST_NOT_RUN"));
        assert!(output.text.contains("NEXT_CHILD_READY"));
    }
}
