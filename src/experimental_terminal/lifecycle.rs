// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! File-based managed commands with explicit, authenticated completion boundaries.
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
#[derive(Debug, Clone)]
pub(crate) struct Completion {
    pub(crate) id: u64,
    pub(crate) status: i32,
    pub(crate) directory: PathBuf,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shell {
    Posix,
    Fish,
    Windows,
}
pub(super) struct Lifecycle {
    directory: PathBuf,
    shell: Shell,
    production: bool,
    nonce: String,
    next_id: u64,
    running: Option<u64>,
    metadata: Option<Completion>,
    end_seen: bool,
    completion: Option<Completion>,
    cleanup_pending: Option<u64>,
}
impl Lifecycle {
    pub(super) fn new(shell: Shell, helpers: &str, production: bool) -> io::Result<Self> {
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random).map_err(|e| io::Error::other(e.to_string()))?;
        let name: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let directory = std::env::temp_dir().join(format!("potyi-managed-{name}"));
        #[allow(unused_mut)]
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        let lifecycle = Self {
            directory,
            shell,
            production,
            nonce: name,
            next_id: 0,
            running: None,
            metadata: None,
            end_seen: false,
            completion: None,
            cleanup_pending: None,
        };
        lifecycle.write_file(lifecycle.driver_filename(), &lifecycle.driver(helpers))?;
        if production && shell != Shell::Windows {
            lifecycle.write_file("helpers", helpers)?;
        }
        if shell == Shell::Windows {
            for operation in ["edit", "view"] {
                lifecycle.write_file(&format!("{operation}.cmd"),&format!("@echo off\r\n\"%POTYI_TERM_CLIENT%\" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% {operation} %*\r\n"))?;
            }
        }
        Ok(lifecycle)
    }
    pub(super) fn nonce(&self) -> &str {
        &self.nonce
    }
    pub(super) fn directory(&self) -> &Path {
        &self.directory
    }
    pub(super) fn launch(&self) -> String {
        match self.shell {
            Shell::Posix => format!(
                ". {}",
                quote_posix(
                    &self
                        .directory
                        .join(self.driver_filename())
                        .to_string_lossy()
                )
            ),
            Shell::Fish => format!(
                "source {}",
                quote_posix(
                    &self
                        .directory
                        .join(self.driver_filename())
                        .to_string_lossy()
                )
            ),
            Shell::Windows => format!(
                "\"{}\"",
                self.directory.join(self.driver_filename()).display()
            ),
        }
    }
    pub(super) fn running(&self) -> bool {
        self.running.is_some()
    }
    pub(super) fn current_id(&self) -> Option<u64> {
        self.running
    }
    #[cfg(unix)]
    pub(super) fn resume_after(&mut self, id: u64) {
        self.next_id = self.next_id.max(id);
    }
    #[cfg(windows)]
    pub(super) fn windows_launcher(&self) -> io::Result<String> {
        let id = self
            .running
            .ok_or_else(|| io::Error::other("No command is prepared"))?;
        let filename = format!("launch-{id}.cmd");
        // Keep the launcher ASCII. A hidden real console supplies a codepage;
        // switch it before the nested shell emits Unicode. Expand the private
        // command environment once, without CALL's second percent expansion,
        // so the nested /C retains original interactive cmd expression syntax.
        self.write_file(&filename, &format!(
            "@echo off\r\nchcp 65001 >NUL\r\nif errorlevel 1 exit /b %ERRORLEVEL%\r\ncd /D \"%POTYI_COMMAND_DIRECTORY%\"\r\nif errorlevel 1 exit /b %ERRORLEVEL%\r\n\"%POTYI_DRIVER_SHELL%\" /D /Q /V:OFF /S /C \"%POTYI_COMMAND_TEXT%\"\r\nexit /b %ERRORLEVEL%\r\n"
        ))?;
        Ok(filename)
    }
    #[cfg(windows)]
    pub(super) fn complete_pipes(&mut self, status: i32, launch_directory: PathBuf) {
        let Some(id) = self.running else {
            return;
        };
        self.running = None;
        self.metadata = None;
        self.end_seen = false;
        // Each command is a fresh process, like the original Windows :term.
        // Its internal cd never changes the native browser's directory.
        self.completion = Some(Completion {
            id,
            status,
            directory: launch_directory,
        });
        let _ = fs::remove_file(self.directory.join(self.command_filename(id)));
        let _ = fs::remove_file(self.directory.join(format!("launch-{id}.cmd")));
    }
    pub(super) fn prepare(&mut self, command: &str, directory: &Path) -> io::Result<Vec<u8>> {
        if self.running() {
            return Err(io::Error::other("A command is already running"));
        }
        if let Some(id) = self.cleanup_pending {
            self.remove_packet(id)?;
            self.cleanup_pending = None;
        }
        if command.len() > 64 * 1024 || command.as_bytes().contains(&0) {
            return Err(io::Error::other("Command exceeds 64 KiB or contains NUL"));
        }
        if !directory.is_dir() {
            return Err(io::Error::other("The command directory is unavailable"));
        }
        let directory = directory.canonicalize()?;
        let path = directory
            .to_str()
            .ok_or_else(|| io::Error::other("Command directory is not valid Unicode"))?;
        let directory_body = match self.shell {
            Shell::Posix => format!(
                "case \"$POTYI_DRIVER_SHELL\" in *zsh|*bash) builtin cd {0} ;; *) command cd {0} ;; esac || return $?\n",
                quote_posix(path)
            ),
            Shell::Fish => format!("builtin cd {}; or return $status\n", quote_posix(path)),
            Shell::Windows => format!(
                "@echo off\r\ncd /D \"{}\"\r\nif errorlevel 1 exit /b %ERRORLEVEL%\r\n",
                windows_directory(path).replace('%', "%%")
            ),
        };
        let id = self.next_id.checked_add(1).ok_or_else(|| {
            io::Error::other("Command IDs are exhausted; open a new terminal session")
        })?;
        let body = if self.production && self.shell != Shell::Windows {
            self.write_file(&format!("directory-{id}"), &directory_body)?;
            if self.shell == Shell::Posix {
                // The production child receives the exact original -c text,
                // including trailing newlines or a final backslash.
                command.to_owned()
            } else {
                format!("{command}\n")
            }
        } else {
            format!("{directory_body}{command}\n")
        };
        self.write_file(&self.command_filename(id), &body)?;
        self.next_id = id;
        self.running = Some(id);
        self.metadata = None;
        self.end_seen = false;
        self.completion = None;
        Ok(if self.shell == Shell::Fish {
            format!("\0{}:{}\0", self.nonce, self.next_id).into_bytes()
        } else {
            format!("\r{}:{}\r", self.nonce, self.next_id).into_bytes()
        })
    }
    pub(super) fn cancel_start(&mut self) {
        if let Some(id) = self.running.take() {
            if self.production && self.shell == Shell::Posix {
                self.cleanup_pending = self.remove_packet(id).err().map(|_| id);
            } else {
                let _ = fs::remove_file(self.directory.join(self.command_filename(id)));
                let _ = fs::remove_file(self.directory.join(format!("directory-{id}")));
            }
            #[cfg(windows)]
            let _ = fs::remove_file(self.directory.join(format!("launch-{id}.cmd")));
        }
        self.metadata = None;
        self.end_seen = false;
    }
    /// Authentication proves the status/cwd, but not that the PTY output has
    /// reached the parser. Keep stdin owned by the command until the in-band
    /// boundary has also arrived and its final screen has been sealed.
    pub(super) fn finish(&mut self, id: u64, status: i32, directory: PathBuf) -> bool {
        if self.running != Some(id) || self.metadata.is_some() {
            return false;
        }
        self.metadata = Some(Completion {
            id,
            status,
            directory,
        });
        true
    }
    pub(super) fn observe_end(&mut self, id: u64) -> bool {
        if self.running != Some(id) || self.end_seen {
            return false;
        }
        self.end_seen = true;
        true
    }
    pub(super) fn metadata(&self) -> Option<&Completion> {
        self.metadata.as_ref()
    }
    pub(super) fn paired(&self) -> bool {
        self.end_seen && self.metadata.is_some()
    }
    pub(super) fn release_after_seal(&mut self) -> bool {
        if !self.paired() {
            return false;
        }
        if self.production && self.shell == Shell::Posix {
            let id = self.running.unwrap();
            self.cleanup_pending = self.remove_packet(id).err().map(|_| id);
        }
        self.running = None;
        self.completion = self.metadata.take();
        self.end_seen = false;
        true
    }
    pub(super) fn abort_after_closed(&mut self) {
        if self.production
            && self.shell == Shell::Posix
            && let Some(id) = self.running
        {
            self.cleanup_pending = self.remove_packet(id).err().map(|_| id);
        }
        self.running = None;
        self.metadata = None;
        self.end_seen = false;
        self.completion = None;
    }
    pub(super) fn take_completion(&mut self) -> Option<Completion> {
        self.completion.take()
    }
    fn remove_packet(&self, id: u64) -> io::Result<()> {
        // POSIX production leaves numbered packets for the owner to remove
        // after the authenticated completion and ordered output seal, or once
        // the driver has closed. No extra shell process is needed per command.
        let remove = |path| match fs::remove_file(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        };
        let command = remove(self.directory.join(self.command_filename(id)));
        let directory = remove(self.directory.join(format!("directory-{id}")));
        command.and(directory)
    }
    fn driver_filename(&self) -> &str {
        if self.shell == Shell::Windows {
            "driver.cmd"
        } else {
            "driver"
        }
    }
    fn command_filename(&self, id: u64) -> String {
        if self.shell == Shell::Windows {
            format!("command-{id}.cmd")
        } else {
            format!("command-{id}")
        }
    }
    fn write_file(&self, name: &str, body: &str) -> io::Result<()> {
        let path = self.directory.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)?.write_all(body.as_bytes())
    }
    fn driver(&self, helpers: &str) -> String {
        match self.shell {
            Shell::Posix=>format!(r#"PS1= PS2= PS3= PS4=
{helpers}
__potyi_complete() {{
    command -p stty sane -echo 2>/dev/null
    "$POTYI_TERM_CLIENT" --term-request "$POTYI_TERM_ADDRESS" "$POTYI_TERM_TOKEN" complete "$__potyi_id" "$1" "{nonce}"
}}
trap ':' INT
command -p stty sane -echo 2>/dev/null
printf '\033[2J\033[H\033]777;potyi-ready\007'
__potyi_last_id=
__potyi_fresh_id() {{
    # Compare bounded decimal IDs without shell signed-integer overflow.
    # Retained files after a cleanup error must never allow an older replay.
    [ "${{#__potyi_id}}" -gt "${{#__potyi_last_id}}" ] && return 0
    [ "${{#__potyi_id}}" -lt "${{#__potyi_last_id}}" ] && return 1
    __potyi_compare=$__potyi_id
    __potyi_previous=$__potyi_last_id
    while [ -n "$__potyi_compare" ]; do
        __potyi_digit=${{__potyi_compare%"${{__potyi_compare#?}}"}}
        __potyi_previous_digit=${{__potyi_previous%"${{__potyi_previous#?}}"}}
        [ "$__potyi_digit" -gt "$__potyi_previous_digit" ] && return 0
        [ "$__potyi_digit" -lt "$__potyi_previous_digit" ] && return 1
        __potyi_compare=${{__potyi_compare#?}}
        __potyi_previous=${{__potyi_previous#?}}
    done
    return 1
}}
while IFS= read -r __potyi_dispatch; do
    case "$__potyi_dispatch" in "$POTYI_DRIVER_NONCE":*) __potyi_id=${{__potyi_dispatch#*:}} ;; *) continue ;; esac
    case "$__potyi_id" in ''|*[!0-9]*) continue ;; esac
    [ -f "$POTYI_DRIVER_DIRECTORY/command-$__potyi_id" ] || continue
    if [ "$POTYI_COMMAND_STDIN_NULL" = 1 ]; then
        # The app owns packet cleanup after the authenticated output seal. A
        # duplicate dispatch cannot rerun a packet while cleanup is pending.
        __potyi_fresh_id || continue
        __potyi_last_id=$__potyi_id
        __potyi_command_file="$POTYI_DRIVER_DIRECTORY/command-$__potyi_id"
    else
        command -p mv "$POTYI_DRIVER_DIRECTORY/command-$__potyi_id" "$POTYI_DRIVER_DIRECTORY/running-command" || exit $?
        __potyi_command_file="$POTYI_DRIVER_DIRECTORY/running-command"
    fi
    __potyi_status=0
    # Production uses a fresh child, whose syntax/exit failure cannot kill the
    # dispatcher. Only raw mode sources the body here and needs preflight.
    if [ "$POTYI_COMMAND_STDIN_NULL" != 1 ]; then
        (
            if [ -n "$POTYI_COMMAND_SHELL" ]; then
                :
            else
                case "$POTYI_DRIVER_SHELL" in
                    *zsh) "$POTYI_DRIVER_SHELL" -f -n "$POTYI_DRIVER_DIRECTORY/running-command" ;;
                    *bash) "$POTYI_DRIVER_SHELL" --noprofile --norc -n "$POTYI_DRIVER_DIRECTORY/running-command" ;;
                    *) "$POTYI_DRIVER_SHELL" -n "$POTYI_DRIVER_DIRECTORY/running-command" ;;
                esac
            fi
        )
        __potyi_status=$?
    fi
    if [ "$__potyi_status" -eq 0 ]; then
        command -p stty sane 2>/dev/null
        if [ "$POTYI_COMMAND_STDIN_NULL" = 1 ]; then
            . "$POTYI_DRIVER_DIRECTORY/directory-$__potyi_id"
            __potyi_status=$?
            if [ "$__potyi_status" -eq 0 ]; then
                (
                    if [ "$POTYI_CHILD_BASH_ENV_SET" = 1 ]; then
                        BASH_ENV=$POTYI_CHILD_BASH_ENV; export BASH_ENV
                    else unset BASH_ENV; fi
                    if [ "$POTYI_CHILD_ENV_SET" = 1 ]; then
                        ENV=$POTYI_CHILD_ENV; export ENV
                    else unset ENV; fi
                    # A sentinel retains all trailing newlines through command
                    # substitution. -c parses the original body as one command
                    # string; sourcing a file would let zsh execute a prefix
                    # before a later syntax error that original :term rejects.
                    __potyi_command=$(command -p cat "$__potyi_command_file" && printf '%s' .) || exit $?
                    __potyi_command=${{__potyi_command%.}}
                    if [ -n "$POTYI_COMMAND_SHELL" ]; then
                        exec "$POTYI_COMMAND_SHELL" -c "$__potyi_command"
                    else
                        exec "$POTYI_DRIVER_SHELL" -c '. "$POTYI_DRIVER_DIRECTORY/helpers"
'"$__potyi_command"
                    fi
                ) < /dev/null
                __potyi_status=$?
            fi
        else
            . "$POTYI_DRIVER_DIRECTORY/running-command"
            __potyi_status=$?
        fi
    fi
    if [ "$POTYI_COMMAND_STDIN_NULL" != 1 ]; then
        command -p rm -f "$POTYI_DRIVER_DIRECTORY/running-command" "$POTYI_DRIVER_DIRECTORY/directory-$__potyi_id" || exit $?
    fi
    __potyi_complete "$__potyi_status" || exit $?
done
"#, nonce = self.nonce),
            Shell::Fish=>format!(r#"set -g __potyi_dispatch_stty (command --search stty)
set -g __potyi_dispatch_mv (command --search mv)
set -g __potyi_dispatch_rm (command --search rm)
test -n "$__potyi_dispatch_stty"; or exit 127
test -n "$__potyi_dispatch_mv"; or exit 127
test -n "$__potyi_dispatch_rm"; or exit 127
{helpers}
function __potyi_complete --argument-names exit_status
    "$__potyi_dispatch_stty" sane -echo -icanon min 1 time 0 2>/dev/null
    "$POTYI_TERM_CLIENT" --term-request "$POTYI_TERM_ADDRESS" "$POTYI_TERM_TOKEN" complete "$__potyi_id" "$exit_status" "{nonce}"
end
function __potyi_interrupt --on-signal INT
end
"$__potyi_dispatch_stty" sane -echo -icanon min 1 time 0 2>/dev/null
printf '\033[2J\033[H\033]777;potyi-ready\007'
while read --null --nchars 64 --global __potyi_dispatch
    set __potyi_parts (string split --max 1 ':' -- "$__potyi_dispatch")
    test (count $__potyi_parts) -eq 2; or continue
    test "$__potyi_parts[1]" = "$POTYI_DRIVER_NONCE"; or continue
    set -g __potyi_id "$__potyi_parts[2]"
    string match -qr '^[0-9]+$' -- "$__potyi_id"; or continue
    test -f "$POTYI_DRIVER_DIRECTORY/command-$__potyi_id"; or continue
    "$__potyi_dispatch_mv" "$POTYI_DRIVER_DIRECTORY/command-$__potyi_id" "$POTYI_DRIVER_DIRECTORY/running-command"; or exit $status
    # Fish retains its existing sourced-body/preflight contract. The POSIX
    # fresh -c optimization is enabled only where exact-byte parity is tested.
    "$POTYI_DRIVER_SHELL" --no-config --no-execute "$POTYI_DRIVER_DIRECTORY/running-command"
    set __potyi_status $status
    if test $__potyi_status -eq 0
        "$__potyi_dispatch_stty" sane 2>/dev/null
        if test "$POTYI_COMMAND_STDIN_NULL" = 1
            source "$POTYI_DRIVER_DIRECTORY/directory-$__potyi_id"
            set __potyi_status $status
            if test $__potyi_status -eq 0
                "$POTYI_DRIVER_SHELL" -c 'source "$POTYI_DRIVER_DIRECTORY/helpers"; source "$POTYI_DRIVER_DIRECTORY/running-command"' < /dev/null
                set __potyi_status $status
            end
        else
            source "$POTYI_DRIVER_DIRECTORY/running-command"
            set __potyi_status $status
        end
    end
    "$__potyi_dispatch_rm" -f "$POTYI_DRIVER_DIRECTORY/running-command" "$POTYI_DRIVER_DIRECTORY/directory-$__potyi_id"; or exit $status
    __potyi_complete $__potyi_status; or exit $status
end
"#, nonce = self.nonce),
            Shell::Windows=>format!(r#"@echo off
{helpers}
"%POTYI_TERM_CLIENT%" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% ready
:potyi_next
set "POTYI_COMMAND_DISPATCH="
set "POTYI_COMMAND_ID="
for /f "delims=" %%I in ('"%POTYI_TERM_CLIENT%" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% read-command') do set "POTYI_COMMAND_DISPATCH=%%I"
if not defined POTYI_COMMAND_DISPATCH exit /b 1
for /f "tokens=1,2 delims=:" %%A in ("%POTYI_COMMAND_DISPATCH%") do if "%%A"=="%POTYI_DRIVER_NONCE%" set "POTYI_COMMAND_ID=%%B"
if not defined POTYI_COMMAND_ID goto potyi_next
if not exist "%POTYI_DRIVER_DIRECTORY%\command-%POTYI_COMMAND_ID%.cmd" goto potyi_next
move /Y "%POTYI_DRIVER_DIRECTORY%\command-%POTYI_COMMAND_ID%.cmd" "%POTYI_DRIVER_DIRECTORY%\running-command.cmd" >NUL
if errorlevel 1 exit /b %ERRORLEVEL%
call "%POTYI_DRIVER_DIRECTORY%\running-command.cmd"
set "POTYI_COMMAND_STATUS=%ERRORLEVEL%"
del /Q "%POTYI_DRIVER_DIRECTORY%\running-command.cmd" >NUL 2>&1
"%POTYI_TERM_CLIENT%" --term-request %POTYI_TERM_ADDRESS% %POTYI_TERM_TOKEN% complete %POTYI_COMMAND_ID% %POTYI_COMMAND_STATUS% {nonce}
if errorlevel 1 exit /b %ERRORLEVEL%
goto potyi_next
"#, nonce = self.nonce).replace('\n',"\r\n"),
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod dispatch_tests;
impl Drop for Lifecycle {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
fn quote_posix(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}
pub(super) fn windows_directory(path: &str) -> String {
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_rejects_overlapping_and_stale_completions_and_cleans_private_files() {
        let mut lifecycle = Lifecycle::new(Shell::Posix, "", false).unwrap();
        let directory = lifecycle.directory.clone();
        assert!(
            lifecycle
                .prepare(
                    "printf '%s\\n' 'quoted'; # final comment",
                    &std::env::temp_dir()
                )
                .is_ok()
        );
        assert!(
            lifecycle
                .prepare("echo too early", &std::env::temp_dir())
                .is_err()
        );
        assert!(!lifecycle.finish(7, 0, "/tmp".into()));
        assert!(lifecycle.running());
        assert!(lifecycle.finish(1, 17, "/tmp".into()));
        assert!(!lifecycle.finish(1, 0, "/tmp".into()));
        assert!(lifecycle.running());
        assert!(lifecycle.take_completion().is_none());
        assert!(!lifecycle.release_after_seal());
        assert!(!lifecycle.observe_end(7));
        assert!(lifecycle.observe_end(1));
        assert!(!lifecycle.observe_end(1));
        assert!(lifecycle.release_after_seal());
        assert_eq!(lifecycle.take_completion().unwrap().status, 17);
        drop(lifecycle);
        assert!(!directory.exists());
    }
}
