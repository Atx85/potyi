// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "potyi setup test {} {}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn runner(&self) -> process::Runner {
        process::Runner::new(self.0.clone(), Arc::new(AtomicBool::new(false)), |_| {})
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn documented_profiles(guide: &str) -> crate::lsp::Config {
    // Windows checkouts can use CRLF, including in include_str! input.
    let guide = guide.replace("\r\n", "\n");
    let entries = guide
        .split("```toml\n")
        .skip(1)
        .map(|p| p.split_once("```").unwrap().0)
        .collect::<Vec<_>>()
        .join("\n");
    toml::from_str(&entries).unwrap()
}

#[test]
fn documented_profiles_accept_lf_and_windows_crlf() {
    let lf = include_str!("../../docs/lsp.md").replace("\r\n", "\n");
    let documented = documented_profiles(&lf);
    assert_eq!(documented.servers.len(), 22);
    let crlf = lf.replace('\n', "\r\n");
    assert_eq!(
        format!("{:?}", documented_profiles(&crlf)),
        format!("{documented:?}"),
        "line endings must not change the documented server configuration"
    );
}

#[test]
fn catalogue_covers_every_documented_profile_and_alias() {
    let documented = documented_profiles(include_str!("../../docs/lsp.md"));
    let profiles = catalog::profiles();
    assert_eq!(profiles.len(), 22);
    assert_eq!(catalog::RECIPES.len(), 15);
    let mut names = HashSet::new();
    for recipe in catalog::RECIPES {
        assert_eq!(
            catalog::find(&recipe.id.to_uppercase()).unwrap().id,
            recipe.id
        );
        assert_eq!(recipe.servers().len(), recipe.profiles.len());
        for alias in recipe.aliases {
            assert_eq!(catalog::find(alias).unwrap().id, recipe.id);
        }
        for server in recipe.servers() {
            assert!(names.insert(server.name.clone()));
            let original = documented
                .servers
                .iter()
                .find(|s| s.name == server.name)
                .unwrap();
            assert_eq!(server.extensions, original.extensions);
            assert_eq!(server.language_id, original.language_id);
            for ext in &server.extensions {
                assert_eq!(
                    catalog::for_file(Path::new(&format!("file.{ext}")))
                        .unwrap()
                        .id,
                    recipe.id
                );
            }
        }
    }
    assert_eq!(names.len(), documented.servers.len());
    assert!(catalog::find("vue").err().unwrap().contains("bridge"));
    assert!(catalog::find("x; echo hello").is_err());
    assert!(catalog::for_file(Path::new("hello.txt")).is_none());
}

#[test]
fn downloads_match_platform_and_architecture_without_ambiguous_tools() {
    for (os, arch, clangd, lua, luau) in [
        (
            "windows",
            "x86_64",
            "clangd-windows-22.1.6.zip",
            "lua-language-server-3.19.1-win32-x64.zip",
            "luau-lsp-win64.zip",
        ),
        (
            "linux",
            "x86_64",
            "clangd-linux-22.1.6.zip",
            "lua-language-server-3.19.1-linux-x64.tar.gz",
            "luau-lsp-linux-x86_64.zip",
        ),
        (
            "macos",
            "aarch64",
            "clangd-mac-22.1.6.zip",
            "lua-language-server-3.19.1-darwin-arm64.tar.gz",
            "luau-lsp-macos.zip",
        ),
        (
            "macos",
            "x86_64",
            "clangd-mac-22.1.6.zip",
            "lua-language-server-3.19.1-darwin-x64.tar.gz",
            "luau-lsp-macos.zip",
        ),
    ] {
        for (id, asset) in [("clangd", clangd), ("lua", lua), ("luau", luau)] {
            assert!(archive::asset_matches(id, asset, os, arch), "{asset}");
            assert!(!archive::asset_matches(id, asset, "unknown", arch));
            assert!(!archive::asset_matches(id, asset, os, "riscv64"));
        }
    }
    assert!(!archive::asset_matches(
        "clangd",
        "clangd_indexing_tools-windows-22.1.6.zip",
        "windows",
        "x86_64"
    ));
    assert!(!archive::asset_matches(
        "lua",
        "lua-language-server-3.19.1-win32-ia32.zip",
        "windows",
        "x86_64"
    ));
    assert!(!archive::asset_matches(
        "clangd",
        "clangd-linux-22.1.6.zip",
        "linux",
        "aarch64"
    ));
    assert_eq!(
        archive::latest_milestone(
            "<a href='/jdtls/milestones/1.9.0'>old</a><a href='/jdtls/milestones/1.61.0/'>new</a>"
        ),
        Some("1.61.0".into())
    );
    assert!(archive::latest_milestone("rate limit").is_none());
}

