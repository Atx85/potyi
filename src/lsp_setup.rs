// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit, per-user setup. No network/process work is performed on the UI thread.
mod archive;
pub(crate) mod catalog;
mod doctor;
mod install;
mod process;
mod registry;
#[cfg(test)]
mod tests;
mod tools;
pub(crate) use registry::{augment, configure_project, is_managed};

use crate::command_bar::CommandBar;
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

pub(crate) struct Event {
    id: u64,
    text: String,
    done: bool,
    installed: Option<&'static str>,
}

#[derive(Default)]
pub(crate) struct Manager {
    next: u64,
    active: Option<(u64, Arc<AtomicBool>, JoinHandle<()>)>,
    epoch: u64,
    file: Option<PathBuf>,
    pub last: Option<String>,
}

impl Manager {
    pub fn cancel(&mut self) {
        if let Some((_, cancel, _)) = &self.active {
            cancel.store(true, Ordering::Relaxed);
            self.last = Some("Stopping LSP setup…".into());
        }
    }
    pub fn start(
        &mut self,
        installing: bool,
        name: Option<&str>,
        file: Option<PathBuf>,
        events: &sdl3::EventSubsystem,
        bar: &mut CommandBar,
    ) -> Result<(), String> {
        if let Some((_, _, worker)) = &self.active {
            if !worker.is_finished() {
                return Err("LSP setup is already running. Use :lsp stop to cancel it.".into());
            }
        }
        self.active.take();
        let recipe = match name {
            Some(name) => Some(catalog::find(name)?),
            None => file.as_deref().and_then(catalog::for_file),
        };
        if installing && recipe.is_none() {
            return Err("Choose a server: :lsp install <server>".into());
        }
        let root = registry::root()?;
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let file = file.map(|p| cwd.join(p));
        let id = self.next.wrapping_add(1);
        self.next = id;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let sender = events.event_sender();
        let updates = events.event_sender();
        let context = file.clone();
        let worker = std::thread::Builder::new()
            .name("lsp-setup".into())
            .spawn(move || {
                let result = (|| {
                    let base = if installing {
                        root.join("installs")
                    } else {
                        std::env::temp_dir()
                    };
                    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
                    let stamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| e.to_string())?
                        .as_nanos();
                    let directory = base.join(format!(
                        "potyi-{}-{}-{stamp}",
                        recipe.map(|r| r.id).unwrap_or("doctor"),
                        std::process::id()
                    ));
                    fs::create_dir(&directory).map_err(|e| e.to_string())?;
                    let mut runner =
                        process::Runner::new(directory.clone(), worker_cancel, move |text| {
                            let _ = updates.push_custom_event(Event {
                                id,
                                text,
                                done: false,
                                installed: None,
                            });
                        });
                    let result = if installing {
                        install::install(recipe.unwrap(), &root, &mut runner)
                    } else {
                        doctor::doctor(recipe, &root, &cwd, context.as_deref(), &mut runner)
                    };
                    if !installing || result.is_err() {
                        let _ = fs::remove_dir_all(&directory);
                    }
                    result
                })();
                let installed = if installing && result.is_ok() {
                    recipe.map(|r| r.id)
                } else {
                    None
                };
                let text = match result {
                    Ok(text) => text,
                    Err(error) => {
                        format!("LSP setup: {error}\nYour documents have not been changed.")
                    }
                };
                let _ = sender.push_custom_event(Event {
                    id,
                    text,
                    done: true,
                    installed,
                });
            })
            .map_err(|e| e.to_string())?;
        self.active = Some((id, cancel, worker));
        self.epoch = bar.epoch();
        self.file = file;
        let text = if installing {
            format!(
                "Setting up {}…\nProgress appears here. Escape closes the panel; :lsp status shows progress and :lsp stop cancels setup.",
                recipe.unwrap().title
            )
        } else {
            "Checking language-server setup…".into()
        };
        self.last = Some(text.clone());
        bar.show_info(&text);
        Ok(())
    }
    pub fn accept(
        &mut self,
        event: Event,
        bar: &mut CommandBar,
        file: Option<&std::path::Path>,
    ) -> Option<&'static str> {
        if self
            .active
            .as_ref()
            .is_none_or(|(id, _, _)| *id != event.id)
        {
            return None;
        }
        let visible = bar.is_active() && bar.epoch() == self.epoch;
        let text = if event.done {
            event.text
        } else {
            format!(
                "{}\n\nEscape closes this panel. :lsp stop cancels setup.",
                event.text
            )
        };
        self.last = Some(text.clone());
        if visible
            || (bar.is_active()
                && matches!(
                    bar.parse(),
                    Ok(crate::command_bar::ParsedCommand::LspStatus)
                ))
        {
            bar.show_info(&text);
        }
        if event.done {
            self.active.take();
            let current = file.and_then(|p| std::path::absolute(p).ok());
            if visible && current == self.file {
                return event.installed;
            }
        }
        None
    }
}
impl Drop for Manager {
    fn drop(&mut self) {
        if let Some((_, cancel, worker)) = self.active.take() {
            cancel.store(true, Ordering::Relaxed);
            let _ = worker.join();
        }
    }
}
