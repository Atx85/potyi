// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    archive,
    catalog::{Kind, Recipe},
    process::Runner,
    registry::{self, Installed, Launch},
    tools,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
const INSTALL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

fn string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
fn bin(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    }
}
fn launch(command: &Path, prefix: Vec<String>, probe: &[&str]) -> Launch {
    Launch {
        command: string(command),
        prefix,
        probe: probe.iter().map(|s| s.to_string()).collect(),
        java_config: None,
    }
}
fn probe_args(id: &str) -> &'static [&'static str] {
    match id {
        "python" | "luau" | "bash" => &["--help"],
        "go" => &["version"],
        "sql" => &["-h"],
        "toml" => &["lsp", "--help"],
        _ => &["--version"],
    }
}

pub(super) fn verify(record: &Installed, runner: &mut Runner) -> Result<(), String> {
    let mut checked = std::collections::HashSet::new();
    for item in record.launches.values() {
        if !Path::new(&item.command).is_file() {
            return Err(format!("Installed executable is missing: {}", item.command));
        }
        if let Some(script) = item
            .prefix
            .first()
            .filter(|p| p.ends_with(".js") || p.ends_with(".mjs") || p.ends_with(".cjs"))
        {
            if !Path::new(script).is_file() {
                return Err(format!("Installed server script is missing: {script}"));
            }
        }
        if let Some(index) = item.prefix.iter().position(|p| p == "-jar") {
            if !item
                .prefix
                .get(index + 1)
                .is_some_and(|p| Path::new(p).is_file())
            {
                return Err("JDT LS launcher jar is missing".into());
            }
        }
        if let Some(config) = &item.java_config {
            if !config.join("config.ini").is_file() {
                return Err("JDT LS configuration is missing".into());
            }
        }
        if checked.insert((item.command.clone(), item.probe.clone())) {
            let args = item.probe.iter().map(OsString::from).collect::<Vec<_>>();
            runner.run(
                "Checking the installed launcher",
                Path::new(&item.command),
                &args,
                &[],
                Duration::from_secs(20),
            )?;
        }
    }
    Ok(())
}

fn system(recipe: &Recipe, runner: &mut Runner) -> Option<Installed> {
    if matches!(recipe.kind, Kind::Npm(_) | Kind::Java | Kind::Rustup) {
        return None;
    }
    let mut launches = BTreeMap::new();
    for profile in recipe.servers() {
        let path = tools::executable(&profile.command)?;
        launches.insert(profile.name, launch(&path, vec![], probe_args(recipe.id)));
    }
    let record = Installed {
        recipe: recipe.id.into(),
        directory: runner.directory.clone(),
        launches,
    };
    verify(&record, runner).ok()?;
    Some(record)
}

