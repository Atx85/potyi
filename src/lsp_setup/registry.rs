// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::catalog::{RECIPES, Recipe};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Launch {
    pub command: String,
    pub prefix: Vec<String>,
    pub probe: Vec<String>,
    #[serde(default)]
    pub java_config: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Installed {
    pub recipe: String,
    pub directory: PathBuf,
    pub launches: BTreeMap<String, Launch>,
}

pub(super) fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

pub(crate) fn root() -> Result<PathBuf, String> {
    let path = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("Potyi/lsp"))
    } else if cfg!(target_os = "macos") {
        home().map(|p| p.join("Library/Application Support/Potyi/lsp"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home().map(|p| p.join(".local/share")))
            .map(|p| p.join("potyi/lsp"))
    };
    path.filter(|p| p.is_absolute())
        .ok_or("Cannot locate your user application-data directory".into())
}

pub(super) fn read(root: &Path, recipe: &Recipe) -> Result<Option<Installed>, String> {
    let path = root.join(format!("{}.json", recipe.id));
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot read {}: {e}", path.display())),
    };
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Installed server record exceeds size limit".into());
    }
    let record: Installed = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid installed server record: {e}"))?;
    if record.recipe != recipe.id
        || record.launches.len() != recipe.profiles.len()
        || recipe
            .profiles
            .iter()
            .any(|name| !record.launches.contains_key(*name))
        || record.launches.values().any(|l| {
            !Path::new(&l.command).is_absolute()
                || l.prefix.len() > 32
                || l.prefix.iter().any(|s| s.contains('\0'))
        })
    {
        return Err("Invalid installed server launcher".into());
    }
    Ok(Some(record))
}

pub(super) fn save(root: &Path, record: &Installed) -> Result<(), String> {
    let path = root.join(format!("{}.json", record.recipe));
    let temporary = root.join(format!("{}.{}.tmp", record.recipe, std::process::id()));
    let bytes = serde_json::to_vec_pretty(record).map_err(|e| e.to_string())?;
    fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    fs::rename(&temporary, &path).map_err(|e| format!("Could not activate installation: {e}"))
}

pub(crate) fn augment(config: &mut crate::lsp::Config) {
    if let Ok(root) = root() {
        augment_from(config, &root);
    }
}

pub(super) fn augment_from(config: &mut crate::lsp::Config, root: &Path) {
    for recipe in RECIPES {
        let Ok(Some(installed)) = read(root, recipe) else {
            continue;
        };
        for mut bundled in recipe.servers() {
            let Some(launch) = installed.launches.get(&bundled.name) else {
                continue;
            };
            if let Some(existing) = config.servers.iter_mut().find(|s| s.name == bundled.name) {
                // Keep explicitly selected alternate executables and all project options.
                if existing.command != bundled.command {
                    continue;
                }
                existing.command = launch.command.clone();
                let mut args = launch.prefix.clone();
                args.extend(existing.args.clone());
                existing.args = args;
            } else {
                bundled.command = launch.command.clone();
                let mut args = launch.prefix.clone();
                args.extend(bundled.args);
                bundled.args = args;
                config.servers.push(bundled);
            }
        }
    }
}

/// JDT LS needs writable configuration and a separate workspace per project/session.
/// Only managed Java launchers are adjusted; explicit project launchers keep their settings.
pub(crate) fn configure_project(
    config: &mut crate::lsp::ServerConfig,
    project: &Path,
) -> Result<(), String> {
    let Ok(root) = root() else { return Ok(()) };
    configure_from(config, project, &root)
}

pub(super) fn configure_from(
    config: &mut crate::lsp::ServerConfig,
    project: &Path,
    root: &Path,
) -> Result<(), String> {
    if config.language_id != "java"
        || config
            .args
            .iter()
            .any(|a| a == "-data" || a == "-configuration")
    {
        return Ok(());
    }
    let Some(recipe) = RECIPES.iter().find(|r| r.id == "java") else {
        return Ok(());
    };
    let Some(record) = read(&root, recipe)? else {
        return Ok(());
    };
    let Some(launch) = record.launches.get("java") else {
        return Ok(());
    };
    if config.command != launch.command || !config.args.starts_with(&launch.prefix) {
        return Ok(());
    }
    let Some(template) = &launch.java_config else {
        return Ok(());
    };
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(project.as_os_str().as_encoded_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let cache =
        root.join("java-workspaces")
            .join(format!("{}-{}", &digest[..24], std::process::id()));
    let configuration = cache.join("config");
    fs::create_dir_all(&configuration).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(template).map_err(|e| e.to_string())?.take(128) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_file() {
            let destination = configuration.join(entry.file_name());
            if !destination.exists() {
                fs::copy(entry.path(), destination).map_err(|e| e.to_string())?;
            }
        }
    }
    config.args.extend([
        "-configuration".into(),
        configuration.to_string_lossy().into_owned(),
        "-data".into(),
        cache.join("data").to_string_lossy().into_owned(),
    ]);
    Ok(())
}

pub(crate) fn is_managed(server: &crate::lsp::ServerConfig, recipe: &Recipe) -> bool {
    let Ok(root) = root() else { return false };
    let Ok(Some(record)) = read(&root, recipe) else {
        return false;
    };
    record.launches.get(&server.name).is_some_and(|launch| {
        server.command == launch.command && server.args.starts_with(&launch.prefix)
    })
}
