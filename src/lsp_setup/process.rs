// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! A cancellable child-process runner. All polling and disk I/O is on the setup worker.
use crate::formatting::process::Running;
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub(super) struct Runner {
    pub directory: PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub report: Box<dyn Fn(String) + Send>,
    serial: usize,
}

impl Runner {
    pub fn new(
        directory: PathBuf,
        cancel: Arc<AtomicBool>,
        report: impl Fn(String) + Send + 'static,
    ) -> Self {
        Self {
            directory,
            cancel,
            report: Box::new(report),
            serial: 0,
        }
    }
    pub fn check(&self) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("Setup cancelled. You can keep editing; run :lsp install again to retry.".into())
        } else {
            Ok(())
        }
    }
    pub fn run(
        &mut self,
        label: &str,
        program: &Path,
        args: &[OsString],
        env: &[(&str, OsString)],
        timeout: Duration,
    ) -> Result<String, String> {
        self.check()?;
        (self.report)(label.into());
        self.serial += 1;
        let log_path = self.directory.join(format!("step-{}.log", self.serial));
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&log_path)
            .map_err(|e| e.to_string())?;
        let mut command = Command::new(program);
        command
            .args(args)
            .envs(env.iter().map(|(k, v)| (*k, v)))
            .current_dir(&self.directory)
            .stdin(Stdio::null())
            .stdout(Stdio::from(output.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(output));
        let mut running = Running::spawn(&mut command).map_err(|e| format!("{label}: {e}"))?;
        let started = Instant::now();
        let mut last_update = Instant::now();
        let mut last_size = 0;
        loop {
            self.check()?;
            if started.elapsed() > timeout {
                return Err(format!(
                    "{label}: timed out. Check the connection and prerequisites, then retry."
                ));
            }
            let size = fs::metadata(&log_path).map_err(|e| e.to_string())?.len();
            if size > 8 * 1024 * 1024 {
                return Err(format!("{label}: output limit exceeded"));
            }
            if size != last_size && last_update.elapsed() >= Duration::from_millis(300) {
                let mut file = File::open(&log_path).map_err(|e| e.to_string())?;
                file.seek(SeekFrom::Start(size.saturating_sub(4096)))
                    .map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                file.take(4096)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                (self.report)(format!(
                    "{label}\n{}",
                    clean(&String::from_utf8_lossy(&bytes))
                ));
                last_size = size;
                last_update = Instant::now();
            }
            if let Some(status) = running.child.try_wait().map_err(|e| e.to_string())? {
                let mut output = Vec::new();
                File::open(&log_path)
                    .map_err(|e| e.to_string())?
                    .take(8 * 1024 * 1024 + 1)
                    .read_to_end(&mut output)
                    .map_err(|e| e.to_string())?;
                if output.len() > 8 * 1024 * 1024 {
                    return Err(format!("{label}: output limit exceeded"));
                }
                let text = String::from_utf8_lossy(&output).into_owned();
                if !status.success() {
                    return Err(format!(
                        "{label} failed ({status}).\n{}",
                        clean(
                            &text
                                .chars()
                                .rev()
                                .take(6000)
                                .collect::<String>()
                                .chars()
                                .rev()
                                .collect::<String>()
                        )
                    ));
                }
                return Ok(text);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    pub fn command(
        &mut self,
        label: &str,
        program: &Path,
        args: &[&str],
    ) -> Result<String, String> {
        self.run(
            label,
            program,
            &args.iter().map(OsString::from).collect::<Vec<_>>(),
            &[],
            Duration::from_secs(20),
        )
    }
}

pub(super) fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}