fn fixture_record(recipe: &catalog::Recipe, root: &Path) -> registry::Installed {
    let command = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    registry::Installed {
        recipe: recipe.id.into(),
        directory: root.into(),
        launches: recipe
            .profiles
            .iter()
            .map(|n| {
                (
                    n.to_string(),
                    registry::Launch {
                        command: command.clone(),
                        prefix: vec!["managed prefix".into()],
                        probe: vec!["--list".into()],
                        java_config: None,
                    },
                )
            })
            .collect(),
    }
}

#[test]
fn managed_configuration_preserves_custom_commands_arguments_and_precedence() {
    let temp = Temp::new();
    for recipe in catalog::RECIPES {
        registry::save(&temp.0, &fixture_record(recipe, &temp.0)).unwrap();
    }
    let mut config: crate::lsp::Config = toml::from_str(crate::embedded_config::LSP).unwrap();
    let mut custom = catalog::profiles()
        .into_iter()
        .find(|s| s.name == "yaml")
        .unwrap();
    custom.name = "my-yaml".into();
    custom.command = "my-custom-server".into();
    config.servers.insert(0, custom);
    let rust = config
        .servers
        .iter_mut()
        .find(|s| s.name == "rust")
        .unwrap();
    rust.command = "/custom/rust-analyzer".into();
    let csharp = config
        .servers
        .iter_mut()
        .find(|s| s.name == "csharp")
        .unwrap();
    csharp.args = vec!["--solution".into(), "My Game.sln".into()];
    csharp.initialization_options = serde_json::json!({"keep": true});
    registry::augment_from(&mut config, &temp.0);
    assert!(!config.enabled);
    assert_eq!(
        config.server_for(Path::new("a.yaml")).unwrap().name,
        "my-yaml"
    );
    assert_eq!(
        config.server_for(Path::new("a.rs")).unwrap().command,
        "/custom/rust-analyzer"
    );
    let server = config.server_for(Path::new("a.cs")).unwrap();
    assert_eq!(server.args, ["managed prefix", "--solution", "My Game.sln"]);
    assert_eq!(
        server.initialization_options,
        serde_json::json!({"keep": true})
    );
    assert_eq!(config.servers.len(), 23);
    let before = format!("{config:?}");
    registry::augment_from(&mut config, &temp.0);
    assert_eq!(
        format!("{config:?}"),
        before,
        "loading twice must not duplicate prefixes"
    );
}

#[test]
fn installation_reuse_releases_windows_lock_and_cancel_keeps_previous_record() {
    let temp = Temp::new();
    let mut runner = temp.runner();
    let recipe = catalog::find("yaml").unwrap();
    let record = fixture_record(recipe, &temp.0);
    registry::save(&temp.0, &record).unwrap();
    let before = fs::read(temp.0.join("yaml.json")).unwrap();
    for _ in 0..2 {
        assert!(
            install::install(recipe, &temp.0, &mut runner)
                .unwrap()
                .contains("already installed")
        );
        assert!(!temp.0.join("yaml.lock").exists());
    }
    runner.cancel.store(true, Ordering::Relaxed);
    assert!(
        install::install(recipe, &temp.0, &mut runner)
            .unwrap_err()
            .contains("cancelled")
    );
    assert!(!temp.0.join("yaml.lock").exists());
    assert_eq!(fs::read(temp.0.join("yaml.json")).unwrap(), before);
    registry::save(&temp.0, &record).unwrap(); // replacement is also supported on Windows
    let mut invalid = record;
    invalid.launches = BTreeMap::new();
    registry::save(&temp.0, &invalid).unwrap();
    assert!(registry::read(&temp.0, recipe).is_err());
}

#[test]
fn npm_bin_scripts_accept_spaces_and_reject_package_escape() {
    let temp = Temp::new();
    let package = temp.0.join("node_modules/server package");
    fs::create_dir_all(package.join("bin")).unwrap();
    fs::write(package.join("bin/server.js"), "// test").unwrap();
    for bin in [
        serde_json::json!("bin/server.js"),
        serde_json::json!({"server": "bin/server.js"}),
    ] {
        fs::write(
            package.join("package.json"),
            serde_json::json!({"bin": bin}).to_string(),
        )
        .unwrap();
        assert_eq!(
            install::npm_script(&package, "server").unwrap(),
            package.join("bin/server.js").canonicalize().unwrap()
        );
    }
    fs::write(temp.0.join("outside.js"), "// test").unwrap();
    fs::write(
        package.join("package.json"),
        r#"{"bin":{"server":"../../outside.js"}}"#,
    )
    .unwrap();
    assert!(
        install::npm_script(&package, "server")
            .unwrap_err()
            .contains("outside")
    );
}

