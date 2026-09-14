// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    catalog::{Kind, Recipe},
    process::Runner,
    registry::home,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(super) fn executable(name: &str) -> Option<PathBuf> {
    let mut directories: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| {
            std::env::split_paths(&p)
                .filter(|p| p.is_absolute())
                .collect()
        })
        .unwrap_or_default();
    if let Some(home) = home() {
        directories.extend([
            home.join(".cargo/bin"),
            home.join(".dotnet/tools"),
            home.join("go/bin"),
        ]);
    }
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(dir) = std::env::var_os(var) {
                let dir = PathBuf::from(dir);
                directories.extend([dir.join("nodejs"), dir.join("dotnet"), dir.join("Go/bin")]);
            }
        }
    } else {
        directories.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ]);
    }
    if name == "java" {
        if let Some(dir) = std::env::var_os("JAVA_HOME") {
            directories.insert(0, PathBuf::from(dir).join("bin"));
        }
    }
    if matches!(name, "gopls" | "sqls") {
        if let Some(dir) = std::env::var_os("GOBIN") {
            directories.push(PathBuf::from(dir));
        }
    }
    for dir in directories {
        let path = dir.join(if cfg!(windows) && !name.ends_with(".exe") {
            format!("{name}.exe")
        } else {
            name.into()
        });
        if is_executable(&path) {
            return std::path::absolute(path).ok();
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    true
}

pub(super) fn hint(tool: &str) -> String {
    let (windows, macos, linux) = match tool {
        "node" | "npm" => (
            "winget install OpenJS.NodeJS.LTS",
            "brew install node",
            "install Node.js LTS and npm using your distribution's package manager or nodejs.org",
        ),
        "dotnet" => (
            "winget install Microsoft.DotNet.SDK.10",
            "brew install --cask dotnet-sdk",
            "install the .NET 10 SDK for your distribution from dotnet.microsoft.com/download",
        ),
        "python" => (
            "winget install Python.Python.3.13",
            "brew install python",
            "install python3, pip and the venv module (Debian/Ubuntu: sudo apt install python3 python3-venv)",
        ),
        "java" => (
            "winget install EclipseAdoptium.Temurin.21.JDK",
            "brew install --cask temurin@21",
            "install a JDK 21 or newer (Debian/Ubuntu: sudo apt install openjdk-21-jdk)",
        ),
        "go" => (
            "winget install GoLang.Go",
            "brew install go",
            "install a current Go toolchain from go.dev/dl or your distribution",
        ),
        "rustup" | "cargo" => (
            "install Rust with rustup from rustup.rs and the Visual Studio C++ build tools",
            "install Rust with rustup from rustup.rs",
            "install Rust with rustup from rustup.rs and your distribution's C build tools",
        ),
        "curl" => (
            "install curl or enable the curl.exe supplied with Windows",
            "install curl",
            "install curl using your distribution's package manager",
        ),
        _ => (
            "install the required tool and add it to PATH",
            "install the required tool and add it to PATH",
            "install the required tool and add it to PATH",
        ),
    };
    format!(
        "{}\nRestart Potyi after changing PATH, then retry setup.",
        if cfg!(windows) {
            windows
        } else if cfg!(target_os = "macos") {
            macos
        } else {
            linux
        }
    )
}

pub(super) fn require(tool: &str) -> Result<PathBuf, String> {
    executable(tool).ok_or_else(|| format!("Missing prerequisite: {tool}.\n{}", hint(tool)))
}

pub(super) fn python() -> Result<(PathBuf, Vec<OsString>), String> {
    if cfg!(windows) {
        if let Some(path) = executable("py") {
            return Ok((path, vec!["-3".into()]));
        }
    }
    for name in ["python3", "python"] {
        if let Some(path) = executable(name) {
            return Ok((path, vec![]));
        }
    }
    Err(format!(
        "Missing prerequisite: Python 3.9 or newer.\n{}",
        hint("python")
    ))
}

pub(super) fn npm(node: &Path) -> Result<PathBuf, String> {
    let parent = node.parent().ok_or("Node path has no directory")?;
    let mut candidates = vec![
        parent.join("node_modules/npm/bin/npm-cli.js"),
        parent.join("../lib/node_modules/npm/bin/npm-cli.js"),
    ];
    if let Some(path) = executable("npm") {
        if let Ok(path) = path.canonicalize() {
            candidates.push(path);
        }
    }
    // On Unix npm is usually a symlink to its JavaScript CLI.
    if !cfg!(windows) {
        if let Some(search) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&search).filter(|p| p.is_absolute()) {
                if let Ok(path) = dir.join("npm").canonicalize() {
                    candidates.push(path);
                }
            }
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file() && p.file_name().is_some_and(|n| n == "npm-cli.js"))
        .and_then(|p| p.canonicalize().ok())
        .ok_or_else(|| {
            format!(
                "Could not find npm's JavaScript launcher beside Node.js.\n{}",
                hint("npm")
            )
        })
}

