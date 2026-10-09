// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Hidden, noninteractive Windows commands with ordinary ordered pipes.
//!
//! ConPTY may reorder custom OSC controls before text. Completion here instead
//! waits for the real child and both pipe EOFs; no timing or terminal markers.
use portable_pty::ChildKiller;
use std::{
    cmp::Ordering,
    ffi::{OsStr, OsString},
    fs::File,
    io,
    mem::{self, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    process::Command,
    ptr,
    sync::Arc,
};
use winapi::{
    shared::minwindef::{FALSE, TRUE},
    um::{
        fileapi::{CreateFileW, OPEN_EXISTING},
        handleapi::{INVALID_HANDLE_VALUE, SetHandleInformation},
        jobapi2::{
            AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject, TerminateJobObject,
        },
        namedpipeapi::CreatePipe,
        processthreadsapi::{
            CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
            InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
            ResumeThread, TerminateProcess, UpdateProcThreadAttribute,
        },
        stringapiset::CompareStringOrdinal,
        synchapi::WaitForSingleObject,
        winbase::{
            CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
            EXTENDED_STARTUPINFO_PRESENT, HANDLE_FLAG_INHERIT, INFINITE, STARTF_USESHOWWINDOW,
            STARTF_USESTDHANDLES, STARTUPINFOEXW,
        },
        winnt::{
            FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, HANDLE,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectExtendedLimitInformation,
        },
        winuser::SW_HIDE,
    },
};

const HANDLE_LIST: usize = 0x0002_0002;
// Stay within the conservative Windows process environment ceiling. Native
// cmd also has a smaller per-line limit; do not truncate either input.
const MAX_ENVIRONMENT_UNITS: usize = 32_767;

/// CMD treats a canonical `\\?\C:\...` cwd as UNC and silently falls back to
/// the Windows directory. Translate at the process boundary only: browser,
/// transcript and completion paths must keep their authoritative spelling.
pub(super) fn cmd_directory(directory: &Path) -> io::Result<PathBuf> {
    let units: Vec<u16> = directory.as_os_str().encode_wide().collect();
    let verbatim = units.starts_with(&[92, 92, 63, 92]);
    let local = if verbatim { &units[4..] } else { &units[..] };
    if local.len() < 3
        || !matches!(local[0], 65..=90 | 97..=122)
        || local[1] != 58
        || local[2] != 92
    {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "CMD commands require a local drive directory; UNC and device paths are unsupported",
        ));
    }
    // CreateProcess and CMD's cwd handling still impose the ordinary Win32
    // path ceiling. Never remove the prefix when doing so can rename a path.
    if local.len() >= 260
        || local.contains(&0)
        || local[3..].split(|unit| *unit == 92).any(|part| {
            part.last().is_some_and(|unit| matches!(unit, 32 | 46))
                || part.iter().any(|unit| {
                    *unit < 32 || matches!(*unit, 34 | 42 | 47 | 58 | 60 | 62 | 63 | 124)
                })
                || dos_device_component(part)
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "CMD cannot use this exact Windows directory; choose a shorter local path without extended-only names",
        ));
    }
    Ok(PathBuf::from(OsString::from_wide(local)))
}

fn dos_device_component(component: &[u16]) -> bool {
    let stem = component
        .split(|unit| *unit == 46)
        .next()
        .unwrap_or_default();
    let upper = |unit: u16| match unit {
        97..=122 => unit - 32,
        _ => unit,
    };
    let matches = |name: &[u8]| {
        stem.len() == name.len()
            && stem
                .iter()
                .zip(name)
                .all(|(a, b)| upper(*a) == u16::from(*b))
    };
    matches(b"CON")
        || matches(b"PRN")
        || matches(b"AUX")
        || matches(b"NUL")
        || (stem.len() == 4
            && matches!(stem[3], 49..=57 | 0x00b9 | 0x00b2 | 0x00b3)
            && (stem[..3].iter().copied().map(upper).eq([67, 79, 77])
                || stem[..3].iter().copied().map(upper).eq([76, 80, 84])))
}

