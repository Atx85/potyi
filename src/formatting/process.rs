// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

//! Nonblocking pipe reads used only during a foreground format command.
//! No reader threads, output queues, or long-lived child processes.

use std::io::{self, Read};
use std::process::{Child, Command};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

#[cfg(unix)]
unsafe extern "C" {
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    fn kill(pid: i32, signal: i32) -> i32;
}

#[cfg(unix)]
pub(super) fn prepare_pipe(pipe: &impl AsRawFd) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    const NONBLOCK: i32 = 0x800;
    #[cfg(not(target_os = "linux"))]
    const NONBLOCK: i32 = 0x4;
    // F_GETFL/F_SETFL and O_NONBLOCK come from fcntl.h on our Unix targets.
    // SAFETY: pipe owns a live descriptor; fcntl neither retains nor closes it.
    unsafe {
        let flags = fcntl(pipe.as_raw_fd(), 3);
        if flags < 0 || fcntl(pipe.as_raw_fd(), 4, flags | NONBLOCK) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn prepare_pipe(_pipe: &impl AsRawHandle) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(super) fn read_pipe(
    pipe: &mut (impl Read + AsRawFd),
    buffer: &mut [u8],
) -> io::Result<Option<usize>> {
    match pipe.read(buffer) {
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(None)
        }
        result => result.map(Some),
    }
}

#[cfg(windows)]
pub(super) fn read_pipe(
    pipe: &mut (impl Read + AsRawHandle),
    buffer: &mut [u8],
) -> io::Result<Option<usize>> {
    use winapi::um::namedpipeapi::PeekNamedPipe;
    let mut available = 0;
    // SAFETY: pipe holds a live readable handle; only available is written.
    if unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle().cast(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    } == 0
    {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(109) {
            return Ok(Some(0));
        } // ERROR_BROKEN_PIPE
        return Err(error);
    }
    if available == 0 {
        return Ok(None);
    }
    let amount = buffer.len().min(available as usize);
    pipe.read(&mut buffer[..amount]).map(Some)
}

pub(crate) struct Running {
    pub child: Child,
    #[cfg(windows)]
    job: winapi::um::winnt::HANDLE,
}

impl Running {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
            Ok(Self {
                child: command.spawn()?,
            })
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use winapi::um::{handleapi::CloseHandle, jobapi2::*, winnt::*};
            // A job closes all remaining formatter processes when this
            // foreground operation ends, including error/timeout exits.
            unsafe {
                let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
                if job.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&limits) as u32,
                ) == 0
                {
                    let error = io::Error::last_os_error();
                    CloseHandle(job);
                    return Err(error);
                }
                command.creation_flags(0x08000000); // CREATE_NO_WINDOW
                let child = match command.spawn() {
                    Ok(child) => child,
                    Err(error) => {
                        CloseHandle(job);
                        return Err(error);
                    }
                };
                let mut running = Self { child, job };
                if AssignProcessToJobObject(job, running.child.as_raw_handle().cast()) == 0 {
                    let error = io::Error::last_os_error();
                    let _ = running.child.kill();
                    return Err(error);
                }
                Ok(running)
            }
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        #[cfg(unix)]
        // SAFETY: the child was given its own process group at spawn. The
        // negative PID targets that formatter group, never Pötyi's group.
        unsafe {
            kill(-(self.child.id() as i32), 9);
        }
        #[cfg(windows)]
        unsafe {
            winapi::um::handleapi::CloseHandle(self.job);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
