// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

use super::ServerConfig;
use std::path::{Path, PathBuf};

pub(super) fn root(path: &Path) -> Option<PathBuf> {
    path.parent()?
        .ancestors()
        .find(|p| {
            p.join("Assets").is_dir() && p.join("ProjectSettings/ProjectVersion.txt").is_file()
        })
        .map(Path::to_path_buf)
}

/// Use Unity's generated solution, never recursive discovery through Library.
/// Explicit server arguments and settings always win. All platforms share this.
pub(super) fn server_config(config: &ServerConfig, root: &Path) -> Result<ServerConfig, String> {
    let mut configured = config.clone();
    if config.language_id != "csharp" || !root.join("ProjectSettings/ProjectVersion.txt").is_file()
    {
        return Ok(configured);
    }
    let executable = config.command.rsplit(['/', '\\']).next().unwrap_or("");
    if !matches!(executable, "csharp-ls" | "csharp-ls.exe")
        || config
            .args
            .iter()
            .any(|arg| arg == "-s" || arg == "--solution" || arg.starts_with("--solution="))
        || config
            .initialization_options
            .pointer("/csharp/solutionPathOverride")
            .is_some()
        || config
            .initialization_options
            .get("csharp.solutionPathOverride")
            .is_some()
    {
        return Ok(configured);
    }
    let preferred = root
        .file_name()
        .map(|n| root.join(format!("{}.sln", n.to_string_lossy())));
    let solution = if let Some(preferred) = preferred.filter(|p| p.is_file()) {
        preferred
    } else {
        let mut solutions = Vec::new();
        for (index, entry) in std::fs::read_dir(root)
            .map_err(|e| e.to_string())?
            .enumerate()
            .take(4097)
        {
            if index == 4096 {
                return Err(
                    "Unity root has too many entries; configure an explicit --solution path".into(),
                );
            }
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_some_and(|e| e == "sln") && path.is_file() {
                solutions.push(path);
            }
        }
        match solutions.len() {
            1 => solutions.remove(0),
            0 => return Err("Unity: regenerate project files in Unity Preferences > External Tools, then use :lsp restart.".into()),
            _ => return Err("Unity has multiple solutions. Set csharp-ls args = [\"--solution\", \"YourProject.sln\"] in config/lsp.toml.".into()),
        }
    };
    configured
        .args
        .extend(["--solution".into(), solution.to_string_lossy().into_owned()]);
    Ok(configured)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unity_root_and_solution_override() {
        let dir = std::env::temp_dir().join(format!(
            "potyi-unity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join("Game");
        std::fs::create_dir_all(root.join("Assets/Scripts")).unwrap();
        std::fs::create_dir_all(root.join("ProjectSettings")).unwrap();
        std::fs::write(root.join("ProjectSettings/ProjectVersion.txt"), "version").unwrap();
        std::fs::create_dir(dir.join(".git")).unwrap();
        let mut config = ServerConfig {
            name: "csharp".into(),
            command: "csharp-ls".into(),
            args: vec![],
            extensions: vec!["cs".into()],
            language_id: "csharp".into(),
            root_markers: vec![".git".into()],
            initialization_options: serde_json::Value::Null,
        };
        assert_eq!(
            super::super::project_root(&root.join("Assets/Scripts/Main.cs"), &config),
            root
        );
        assert!(
            server_config(&config, &root)
                .unwrap_err()
                .contains("regenerate")
        );
        std::fs::write(root.join("Game.sln"), "").unwrap();
        std::fs::write(root.join("Other.sln"), "").unwrap();
        assert_eq!(
            server_config(&config, &root).unwrap().args,
            ["--solution", root.join("Game.sln").to_str().unwrap()]
        );
        config.args = vec!["--solution".into(), "Custom.sln".into()];
        assert_eq!(server_config(&config, &root).unwrap().args, config.args);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