pub(super) fn install(recipe: &Recipe, root: &Path, runner: &mut Runner) -> Result<String, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let lock_path = root.join(format!("{}.lock", recipe.id));
    let lock=fs::OpenOptions::new().write(true).create_new(true).open(&lock_path)
        .map_err(|e|format!("Cannot begin setup: {e}. Another Potyi window may be installing {}. If all windows are closed, remove the stale lock at {}.",recipe.id,lock_path.display()))?;
    struct Lock(PathBuf, Option<fs::File>);
    impl Drop for Lock {
        fn drop(&mut self) {
            drop(self.1.take());
            let _ = fs::remove_file(&self.0);
        }
    }
    let _lock = Lock(lock_path, Some(lock));
    if let Ok(Some(record)) = registry::read(root, recipe) {
        if verify(&record, runner).is_ok() {
            return Ok(format!(
                "{} is already installed and its launcher works.\nUse :lsp doctor {} to check this project.",
                recipe.title, recipe.id
            ));
        }
        runner.check()?;
    }
    if let Some(record) = system(recipe, runner) {
        registry::save(root, &record)?;
        return Ok(format!(
            "Found and configured your existing {} installation.\nProject settings are preserved.",
            recipe.title
        ));
    }
    tools::prerequisites(recipe, runner)?;
    let directory = runner.directory.join("server");
    fs::create_dir(&directory).map_err(|e| e.to_string())?;
    let mut launches = BTreeMap::new();
    match recipe.kind {
        Kind::Rustup => {
            let rustup = tools::require("rustup")?;
            runner.run("Installing rust-analyzer and Rust standard-library sources",&rustup,&["component".into(),"add".into(),"--toolchain".into(),"stable".into(),"rust-analyzer".into(),"rust-src".into()],&[],INSTALL_TIMEOUT)
                .map_err(|e|format!("{e}\nIf the stable Rust toolchain is missing, install it with rustup toolchain install stable."))?;
            launches.insert(
                "rust".into(),
                launch(
                    &rustup,
                    vec!["run".into(), "stable".into(), "rust-analyzer".into()],
                    &["run", "stable", "rust-analyzer", "--version"],
                ),
            );
        }
        Kind::Dotnet => {
            runner.run(
                "Installing csharp-ls",
                &tools::require("dotnet")?,
                &[
                    "tool".into(),
                    "install".into(),
                    "--tool-path".into(),
                    directory.as_os_str().to_owned(),
                    "csharp-ls".into(),
                ],
                &[("DOTNET_NOLOGO", "1".into())],
                INSTALL_TIMEOUT,
            )?;
            launches.insert(
                "csharp".into(),
                launch(&directory.join(bin("csharp-ls")), vec![], &["--version"]),
            );
        }
        Kind::Python => {
            let (python, mut args) = tools::python()?;
            args.extend(["-m".into(), "venv".into(), directory.as_os_str().to_owned()]);
            runner.run(
                "Creating a Python environment for the server",
                &python,
                &args,
                &[],
                INSTALL_TIMEOUT,
            )?;
            let binaries = directory.join(if cfg!(windows) { "Scripts" } else { "bin" });
            runner.run(
                "Installing python-lsp-server",
                &binaries.join(bin("python")),
                &[
                    "-m".into(),
                    "pip".into(),
                    "install".into(),
                    "--disable-pip-version-check".into(),
                    "python-lsp-server".into(),
                ],
                &[],
                INSTALL_TIMEOUT,
            )?;
            launches.insert(
                "python".into(),
                launch(&binaries.join(bin("pylsp")), vec![], &["--help"]),
            );
        }
        Kind::Go(package) => {
            let binaries = directory.join("bin");
            fs::create_dir(&binaries).map_err(|e| e.to_string())?;
            runner.run(
                &format!("Installing {}", recipe.title),
                &tools::require("go")?,
                &["install".into(), package.into()],
                &[("GOBIN", binaries.as_os_str().to_owned())],
                INSTALL_TIMEOUT,
            )?;
            for profile in recipe.servers() {
                launches.insert(
                    profile.name,
                    launch(
                        &binaries.join(bin(&profile.command)),
                        vec![],
                        probe_args(recipe.id),
                    ),
                );
            }
        }
        Kind::Cargo => {
            runner.run(
                "Building Taplo with language-server support",
                &tools::require("cargo")?,
                &[
                    "install".into(),
                    "taplo-cli".into(),
                    "--locked".into(),
                    "--features".into(),
                    "lsp".into(),
                    "--root".into(),
                    directory.as_os_str().to_owned(),
                ],
                &[],
                INSTALL_TIMEOUT,
            )?;
            launches.insert(
                "toml".into(),
                launch(
                    &directory.join("bin").join(bin("taplo")),
                    vec![],
                    &["lsp", "--help"],
                ),
            );
        }
        Kind::Npm(packages) => {
            let node = tools::require("node")?;
            let npm = tools::npm(&node)?;
            let mut args = vec![
                npm.as_os_str().to_owned(),
                "install".into(),
                "--prefix".into(),
                directory.as_os_str().to_owned(),
                "--no-audit".into(),
                "--no-fund".into(),
                "--engine-strict".into(),
            ];
            args.extend(packages.iter().map(OsString::from));
            runner.run(
                &format!("Installing {}", recipe.title),
                &node,
                &args,
                &[],
                INSTALL_TIMEOUT,
            )?;
            for profile in recipe.servers() {
                let package = directory.join("node_modules").join(packages[0]);
                let script = npm_script(&package, &profile.command)?;
                launches.insert(
                    profile.name,
                    launch(&node, vec![string(&script)], &["--check", &string(&script)]),
                );
            }
        }
        Kind::Github(repo) => {
            archive::github(recipe, repo, runner, &directory)?;
            for profile in recipe.servers() {
                let executable = find_file(&directory, |p| {
                    p.file_name()
                        .is_some_and(|n| n == bin(&profile.command).as_str())
                })?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
                        .map_err(|e| e.to_string())?;
                }
                launches.insert(
                    profile.name,
                    launch(&executable, vec![], probe_args(recipe.id)),
                );
            }
        }
        Kind::Java => {
            archive::java(runner, &directory)?;
            let jar = find_file(&directory, |p| {
                p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    n.starts_with("org.eclipse.equinox.launcher_") && n.ends_with(".jar")
                })
            })?;
            let configuration = directory.join(match std::env::consts::OS {
                "windows" => "config_win",
                "macos" => "config_mac",
                _ => "config_linux",
            });
            if !configuration.join("config.ini").is_file() {
                return Err("JDT LS archive has no configuration for this operating system".into());
            }
            let mut item = launch(
                &tools::require("java")?,
                vec![
                    "-Declipse.application=org.eclipse.jdt.ls.core.id1".into(),
                    "-Dosgi.bundles.defaultStartLevel=4".into(),
                    "-Declipse.product=org.eclipse.jdt.ls.core.product".into(),
                    "-Xmx1G".into(),
                    "--add-modules=ALL-SYSTEM".into(),
                    "--add-opens".into(),
                    "java.base/java.util=ALL-UNNAMED".into(),
                    "--add-opens".into(),
                    "java.base/java.lang=ALL-UNNAMED".into(),
                    "-jar".into(),
                    string(&jar),
                ],
                &["-version"],
            );
            item.java_config = Some(configuration);
            launches.insert("java".into(), item);
        }
    }
    let record = Installed {
        recipe: recipe.id.into(),
        directory: directory.clone(),
        launches,
    };
    verify(&record, runner)?;
    runner.check()?;
    registry::save(root, &record)?;
    Ok(format!(
        "Installed and configured {}.\nFiles: {}\nExisting project settings are preserved. Use :lsp doctor {} for project requirements.",
        recipe.title,
        directory.display(),
        recipe.id
    ))
}

pub(super) fn npm_script(package: &Path, command: &str) -> Result<PathBuf, String> {
    use std::io::Read;
    let package = package.canonicalize().map_err(|e| e.to_string())?;
    let mut text = String::new();
    fs::File::open(package.join("package.json"))
        .map_err(|e| e.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() > 1024 * 1024 {
        return Err("npm package metadata exceeds size limit".into());
    }
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let entry = value["bin"]
        .as_str()
        .or_else(|| value["bin"][command].as_str())
        .ok_or_else(|| format!("Package has no {command} launcher"))?;
    let script = package
        .join(entry)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !script.starts_with(&package) || !script.is_file() {
        return Err("npm launcher points outside its package".into());
    }
    Ok(script)
}

fn find_file(root: &Path, check: impl Fn(&Path) -> bool) -> Result<PathBuf, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut found = None;
    let mut count = 0;
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            count += 1;
            if count > 50000 {
                return Err("Distribution contains too many files".into());
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() && check(&entry.path()) {
                if found.is_some() {
                    return Err("Distribution contains multiple matching launchers".into());
                }
                found = Some(entry.path());
            }
        }
    }
    found.ok_or("Installed distribution does not contain the expected launcher".into())
}
