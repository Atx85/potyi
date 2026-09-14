// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::{catalog::Recipe, process::Runner, tools};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path},
    time::Duration,
};

const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;
const MAX_EXPANDED: u64 = 1024 * 1024 * 1024;

pub(super) fn download(
    runner: &mut Runner,
    url: &str,
    path: &Path,
    limit: u64,
) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Download requires an HTTPS URL".into());
    }
    let args = vec![
        "--fail".into(),
        "--location".into(),
        "--silent".into(),
        "--show-error".into(),
        "--proto".into(),
        "=https".into(),
        "--proto-redir".into(),
        "=https".into(),
        "--connect-timeout".into(),
        "20".into(),
        "--max-time".into(),
        "300".into(),
        "--max-filesize".into(),
        limit.to_string().into(),
        "--user-agent".into(),
        "Potyi-LSP-Setup".into(),
        "--output".into(),
        path.as_os_str().to_owned(),
        url.into(),
    ];
    runner.run(
        "Downloading from the server's official distribution",
        &tools::require("curl")?,
        &args,
        &[],
        Duration::from_secs(320),
    )?;
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > limit {
        return Err("Download exceeds size limit".into());
    }
    Ok(())
}

fn text(runner: &mut Runner, url: &str, name: &str) -> Result<String, String> {
    let path = runner.directory.join(name);
    download(runner, url, &path, 2 * 1024 * 1024)?;
    fs::read_to_string(path).map_err(|e| e.to_string())
}

pub(super) fn asset_matches(id: &str, name: &str, os: &str, arch: &str) -> bool {
    match id {
        "clangd" => {
            if arch != "x86_64" && !(os == "macos" && arch == "aarch64") {
                return false;
            }
            let platform = match os {
                "windows" => "windows",
                "macos" => "mac",
                "linux" => "linux",
                _ => return false,
            };
            name.starts_with(&format!("clangd-{platform}-")) && name.ends_with(".zip")
        }
        "lua" => {
            let platform = match os {
                "windows" => "win32",
                "macos" => "darwin",
                "linux" => "linux",
                _ => return false,
            };
            let arch = match arch {
                "x86_64" => "x64",
                "aarch64" => "arm64",
                _ => return false,
            };
            name.starts_with("lua-language-server-")
                && (name.ends_with(&format!("-{platform}-{arch}.zip"))
                    || name.ends_with(&format!("-{platform}-{arch}.tar.gz")))
        }
        "luau" => match (os, arch) {
            ("windows", "x86_64") => name == "luau-lsp-win64.zip",
            ("macos", "x86_64" | "aarch64") => name == "luau-lsp-macos.zip",
            ("linux", "x86_64") => name == "luau-lsp-linux-x86_64.zip",
            ("linux", "aarch64") => name == "luau-lsp-linux-arm64.zip",
            _ => false,
        },
        _ => false,
    }
}

pub(super) fn github(
    recipe: &Recipe,
    repo: &str,
    runner: &mut Runner,
    destination: &Path,
) -> Result<(), String> {
    let response = text(
        runner,
        &format!("https://api.github.com/repos/{repo}/releases/latest"),
        "release.json",
    )?;
    let release: serde_json::Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;
    let assets = release["assets"].as_array().ok_or(
        "Official release metadata has no assets; GitHub may be rate limited. Retry later.",
    )?;
    let matches: Vec<_> = assets
        .iter()
        .filter(|a| {
            a["name"].as_str().is_some_and(|n| {
                asset_matches(recipe.id, n, std::env::consts::OS, std::env::consts::ARCH)
            })
        })
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "No unambiguous {} download for {} / {}. Install the native server manually using {} and add it to PATH, then retry.",
            recipe.title,
            std::env::consts::OS,
            std::env::consts::ARCH,
            recipe.url
        ));
    }
    let asset = matches[0];
    if asset["size"].as_u64().is_none_or(|s| s > MAX_DOWNLOAD) {
        return Err("Official archive exceeds the download limit".into());
    }
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("Missing download URL")?;
    if !url.starts_with(&format!("https://github.com/{repo}/releases/download/")) {
        return Err("Unexpected release download origin".into());
    }
    let path = runner
        .directory
        .join(if asset["name"].as_str().unwrap().ends_with(".zip") {
            "download.zip"
        } else {
            "download.tar.gz"
        });
    download(runner, url, &path, MAX_DOWNLOAD)?;
    if let Some(digest) = asset["digest"]
        .as_str()
        .and_then(|d| d.strip_prefix("sha256:"))
    {
        verify_digest(&path, digest)?;
    }
    extract(runner, &path, destination)
}

