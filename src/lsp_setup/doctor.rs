// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    catalog::{RECIPES, Recipe},
    install,
    process::Runner,
    registry, tools,
};
use std::path::Path;

pub(super) fn doctor(
    recipe: Option<&Recipe>,
    root: &Path,
    project: &Path,
    file: Option<&Path>,
    runner: &mut Runner,
) -> Result<String, String> {
    let Some(recipe) = recipe else {
        let mut lines = vec![format!(
            "LSP setup · {} / {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )];
        for recipe in RECIPES {
            let status = match registry::read(root, recipe) {
                Ok(Some(_)) => "configured",
                Ok(None) => "not managed by Potyi",
                Err(_) => "installation record needs repair",
            };
            lines.push(format!("{} — {status}", recipe.title));
        }
        lines.push("Use :lsp doctor <server> for prerequisites and project checks, or :lsp install <server> to set it up.".into());
        return Ok(lines.join("\n"));
    };
    let mut lines = vec![format!(
        "{} · {} / {}",
        recipe.title,
        std::env::consts::OS,
        std::env::consts::ARCH
    )];
    match registry::read(root, recipe) {
        Ok(Some(record)) => match install::verify(&record, runner) {
            Ok(()) => lines.push("Installed launcher: OK".into()),
            Err(e) => lines.push(format!("Installed launcher: {e}")),
        },
        Ok(None) => {
            let external = recipe
                .servers()
                .iter()
                .filter_map(|s| tools::executable(&s.command))
                .next();
            lines.push(match external {
                Some(path) => format!(
                    "Found on this computer: {}. Use :lsp install {} to verify and configure it.",
                    path.display(),
                    recipe.id
                ),
                None => format!(
                    "Server is not installed by Potyi. Run :lsp install {}.",
                    recipe.id
                ),
            });
        }
        Err(error) => lines.push(format!("Installation record: {error}")),
    }
    match tools::prerequisites(recipe, runner) {
        Ok(()) => lines.push("Installation prerequisites: OK".into()),
        Err(e) => lines.push(e),
    }
    match crate::lsp::Config::load() {
        Err(error) => lines.push(format!("Project configuration: {error}")),
        Ok(config) => {
            lines.push(
                if config.enabled {
                    "LSP is enabled in configuration."
                } else {
                    "Use :lsp start to enable LSP for this window."
                }
                .into(),
            );
            if let Some(file) = file {
                if let Some(server) = config.server_for(file) {
                    if recipe.profiles.contains(&server.name.as_str())
                        && !registry::is_managed(server, recipe)
                    {
                        lines.push(format!("Project-selected executable: {}. Explicit alternate commands take precedence over managed setup.", server.command));
                    }
                    let root = crate::lsp::project_root(file, server);
                    lines.push(format!(
                        "Selected server: {}\nProject root: {}",
                        server.name,
                        root.display()
                    ));
                    if recipe.id == "csharp"
                        && root.join("ProjectSettings/ProjectVersion.txt").is_file()
                    {
                        match crate::lsp::unity::server_config(server, &root) {
                            Ok(_) => lines.push("Unity solution selection: OK".into()),
                            Err(e) => lines.push(e),
                        }
                    }
                    if recipe.id == "clangd"
                        && !root.join("compile_commands.json").is_file()
                        && !root.join("build/compile_commands.json").is_file()
                    {
                        lines.push("Project needs a compile_commands.json with its C/C++ build flags; none was found at the root or in build/.".into());
                    }
                } else {
                    lines.push("No server is currently selected for this file. Installing its recipe adds the default configuration.".into());
                }
            }
        }
    }
    let note = match recipe.id {
        "csharp" => {
            "For Unity, keep the generated .sln or .slnx solution and .csproj files at the Unity project root, with Unity's assemblies available. For other C# projects, restore their .NET dependencies."
        }
        "python" => {
            "The managed Python server uses its own environment. For project-only dependencies, select your project's pylsp interpreter in config/lsp.toml or configure Jedi's environment."
        }
        "java" => {
            "Java projects need resolvable Maven/Gradle dependencies. Potyi assigns separate JDT workspaces to each project and app process."
        }
        "sql" => {
            "Configure sqls with your local database connection settings for schema-aware features. Potyi does not read or install database credentials."
        }
        "toml" => {
            "Building Taplo may require C build tools and OpenSSL development packages on Linux. Schema associations enable useful TOML descriptions."
        }
        "go" => "Open a Go module/workspace with its go.mod/go.work and dependencies available.",
        "typescript" => {
            "Install the project's npm dependencies and keep its tsconfig.json/jsconfig.json available. Vue's TypeScript bridge is not supported."
        }
        "bash" => {
            "This server supports Bash syntax. ShellCheck and shfmt are optional; other shell dialects may need a different server."
        }
        "rust" => {
            "The managed server uses the stable Rust toolchain. Check that the project's Cargo dependencies and toolchain are available."
        }
        _ => {
            "Project dependencies, schemas and library settings may still be needed for useful language features."
        }
    };
    lines.push(note.into());
    lines.push(format!(
        "Working folder: {}\nUpstream setup: {}",
        project.display(),
        recipe.url
    ));
    Ok(lines.join("\n\n"))
}