#[derive(Debug, Clone)]
struct Killer {
    job: Arc<OwnedHandle>,
}
impl ChildKiller for Killer {
    fn kill(&mut self) -> io::Result<()> {
        // The job owns the command and every descendant. Stop/drop cannot
        // strand grandchildren holding an inherited output pipe open.
        check(unsafe { TerminateJobObject(handle(&self.job), 130) })
    }
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(self.clone())
    }
}

pub(super) struct Child {
    process: OwnedHandle,
    job: Arc<OwnedHandle>,
    pub(super) stdout: Option<File>,
    pub(super) stderr: Option<File>,
}
impl Child {
    pub(super) fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(Killer {
            job: self.job.clone(),
        })
    }
    pub(super) fn wait(self) -> io::Result<i32> {
        if unsafe { WaitForSingleObject(handle(&self.process), INFINITE) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        let mut status = 0;
        let result = check(unsafe { GetExitCodeProcess(handle(&self.process), &mut status) });
        // A background descendant must not keep this completed command's
        // pipes open forever. Normal command completion ends its whole job.
        unsafe {
            TerminateJobObject(handle(&self.job), 130);
        }
        result.map(|()| status as i32)
    }
    pub(super) fn kill(&mut self) -> io::Result<()> {
        check(unsafe { TerminateJobObject(handle(&self.job), 130) })
    }
    pub(super) fn try_wait(&mut self) -> io::Result<Option<i32>> {
        match unsafe { WaitForSingleObject(handle(&self.process), 0) } {
            258 => Ok(None),
            0 => {
                let mut status = 0;
                check(unsafe { GetExitCodeProcess(handle(&self.process), &mut status) })?;
                // Finish daemons/producers after the main process exits so no
                // inherited writer can strand the structured output worker.
                unsafe {
                    TerminateJobObject(handle(&self.job), 130);
                }
                Ok(Some(status as i32))
            }
            _ => Err(io::Error::last_os_error()),
        }
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        // Windows process handles are reaped by the kernel when closed. The
        // last job owner kills descendants even on reader/worker setup errors.
        unsafe {
            TerminateJobObject(handle(&self.job), 130);
        }
    }
}

pub(super) fn spawn(
    shell: &OsStr,
    launcher: &str,
    directory: &Path,
    variables: &[(String, String)],
) -> io::Result<Child> {
    let cmd_directory = cmd_directory(directory)?;
    if launcher
        .bytes()
        .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, b'-' | b'.'))
    {
        return Err(io::Error::other("Invalid private command launcher"));
    }
    let mut command_text = OsString::from("\"");
    command_text.push(shell);
    command_text.push(format!(
        "\" /D /Q /V:OFF /S /C \"\"%POTYI_DRIVER_DIRECTORY%\\{launcher}\"\""
    ));
    let variables: Vec<_> = variables
        .iter()
        .map(|(key, value)| (OsString::from(key), Some(OsString::from(value))))
        .collect();
    spawn_native(
        Some(shell),
        &command_text,
        &cmd_directory,
        &variables,
        CREATE_NEW_CONSOLE,
    )
}

#[cfg(test)]
#[path = "windows_command_tests.rs"]
mod tests;

/// Restricted to structured Git's inherited environment plus explicit
/// overrides/removals and null stdin/piped outputs. std::Command does not expose
/// its env_clear bit, so this helper does not promise generic Command cloning.
pub(super) fn spawn_process(command: &Command) -> io::Result<Child> {
    let mut words = Vec::new();
    append_argument(&mut words, command.get_program())?;
    for argument in command.get_args() {
        words.push(b' ' as u16);
        append_argument(&mut words, argument)?;
    }
    let command_line = OsString::from_wide(&words);
    let current = std::env::current_dir()?;
    let directory = command.get_current_dir().unwrap_or(&current);
    let variables: Vec<_> = command
        .get_envs()
        .map(|(key, value)| (key.to_os_string(), value.map(OsStr::to_os_string)))
        .collect();
    // Null application name preserves Windows executable lookup for git/git.exe
    // on PATH. The first command-line argument is independently CRT-quoted.
    spawn_native(None, &command_line, directory, &variables, CREATE_NO_WINDOW)
}

