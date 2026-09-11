# Syntax designer

The static app is in `docs/designer/`. It works with the existing GitHub Pages
site: publish `main` → `/docs` and visit `/potyi/designer/`. The page uses only
relative asset URLs, so it also works on a custom domain or under another
repository name. No server, CDN scripts, or npm install is required.

## Build and check

Install the Rust target once:

```sh
rustup target add wasm32-unknown-unknown
```

From the repository root, using Python 3.11+ and Node 18+:

```sh
python3 tools/syntax-designer/build.py
node tools/syntax-designer/test.mjs
cargo test --locked --manifest-path tools/syntax-designer/Cargo.toml
cargo test --locked
```

The build copies the WebAssembly engine into `docs/designer/engine.wasm` and
generates `presets.json` from the real files under `config/syntax/`, plus a
starter definition. Commit both generated assets with source changes so
GitHub Pages can serve them directly. `--assets-only` regenerates the presets
and copies an already-built engine.

Preview locally:

```sh
python3 -m http.server 8765 --directory docs
```

Open `http://localhost:8765/designer/`. Opening the HTML through `file://`
does not provide the HTTP origin needed to load the WebAssembly asset.

## Compatibility and resource limits

Both the editor and designer use `src/syntax_core.rs`: the TOML schema,
UTF-8-safe line limit, matching, overlap resolution, and colors are shared.
The build verifies that both lockfiles use the same regex and TOML versions.
The small bridge in `src/lib.rs` contains no SDL or desktop dependencies.

The browser accepts at most 64 KiB of TOML, 32 rules, and 4 KiB per pattern.
Regex compilation has a 1 MiB size limit and a 128 KiB DFA cache per rule.
Samples are limited to 16 KiB and 200 displayed lines. Matching uses Rust's
non-backtracking regex engine, not JavaScript's regular expression engine.
Downloads are enabled only after Rust accepts the generated definition.

The app fetches its engine and presets once. There are no network requests
for imports, samples, colors, or downloads; no persistent worker, analytics,
storage, polling, or animation loop. A short debounce runs only after edits.
Users must download their work before reloading or changing presets.

Tests cover all bundled imports and export round trips, byte offsets for
Unicode, unsupported expressions, invalid input, and preview limits.
An actual browser download in `fixtures/` is also loaded by the desktop syntax
tests to verify extension detection and custom colors through the file loader.
The `Check syntax designer` workflow tests both the shipped and freshly built
engine, and checks that generated presets match the language files.
The browser UI also handles loading failures and leaves the current design
intact if importing a file fails.

## GitHub Pages

The existing site and designer are plain files in `docs/`. In the repository's
Settings → Pages, use “Deploy from a branch”, select `main` and `/docs`, and
save. After pushing the files, the designer will be at
`https://atx85.github.io/potyi/designer/` unless the site uses a custom domain.
This setting requires repository administration access. This repository does
not silently change Pages settings or publish from a local build.

The designer and shared code use the repository's GPL-3.0-or-later license.