#[test]
fn archives_extract_and_check_integrity_and_reject_traversal_and_links() {
    use std::io::Write;
    let temp = Temp::new();
    let runner = temp.runner();
    for name in ["../evil", "/absolute", "C:/evil", "folder\\evil"] {
        assert!(!archive::safe_path(Path::new(name)));
    }
    let path = temp.0.join("bundle.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    zip.start_file(
        "bin/server",
        zip::write::SimpleFileOptions::default().unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(b"hello").unwrap();
    zip.finish().unwrap();
    archive::extract(&runner, &path, &temp.0.join("out")).unwrap();
    assert_eq!(fs::read(temp.0.join("out/bin/server")).unwrap(), b"hello");
    archive::verify_digest(
        &temp.0.join("out/bin/server"),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .unwrap();
    assert!(archive::verify_digest(&path, "0000").is_err());
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    zip.start_file("../escape", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"bad").unwrap();
    zip.finish().unwrap();
    assert!(archive::extract(&runner, &path, &temp.0.join("bad")).is_err());
    assert!(!temp.0.join("escape").exists());
    let path = temp.0.join("bundle.tar.gz");
    let gzip = flate2::write::GzEncoder::new(
        fs::File::create(&path).unwrap(),
        flate2::Compression::default(),
    );
    let mut tar = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.set_size(5);
    header.set_mode(0o755);
    header.set_cksum();
    tar.append_data(&mut header, "bin/server", &b"hello"[..])
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    archive::extract(&runner, &path, &temp.0.join("tar")).unwrap();
    assert_eq!(fs::read(temp.0.join("tar/bin/server")).unwrap(), b"hello");
    let gzip = flate2::write::GzEncoder::new(
        fs::File::create(&path).unwrap(),
        flate2::Compression::default(),
    );
    let mut tar = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o755);
    tar.append_link(&mut header, "link", "../outside").unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    assert!(archive::extract(&runner, &path, &temp.0.join("link")).is_err());
    runner.cancel.store(true, Ordering::Relaxed);
    assert!(
        archive::extract(&runner, &path, &temp.0.join("cancel"))
            .unwrap_err()
            .contains("cancelled")
    );
}

#[test]
#[ignore = "child process fixture; invoked only by runner tests"]
fn process_fixture() {
    match std::env::var("POTYI_SETUP_FIXTURE").as_deref() {
        Ok("fail") => {
            eprintln!("missing test dependency");
            std::process::exit(3);
        }
        Ok("sleep") => std::thread::sleep(Duration::from_secs(30)),
        Ok("args") => println!("{}", std::env::var("POTYI_SETUP_LITERAL").unwrap()),
        _ => (),
    }
}

#[test]
fn background_runner_reports_failure_timeout_cancellation_and_literal_arguments() {
    let temp = Temp::new();
    let mut runner = temp.runner();
    let program = std::env::current_exe().unwrap();
    let args: Vec<OsString> = [
        "--exact",
        "lsp_setup::tests::process_fixture",
        "--ignored",
        "--nocapture",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    let literal = "a space & $(echo bad); `bad` é";
    let out = runner
        .run(
            "test",
            &program,
            &args,
            &[
                ("POTYI_SETUP_FIXTURE", "args".into()),
                ("POTYI_SETUP_LITERAL", literal.into()),
            ],
            Duration::from_secs(10),
        )
        .unwrap();
    assert!(out.contains(literal));
    let error = runner
        .run(
            "test",
            &program,
            &args,
            &[("POTYI_SETUP_FIXTURE", "fail".into())],
            Duration::from_secs(10),
        )
        .unwrap_err();
    assert!(error.contains("missing test dependency"));
    let start = Instant::now();
    assert!(
        runner
            .run(
                "test",
                &program,
                &args,
                &[("POTYI_SETUP_FIXTURE", "sleep".into())],
                Duration::from_millis(200)
            )
            .unwrap_err()
            .contains("timed out")
    );
    assert!(start.elapsed() < Duration::from_secs(5));
    let cancel = runner.cancel.clone();
    let worker = std::thread::spawn(move || {
        runner.run(
            "test",
            &program,
            &args,
            &[("POTYI_SETUP_FIXTURE", "sleep".into())],
            Duration::from_secs(20),
        )
    });
    std::thread::sleep(Duration::from_millis(100));
    cancel.store(true, Ordering::Relaxed);
    assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
}

#[test]
fn setup_results_respect_dismissed_panels_and_other_documents() {
    let temp = Temp::new();
    let file = temp.0.join("file.cs");
    let mut bar = CommandBar::new();
    bar.open(":lsp install csharp");
    let mut manager = Manager::default();
    manager.active = Some((
        1,
        Arc::new(AtomicBool::new(false)),
        std::thread::spawn(|| {}),
    ));
    manager.epoch = bar.epoch();
    manager.file = Some(file.clone());
    assert_eq!(
        manager.accept(
            Event {
                id: 99,
                text: "stale".into(),
                done: true,
                installed: Some("csharp")
            },
            &mut bar,
            Some(&file)
        ),
        None
    );
    assert!(manager.last.is_none());
    bar.close();
    assert_eq!(
        manager.accept(
            Event {
                id: 1,
                text: "ready".into(),
                done: true,
                installed: Some("csharp")
            },
            &mut bar,
            Some(&file)
        ),
        None
    );
    assert!(!bar.is_active());
    assert_eq!(manager.last.as_deref(), Some("ready"));
}

#[test]
#[ignore = "downloads real servers to a temporary directory; set POTYI_LSP_SMOKE to comma-separated recipe names"]
fn live_install_and_initialize() {
    let selected = std::env::var("POTYI_LSP_SMOKE").expect("explicit smoke-test recipes required");
    let temp = Temp::new();
    for name in selected.split(',') {
        let recipe = catalog::find(name).unwrap();
        let job = temp.0.join(recipe.id);
        fs::create_dir(&job).unwrap();
        let mut runner = process::Runner::new(job, Arc::new(AtomicBool::new(false)), |text| {
            eprintln!("{text}")
        });
        eprintln!(
            "{}",
            install::install(recipe, &temp.0, &mut runner).unwrap()
        );
        let mut config = crate::lsp::Config {
            enabled: true,
            servers: vec![],
        };
        registry::augment_from(&mut config, &temp.0);
        for profile in recipe.profiles {
            let mut server = config
                .servers
                .iter()
                .find(|s| s.name == *profile)
                .unwrap()
                .clone();
            let root = temp.0.join(format!("project {profile}"));
            fs::create_dir(&root).unwrap();
            if recipe.id == "java" {
                registry::configure_from(&mut server, &root, &temp.0).unwrap();
            }
            let path = root.join(format!("example.{}", server.extensions[0]));
            fs::write(&path, "").unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            let client = crate::lsp::Client::start(server, root, move |event| {
                let _ = tx.send(event);
            })
            .unwrap();
            client
                .request(crate::lsp::Request {
                    id: 1,
                    action: crate::lsp::Action::Start,
                    documents: vec![crate::lsp::Document {
                        path,
                        text: String::new(),
                    }],
                    cursor: 0,
                })
                .unwrap();
            let reply = rx.recv_timeout(Duration::from_secs(45)).unwrap();
            assert!(reply.result.is_ok(), "{profile}: {:?}", reply.result);
            eprintln!("{profile}: initialize OK");
            drop(client);
        }
    }
}

#[test]
fn java_configuration_uses_distinct_workspaces_and_keeps_explicit_arguments() {
    let temp = Temp::new();
    let recipe = catalog::find("java").unwrap();
    let template = temp.0.join("template");
    fs::create_dir(&template).unwrap();
    fs::write(template.join("config.ini"), "configuration").unwrap();
    let mut record = fixture_record(recipe, &temp.0);
    record.launches.get_mut("java").unwrap().java_config = Some(template);
    registry::save(&temp.0, &record).unwrap();
    let mut config = crate::lsp::Config {
        enabled: false,
        servers: vec![],
    };
    registry::augment_from(&mut config, &temp.0);
    let original = config.servers.remove(0);
    let mut first = original.clone();
    let mut second = original.clone();
    registry::configure_from(&mut first, &temp.0.join("project one"), &temp.0).unwrap();
    registry::configure_from(&mut second, &temp.0.join("project two"), &temp.0).unwrap();
    assert_ne!(first.args.last(), second.args.last());
    let index = first
        .args
        .iter()
        .position(|s| s == "-configuration")
        .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&first.args[index + 1]).join("config.ini")).unwrap(),
        "configuration"
    );
    let before = first.args.clone();
    registry::configure_from(&mut first, &temp.0.join("project one"), &temp.0).unwrap();
    assert_eq!(first.args, before);
    let mut explicit = original;
    explicit
        .args
        .extend(["-data".into(), "my workspace".into()]);
    let before = explicit.args.clone();
    registry::configure_from(&mut explicit, &temp.0, &temp.0).unwrap();
    assert_eq!(explicit.args, before);
}

#[test]
#[ignore = "downloads the current Eclipse archive to verify milestone discovery and extraction"]
fn live_java_distribution() {
    let temp = Temp::new();
    let mut runner = temp.runner();
    let destination = temp.0.join("server");
    archive::java(&mut runner, &destination).unwrap();
    for platform in ["config_win", "config_mac", "config_linux"] {
        assert!(
            destination.join(platform).join("config.ini").is_file(),
            "{platform}"
        );
    }
    assert!(fs::read_dir(destination.join("plugins")).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("org.eclipse.equinox.launcher_")
    }));
}
