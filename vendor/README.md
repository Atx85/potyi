# SDL build patches

These are the published `sdl3-sys` 0.6.8+SDL-3.4.14 and `sdl3-ttf-sys`
0.6.1+SDL-ttf-3.2.2 crates, with their original licenses. Cargo uses them through
the root manifest's `[patch.crates-io]` entries.

The only source change is in each crate's `build-common.rs`: when building
without frameworks, add CMake's `out/lib` and `out/lib64` directories to the
native library search paths before processing the generated pkg-config file.
That file embeds the installation prefix without escaping spaces; its parser
can therefore truncate a path such as `untitled folder` to `untitled`. The
libraries compile successfully but Rust cannot find them when linking.

The explicit paths use the actual build output directory, including spaces,
and preserve the pkg-config flags needed for system libraries and frameworks.
There are no machine-specific paths and no changes to Pötyi's runtime.

When upgrading SDL, recheck a build from a checkout containing spaces. Remove
these patches if the upstream build helper handles that case itself. Keep the
two copies of this change identical until then.
