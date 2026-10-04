# Examples and new projects

## Start a new project

Select dependencies at project creation time rather than inheriting the bundled example's manifest:

1. Inspect the installed Rust toolchain, Qt kit, target architecture, and C++ compiler. Check [crates.io](https://crates.io/crates/qtbridge) for the latest non-yanked QtBridge release compatible with those prerequisites and the user's requirements. Check its matching documentation for minimum versions and platform support. Do not assume the highest version is compatible or treat a prerelease as a stable release.
2. Read the Hello World example from the selected release's source tag or published source. Verify that the source actually matches that release; the moving `dev` branch and documentation snapshots may expose unpublished APIs. Use the matching Rust and QML files as the starting point, preserving required license notices.
3. Create the destination application's Cargo manifest with an explicit compatible release version. Do not copy the bundled Git revision, upstream workspace dependencies, repository-relative paths, or an upstream lockfile. Preserve an existing destination workspace's conventions rather than automatically inserting `[workspace]`.
4. If the task requires unreleased APIs, resolve the appropriate upstream revision at that time, record why it is needed, and pin its full commit hash. Use examples from that same commit. Avoid floating Git branches and dependency wildcards.
5. Build the destination application with its selected native kit, resolve API differences against the selected version, and retain the generated `Cargo.lock` for reproducible application builds. Report the selected QtBridge version/revision and any unavailable native prerequisites.

If current sources cannot be reached, disclose that freshness could not be verified. The bundled snapshot can support an explicitly identified snapshot-based project, but do not silently present its dependencies as current. For an existing application, preserve its dependency and lockfile unless an upgrade is requested or necessary for the task.

## Bundled reproducible snapshot

`../assets/hello_world/` contains the upstream Hello World Rust and QML sources, with their copyright and license notices preserved and the BSD license included. It demonstrates an explicitly registered singleton, a slot invoked by a QML button, and embedded QML via `include_bytes!`. There is no custom build script or automatic registration feature.

Source: https://github.com/qt/qtbridge-rust/tree/f7fe523d10357751bc90c334263188610c0382cf/apps/hello_world

The only manifest adaptations are replacing the repository-relative QtBridge dependency with the same pinned Git revision and adding an empty `[workspace]` so the copy can build independently inside another repository. This is a development snapshot of QtBridge 0.3.0, not a guarantee that a published 0.3.0 has the same API. Git is required for the dependency. Keep this manifest pinned for reproducing the documented example; it is not the dependency-selection policy for new applications.

To reproduce this historical example, copy the directory to a temporary location, configure the native environment using [development setup](development-setup.md), then run from the copy:

```sh
cargo build
cargo run
```

Clicking the button prints `Hello World!` in the terminal. Cargo generates the application lockfile; keep it in the destination application. Build outputs belong in the destination or a temporary directory, rather than the bundled asset.

The bundled starter was built successfully on Windows x64 with Rust 1.97.1, Qt 6.12.0 `msvc2022_64`, and the installed MSVC toolchain. No GUI launch was performed. MinGW, macOS, and Linux setup instructions are source-based guidance, not locally verified builds.

For additional patterns, inspect the matching revision's `apps/minimal_app` (list model, slots, signals), `apps/host_monitor` (Tokio and `QmlMethodInvoker`), or `apps/color_palette` (larger application). Do not copy their repository-relative dependency paths into standalone projects.