pub(super) fn prerequisites(recipe: &Recipe, runner: &mut Runner) -> Result<(), String> {
    prerequisites_inner(recipe, runner).map_err(|error| {
        let tool = match recipe.kind {
            Kind::Npm(_) => "node",
            Kind::Dotnet => "dotnet",
            Kind::Python => "python",
            Kind::Java => "java",
            Kind::Rustup => "rustup",
            Kind::Cargo => "cargo",
            Kind::Go(_) => "go",
            Kind::Github(_) => "curl",
        };
        if error.contains("Restart Potyi") || runner.check().is_err() {
            error
        } else {
            format!("{error}\n{}", hint(tool))
        }
    })
}

fn prerequisites_inner(recipe: &Recipe, runner: &mut Runner) -> Result<(), String> {
    match recipe.kind {
        Kind::Npm(_) => {
            let node = require("node")?;
            npm(&node)?;
            let version = runner.command("Checking Node.js", &node, &["--version"])?;
            let major = version
                .trim()
                .trim_start_matches('v')
                .split('.')
                .next()
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0);
            if major < 20 {
                return Err(format!(
                    "Node.js 20 or newer is required.\n{}",
                    hint("node")
                ));
            }
        }
        Kind::Dotnet => {
            let output =
                runner.command("Checking .NET SDK", &require("dotnet")?, &["--list-sdks"])?;
            if !output.lines().any(|l| {
                l.split('.')
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                    .is_some_and(|v| v >= 10)
            }) {
                return Err(format!(
                    "csharp-ls requires the .NET 10 SDK or newer.\n{}",
                    hint("dotnet")
                ));
            }
        }
        Kind::Python => {
            let (python, mut args) = python()?;
            args.extend(["-c".into(),"import sys, venv; assert sys.version_info >= (3,9), 'Python 3.9 or newer is required'".into()]);
            runner.run(
                "Checking Python and venv",
                &python,
                &args,
                &[],
                std::time::Duration::from_secs(20),
            )?;
        }
        Kind::Rustup => {
            runner.command("Checking rustup", &require("rustup")?, &["--version"])?;
        }
        Kind::Cargo => {
            runner.command("Checking Cargo", &require("cargo")?, &["--version"])?;
        }
        Kind::Go(_) => {
            runner.command("Checking Go", &require("go")?, &["version"])?;
        }
        Kind::Github(_) => {
            runner.command("Checking downloader", &require("curl")?, &["--version"])?;
        }
        Kind::Java => {
            require("curl")?;
            let output = runner.command("Checking Java", &require("java")?, &["-version"])?;
            let major = output
                .split('"')
                .nth(1)
                .and_then(|v| v.split('.').next())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            if major < 21 {
                return Err(format!(
                    "JDT Language Server requires Java 21 or newer.\n{}",
                    hint("java")
                ));
            }
        }
    }
    Ok(())
}