pub(super) fn verify_digest(path: &Path, expected: &str) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        != expected.to_ascii_lowercase()
    {
        return Err("Downloaded archive checksum does not match the official release".into());
    }
    Ok(())
}

pub(super) fn latest_milestone(page: &str) -> Option<String> {
    let pattern = regex::Regex::new(r#"href=["'][^"']*?/([0-9]+\.[0-9]+\.[0-9]+)/?["']"#).unwrap();
    pattern
        .captures_iter(page)
        .filter_map(|c| {
            let name = c[1].to_string();
            let parts = name
                .split('.')
                .map(str::parse::<u64>)
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            Some((parts, name))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|p| p.1)
}

pub(super) fn java(runner: &mut Runner, destination: &Path) -> Result<(), String> {
    let base = "https://download.eclipse.org/jdtls/milestones/";
    let listing = text(runner, base, "milestones.html")?;
    let version=latest_milestone(&listing).ok_or("Could not find the latest JDT LS milestone; see https://download.eclipse.org/jdtls/milestones/")?;
    let listing = text(runner, &format!("{base}{version}/"), "milestone.html")?;
    let pattern = regex::Regex::new(r"jdt-language-server-[0-9A-Za-z.-]+\.tar\.gz").unwrap();
    let name = pattern
        .find(&listing)
        .ok_or("Could not locate the JDT LS archive in the milestone")?
        .as_str();
    let path = runner.directory.join("download.tar.gz");
    download(
        runner,
        &format!("{base}{version}/{name}"),
        &path,
        MAX_DOWNLOAD,
    )?;
    extract(runner, &path, destination)
}

pub(super) fn safe_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        && !path.to_string_lossy().contains(['\\', ':'])
}

fn copy_limited(
    input: &mut impl Read,
    output: &mut impl Write,
    size: u64,
    runner: &Runner,
) -> Result<(), String> {
    let mut copied = 0u64;
    let mut bytes = [0; 65536];
    loop {
        runner.check()?;
        let n = input.read(&mut bytes).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        copied += n as u64;
        if copied > size {
            return Err("Archive entry exceeds its declared size".into());
        }
        output.write_all(&bytes[..n]).map_err(|e| e.to_string())?;
    }
    if copied != size {
        return Err("Truncated archive entry".into());
    }
    Ok(())
}

pub(super) fn extract(runner: &Runner, archive: &Path, destination: &Path) -> Result<(), String> {
    (runner.report)("Unpacking the language server".into());
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    let mut total = 0u64;
    if archive.extension().is_some_and(|e| e == "zip") {
        let mut zip = zip::ZipArchive::new(File::open(archive).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if zip.len() > 50000 {
            return Err("Archive contains too many files".into());
        }
        for i in 0..zip.len() {
            runner.check()?;
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            let path = entry
                .enclosed_name()
                .ok_or("Unsafe archive path")?
                .to_path_buf();
            if !safe_path(&path) || entry.is_symlink() {
                return Err("Unsafe archive path or symbolic link".into());
            }
            total = total
                .checked_add(entry.size())
                .ok_or("Archive size overflow")?;
            if total > MAX_EXPANDED {
                return Err("Expanded archive exceeds size limit".into());
            }
            let path = destination.join(path);
            if entry.is_dir() {
                fs::create_dir_all(path).map_err(|e| e.to_string())?;
                continue;
            }
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            let size = entry.size();
            copy_limited(&mut entry, &mut output, size, runner)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(entry.unix_mode().unwrap_or(0o644) & 0o777),
                )
                .map_err(|e| e.to_string())?;
            }
        }
    } else {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(
            File::open(archive).map_err(|e| e.to_string())?,
        ));
        for (i, entry) in tar.entries().map_err(|e| e.to_string())?.enumerate() {
            runner.check()?;
            if i >= 50000 {
                return Err("Archive contains too many files".into());
            }
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?.into_owned();
            if !safe_path(&path) {
                return Err("Unsafe archive path".into());
            }
            let kind = entry.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err("Archive links and special files are not supported".into());
            }
            let size = entry.size();
            total = total.checked_add(size).ok_or("Archive size overflow")?;
            if total > MAX_EXPANDED {
                return Err("Expanded archive exceeds size limit".into());
            }
            let path = destination.join(path);
            if kind.is_dir() {
                fs::create_dir_all(path).map_err(|e| e.to_string())?;
                continue;
            }
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            copy_limited(&mut entry, &mut output, size, runner)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(
                        entry.header().mode().map_err(|e| e.to_string())? & 0o777,
                    ),
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
