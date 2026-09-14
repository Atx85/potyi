// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Install recipes are shipped with the editor, never read from a project.
use crate::lsp::{Config, ServerConfig};

#[derive(Clone, Copy, Debug)]
pub(super) enum Kind {
    Rustup,
    Dotnet,
    Python,
    Go(&'static str),
    Npm(&'static [&'static str]),
    Github(&'static str),
    Java,
    Cargo,
}

pub(crate) struct Recipe {
    pub id: &'static str,
    pub title: &'static str,
    pub aliases: &'static [&'static str],
    pub profiles: &'static [&'static str],
    pub(super) kind: Kind,
    pub url: &'static str,
}

pub(crate) const RECIPES: &[Recipe] = &[
    Recipe {
        id: "rust",
        title: "Rust · rust-analyzer",
        aliases: &["rust-analyzer"],
        profiles: &["rust"],
        kind: Kind::Rustup,
        url: "https://rust-analyzer.github.io/book/installation.html",
    },
    Recipe {
        id: "clangd",
        title: "C and C++ · clangd",
        aliases: &["c", "cpp", "c++", "clangd-c", "clangd-cpp"],
        profiles: &["clangd-c", "clangd-cpp"],
        kind: Kind::Github("clangd/clangd"),
        url: "https://clangd.llvm.org/installation",
    },
    Recipe {
        id: "csharp",
        title: "C# and Unity · csharp-ls",
        aliases: &["c#", "cs", "unity", "csharp-ls"],
        profiles: &["csharp"],
        kind: Kind::Dotnet,
        url: "https://github.com/razzmatazz/csharp-language-server",
    },
    Recipe {
        id: "python",
        title: "Python · python-lsp-server",
        aliases: &["py", "pylsp", "python-lsp-server"],
        profiles: &["python"],
        kind: Kind::Python,
        url: "https://github.com/python-lsp/python-lsp-server",
    },
    Recipe {
        id: "typescript",
        title: "JavaScript, JSX, TypeScript and TSX",
        aliases: &[
            "javascript",
            "js",
            "jsx",
            "ts",
            "tsx",
            "typescript-language-server",
        ],
        profiles: &["javascript", "jsx", "typescript", "tsx"],
        kind: Kind::Npm(&["typescript-language-server", "typescript@6"]),
        url: "https://github.com/typescript-language-server/typescript-language-server",
    },
    Recipe {
        id: "go",
        title: "Go · gopls",
        aliases: &["gopls", "golang"],
        profiles: &["go"],
        kind: Kind::Go("golang.org/x/tools/gopls@latest"),
        url: "https://go.dev/gopls/",
    },
    Recipe {
        id: "java",
        title: "Java · Eclipse JDT Language Server",
        aliases: &["jdtls", "eclipse-jdtls"],
        profiles: &["java"],
        kind: Kind::Java,
        url: "https://github.com/eclipse-jdtls/eclipse.jdt.ls",
    },
    Recipe {
        id: "php",
        title: "PHP · Intelephense",
        aliases: &["intelephense"],
        profiles: &["php"],
        kind: Kind::Npm(&["intelephense"]),
        url: "https://github.com/bmewburn/intelephense-docs",
    },
    Recipe {
        id: "bash",
        title: "Shell · Bash Language Server",
        aliases: &["shell", "sh", "bash-language-server"],
        profiles: &["bash"],
        kind: Kind::Npm(&["bash-language-server"]),
        url: "https://github.com/bash-lsp/bash-language-server",
    },
    Recipe {
        id: "lua",
        title: "Lua · Lua Language Server",
        aliases: &["lua-language-server", "luals"],
        profiles: &["lua"],
        kind: Kind::Github("LuaLS/lua-language-server"),
        url: "https://github.com/LuaLS/lua-language-server",
    },
    Recipe {
        id: "luau",
        title: "Luau · luau-lsp",
        aliases: &["luau-lsp"],
        profiles: &["luau"],
        kind: Kind::Github("JohnnyMorganz/luau-lsp"),
        url: "https://github.com/JohnnyMorganz/luau-lsp",
    },
    Recipe {
        id: "web",
        title: "HTML, CSS, JSON and JSONC",
        aliases: &[
            "html",
            "css",
            "json",
            "jsonc",
            "vscode-langservers-extracted",
            "vscode-html-language-server",
            "vscode-css-language-server",
            "vscode-json-language-server",
        ],
        profiles: &["html", "css", "json", "jsonc"],
        kind: Kind::Npm(&["vscode-langservers-extracted"]),
        url: "https://github.com/hrsh7th/vscode-langservers-extracted",
    },
    Recipe {
        id: "yaml",
        title: "YAML · YAML Language Server",
        aliases: &["yml", "yaml-language-server"],
        profiles: &["yaml"],
        kind: Kind::Npm(&["yaml-language-server"]),
        url: "https://github.com/redhat-developer/yaml-language-server",
    },
    Recipe {
        id: "toml",
        title: "TOML · Taplo",
        aliases: &["taplo", "taplo-cli"],
        profiles: &["toml"],
        kind: Kind::Cargo,
        url: "https://taplo.tamasfe.dev/cli/installation/cargo.html",
    },
    Recipe {
        id: "sql",
        title: "SQL · sqls",
        aliases: &["sqls"],
        profiles: &["sql"],
        kind: Kind::Go("github.com/sqls-server/sqls@latest"),
        url: "https://github.com/sqls-server/sqls",
    },
];

pub(crate) fn find(name: &str) -> Result<&'static Recipe, String> {
    RECIPES.iter().find(|r| r.id.eq_ignore_ascii_case(name) || r.aliases.iter().any(|a| a.eq_ignore_ascii_case(name)))
        .ok_or_else(|| if name.eq_ignore_ascii_case("vue") {
            "Vue's server needs a TypeScript bridge Potyi does not implement yet. JavaScript/TypeScript setup is available with :lsp install typescript.".into()
        } else { format!("Unknown supported server '{name}'. Choose: {}", RECIPES.iter().map(|r| r.id).collect::<Vec<_>>().join(", ")) })
}

pub(crate) fn profiles() -> Vec<ServerConfig> {
    static PROFILES: std::sync::OnceLock<Vec<ServerConfig>> = std::sync::OnceLock::new();
    PROFILES
        .get_or_init(|| {
            toml::from_str::<Config>(include_str!("servers.toml"))
                .expect("bundled installer profiles must be valid")
                .servers
        })
        .clone()
}

impl Recipe {
    pub(super) fn servers(&self) -> Vec<ServerConfig> {
        profiles()
            .into_iter()
            .filter(|s| self.profiles.contains(&s.name.as_str()))
            .collect()
    }
}

pub(super) fn for_file(path: &std::path::Path) -> Option<&'static Recipe> {
    let extension = path.extension()?.to_str()?;
    let profile = profiles().into_iter().find(|s| {
        s.extensions
            .iter()
            .any(|e| e.eq_ignore_ascii_case(extension))
    })?;
    RECIPES
        .iter()
        .find(|r| r.profiles.contains(&profile.name.as_str()))
}