fn spawn_native(
    application: Option<&OsStr>,
    command_text: &OsStr,
    directory: &Path,
    variables: &[(OsString, Option<OsString>)],
    console_flags: u32,
) -> io::Result<Child> {
    let (stdout, stdout_write) = pipe()?;
    let (stderr, stderr_write) = pipe()?;
    let mut attributes: winapi::um::minwinbase::SECURITY_ATTRIBUTES = unsafe { mem::zeroed() };
    attributes.nLength = size_of::<winapi::um::minwinbase::SECURITY_ATTRIBUTES>() as u32;
    attributes.bInheritHandle = TRUE;
    let nul = wide(OsStr::new("NUL"))?;
    let stdin = owned(unsafe {
        CreateFileW(
            nul.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &mut attributes,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            ptr::null_mut(),
        )
    })?;
    let handles = [handle(&stdin), handle(&stdout_write), handle(&stderr_write)];
    let mut attributes = Attributes::new(&handles)?;
    let job = Arc::new(owned(unsafe {
        CreateJobObjectW(ptr::null_mut(), ptr::null())
    })?);
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    check(unsafe {
        SetInformationJobObject(
            handle(&job),
            JobObjectExtendedLimitInformation,
            &mut limits as *mut _ as *mut _,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    })?;
    let application = application.map(wide).transpose()?;
    let mut command = wide(command_text)?;
    if command.len() > 32_767 {
        return Err(io::Error::other(
            "Windows command line exceeds its native UTF-16 limit",
        ));
    }
    let cwd = wide(directory.as_os_str())?;
    let mut environment = environment(variables)?;
    let mut startup: STARTUPINFOEXW = unsafe { mem::zeroed() };
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
    startup.StartupInfo.wShowWindow = SW_HIDE as u16;
    startup.StartupInfo.hStdInput = handle(&stdin);
    startup.StartupInfo.hStdOutput = handle(&stdout_write);
    startup.StartupInfo.hStdError = handle(&stderr_write);
    startup.lpAttributeList = attributes.pointer();
    let mut information: PROCESS_INFORMATION = unsafe { mem::zeroed() };
    check(unsafe {
        CreateProcessW(
            application
                .as_ref()
                .map_or(ptr::null(), |application| application.as_ptr()),
            command.as_mut_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            TRUE,
            console_flags
                | CREATE_SUSPENDED
                | CREATE_UNICODE_ENVIRONMENT
                | EXTENDED_STARTUPINFO_PRESENT,
            environment.as_mut_ptr() as *mut _,
            cwd.as_ptr(),
            &mut startup.StartupInfo,
            &mut information,
        )
    })?;
    // CreateProcess returned two independently owned handles. Adopt both
    // before any fallible assignment/resume step so every path closes them.
    let process = owned(information.hProcess)?;
    let thread = owned(information.hThread)?;
    if let Err(error) = check(unsafe { AssignProcessToJobObject(handle(&job), handle(&process)) }) {
        unsafe {
            TerminateProcess(handle(&process), 130);
            WaitForSingleObject(handle(&process), INFINITE);
        }
        return Err(error);
    }
    if unsafe { ResumeThread(handle(&thread)) } == u32::MAX {
        let error = io::Error::last_os_error();
        unsafe {
            TerminateJobObject(handle(&job), 130);
            WaitForSingleObject(handle(&process), INFINITE);
        }
        return Err(error);
    }
    // Parent copies of inherited writers/NUL die here, leaving only child
    // handles. Thus EOF cannot precede the child's last writes.
    Ok(Child {
        process,
        job,
        stdout: Some(File::from(stdout)),
        stderr: Some(File::from(stderr)),
    })
}

fn pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let mut attributes: winapi::um::minwinbase::SECURITY_ATTRIBUTES = unsafe { mem::zeroed() };
    attributes.nLength = size_of::<winapi::um::minwinbase::SECURITY_ATTRIBUTES>() as u32;
    attributes.bInheritHandle = TRUE;
    let (mut read, mut write) = (ptr::null_mut(), ptr::null_mut());
    check(unsafe { CreatePipe(&mut read, &mut write, &mut attributes, 0) })?;
    let (read, write) = (owned(read)?, owned(write)?);
    check(unsafe { SetHandleInformation(handle(&read), HANDLE_FLAG_INHERIT, 0) })?;
    Ok((read, write))
}

