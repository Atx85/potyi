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
pub(crate) fn server_config(config: &ServerConfig, root: &Path) -> Result<ServerConfig, String> {
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
    // Keep the existing .sln preference when both formats are present.
    let preferred = root.file_name().and_then(|name| {
        ["sln", "slnx"]
            .into_iter()
            .map(|extension| root.join(format!("{}.{extension}", name.to_string_lossy())))
            .find(|path| path.is_file())
    });
    let solution = if let Some(preferred) = preferred {
        preferred
    } else {
        let mut solutions = Vec::new();
        let mut has_projects = false;
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
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if (extension.eq_ignore_ascii_case("sln") || extension.eq_ignore_ascii_case("slnx"))
                && path.is_file()
            {
                solutions.push(path);
            } else if extension.eq_ignore_ascii_case("csproj") && path.is_file() {
                has_projects = true;
            }
        }
        // Also recognize capitalized filenames on case-sensitive filesystems.
        solutions.sort();
        let named = ["sln", "slnx"].into_iter().find_map(|extension| {
            solutions.iter().position(|path| {
                path.file_stem()
                    .zip(root.file_name())
                    .is_some_and(|(stem, name)| {
                        stem.to_string_lossy()
                            .eq_ignore_ascii_case(&name.to_string_lossy())
                    })
                    && path
                        .extension()
                        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(extension))
            })
        });
        if let Some(index) = named {
            solutions.remove(index)
        } else {
            match solutions.len() {
                1 => solutions.remove(0),
                0 => {
                    let found = if has_projects {
                        " Found .csproj files, but no solution."
                    } else {
                        ""
                    };
                    return Err(format!(
                        "Unity: no .sln or .slnx solution found in {}.{found} Generate the solution in Unity Preferences > External Tools, or set an explicit --solution path in config/lsp.toml, then use :lsp restart.",
                        root.display()
                    ));
                }
                _ => {
                    return Err(format!(
                        "Unity has multiple solutions in {}. Set csharp-ls args = [\"--solution\", \"YourProject.slnx\"] in config/lsp.toml to select a .sln or .slnx file.",
                        root.display()
                    ));
                }
            }
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
                .contains("no .sln or .slnx solution")
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

    #[test]
    fn unity_solution_formats_and_missing_solution_diagnostics() {
        let dir = std::env::temp_dir().join(format!(
            "potyi-unity-formats-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join("Game");
        let config = ServerConfig {
            name: "csharp".into(),
            command: r"C:\Tools\csharp-ls.exe".into(),
            args: vec!["--loglevel".into(), "debug".into()],
            extensions: vec!["cs".into()],
            language_id: "csharp".into(),
            root_markers: vec![],
            initialization_options: serde_json::Value::Null,
        };
        for (files, selected) in [
            (vec!["Game.slnx", "Assembly-CSharp.csproj"], "Game.slnx"),
            (
                vec!["Renamed.slnx", "Assembly-CSharp.csproj"],
                "Renamed.slnx",
            ),
            (vec!["Game.slnx", "Other.sln"], "Game.slnx"),
            (vec!["Game.sln", "Game.slnx"], "Game.sln"),
            (vec!["Game.SLNX", "Other.sln"], "Game.SLNX"),
            (vec!["Renamed.SLN"], "Renamed.SLN"),
            (vec!["First.sln", "Second.slnx"], "multiple solutions"),
            (vec!["Assembly-CSharp.csproj"], "Found .csproj files"),
            (vec![], "no .sln or .slnx solution"),
        ] {
            std::fs::create_dir_all(root.join("ProjectSettings")).unwrap();
            std::fs::write(root.join("ProjectSettings/ProjectVersion.txt"), "version").unwrap();
            // Solutions in Library must never affect project selection.
            std::fs::create_dir_all(root.join("Library")).unwrap();
            std::fs::write(root.join("Library/Hidden.sln"), "").unwrap();
            for file in &files {
                std::fs::write(root.join(file), "").unwrap();
            }
            if files.contains(&selected) {
                let actual = server_config(&config, &root).unwrap();
                assert_eq!(&actual.args[..config.args.len()], config.args.as_slice());
                assert_eq!(actual.args[config.args.len()], "--solution");
                assert_eq!(actual.args.len(), config.args.len() + 2);
                assert_eq!(
                    Path::new(actual.args.last().unwrap())
                        .canonicalize()
                        .unwrap(),
                    root.join(selected).canonicalize().unwrap(),
                    "{files:?}"
                );
            } else {
                let error = server_config(&config, &root).unwrap_err();
                assert!(error.contains(selected), "{error}");
                assert!(error.contains(&root.display().to_string()), "{error}");
            }
            // Explicit settings and alternate servers must bypass discovery.
            for options in [
                serde_json::json!({"csharp": {"solutionPathOverride": "Custom.slnx"}}),
                serde_json::json!({"csharp.solutionPathOverride": "Custom.slnx"}),
            ] {
                let mut explicit = config.clone();
                explicit.initialization_options = options;
                assert_eq!(server_config(&explicit, &root).unwrap().args, config.args);
            }
            let mut alternate = config.clone();
            alternate.command = "OtherCSharpServer".into();
            assert_eq!(server_config(&alternate, &root).unwrap().args, config.args);
            std::fs::remove_dir_all(&root).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