struct Attributes {
    storage: Vec<u128>,
    initialized: bool,
}
impl Attributes {
    fn new(handles: &[HANDLE; 3]) -> io::Result<Self> {
        let mut bytes = 0;
        unsafe {
            InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut bytes);
        }
        if bytes == 0 || bytes > 64 * 1024 {
            return Err(io::Error::last_os_error());
        }
        let mut result = Self {
            storage: vec![0; bytes.div_ceil(size_of::<u128>())],
            initialized: false,
        };
        check(unsafe { InitializeProcThreadAttributeList(result.pointer(), 1, 0, &mut bytes) })?;
        result.initialized = true;
        check(unsafe {
            UpdateProcThreadAttribute(
                result.pointer(),
                0,
                HANDLE_LIST,
                handles.as_ptr() as *mut _,
                size_of::<[HANDLE; 3]>(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })?;
        Ok(result)
    }
    fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr() as *mut _
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                DeleteProcThreadAttributeList(self.pointer());
            }
        }
    }
}

fn handle(value: &OwnedHandle) -> HANDLE {
    value.as_raw_handle() as HANDLE
}
fn owned(value: HANDLE) -> io::Result<OwnedHandle> {
    if value.is_null() || value == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(value as _) })
    }
}
fn check(value: i32) -> io::Result<()> {
    if value == FALSE {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut result = Vec::new();
    for unit in value.encode_wide() {
        if unit == 0 {
            return Err(io::Error::other("Windows command value contains NUL"));
        }
        result.push(unit);
    }
    result.push(0);
    Ok(result)
}
fn compare(a: &[u16], b: &[u16]) -> Ordering {
    match unsafe {
        CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, TRUE)
    } {
        1 => Ordering::Less,
        3 => Ordering::Greater,
        _ => Ordering::Equal,
    }
}
fn environment(variables: &[(OsString, Option<OsString>)]) -> io::Result<Vec<u16>> {
    let mut entries: Vec<(Vec<u16>, Vec<u16>)> = std::env::vars_os()
        .map(|(key, value)| (key.encode_wide().collect(), value.encode_wide().collect()))
        .collect();
    for (key, value) in variables {
        let key: Vec<u16> = key.encode_wide().collect();
        if key.is_empty() || key.contains(&(b'=' as u16)) || key.contains(&0) {
            return Err(io::Error::other("Invalid command environment"));
        }
        entries.retain(|(existing, _)| compare(existing, &key) != Ordering::Equal);
        if let Some(value) = value {
            let value: Vec<u16> = value.encode_wide().collect();
            if value.contains(&0) {
                return Err(io::Error::other("Invalid command environment"));
            }
            entries.push((key, value));
        }
    }
    entries.sort_by(|(a, _), (b, _)| compare(a, b));
    let length = entries
        .iter()
        .try_fold(1usize, |count, (key, value)| {
            count.checked_add(key.len() + value.len() + 2)
        })
        .ok_or_else(|| io::Error::other("Command environment is too large"))?;
    if length > MAX_ENVIRONMENT_UNITS {
        return Err(io::Error::other("Command environment is too large"));
    }
    let mut result = Vec::with_capacity(length.max(2));
    for (key, value) in entries {
        result.extend(key);
        result.push(b'=' as u16);
        result.extend(value);
        result.push(0);
    }
    result.push(0);
    if result.len() == 1 {
        result.push(0);
    }
    Ok(result)
}

fn append_argument(command: &mut Vec<u16>, argument: &OsStr) -> io::Result<()> {
    let mut slashes = 0;
    command.push(b'"' as u16);
    for unit in argument.encode_wide() {
        if unit == 0 {
            return Err(io::Error::other("Windows argument contains NUL"));
        }
        if unit == b'\\' as u16 {
            slashes += 1;
        } else {
            if unit == b'"' as u16 {
                command.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2 + 1));
            } else {
                command.extend(std::iter::repeat_n(b'\\' as u16, slashes));
            }
            slashes = 0;
            command.push(unit);
        }
        if command.len().saturating_add(slashes * 2 + 2) > 32_767 {
            return Err(io::Error::other(
                "Windows command line exceeds its native UTF-16 limit",
            ));
        }
    }
    command.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
    command.push(b'"' as u16);
    Ok(())
}
