---
name: qt-rust-development
description: Create, edit, or troubleshoot Rust applications using QtBridge with Qt Quick and QML, including native development setup, QObject APIs, models, and threading. Use when the project uses QtBridge or the user requests it.
---

# QtBridge for Rust

Use QtBridge to keep application logic in Rust and build the interface in QML/Qt Quick. The bridge generates Qt wrappers at compile time; application code normally uses Rust APIs and QML without writing C++ or hand-maintained FFI.

## Establish the project constraints

- For native prerequisites or a first build, read [development setup](references/development-setup.md). For a new project, follow [examples](references/examples.md) to select a current compatible QtBridge version and use its matching upstream example. The bundled `assets/hello_world` is a historical reference; its manifest is only for reproducing that snapshot.
- Inspect `Cargo.toml`, `Cargo.lock`, the installed Qt version, and existing QML imports before proposing API changes. The linked QtBridge docs may describe a newer release than the project uses; treat the project's pinned crate and matching documentation/examples as authoritative for exact syntax and availability.
- QtBridge is a Qt Quick/QML bridge. For Qt Widgets, direct C++ integration, or Qt modules that expose only a C++ API, assess CXX-Qt instead of forcing those requirements through QtBridge.
- Before troubleshooting build setup, identify the target OS, Rust toolchain, Qt installation, C++ compiler, and `qmake` visible on `PATH`. Consult Qt's platform requirements and the project's actual QtBridge version. Current docs describe Qt 6.10+ and stable Rust 1.88+, but those are version-sensitive requirements, not assumptions for older releases.
- Treat the bridge as beta/pre-release where the selected release or Qt terms say so. Do not promise stable API or production guarantees beyond the project's version and license terms.

## Shape the Rust/QML boundary

- Keep application state and business logic in Rust; use QML for declarative layout, Qt Quick controls, and view-level interaction. Use JavaScript in QML only where it suits UI behavior, not as a second backend.
- Start the app with `QApp`, register the QML-facing Rust types as required by the selected version, load the QML/resource entry point, then run the app. Check the existing project and examples for the exact initialization flow.
- Expose only the API QML needs. Use `#[qobject]` on the appropriate `impl` or module and the version-supported `#[qproperty]`, `#[qslot]`, and `#[qsignal]` annotations. Use the crate's traits for special QML roles and model/view integration instead of inventing parallel mechanisms.
- Keep names and types clear across the boundary. Check QML imports, registered module/type names, property names, slot signatures, and signal handlers together whenever changing either side.
- In the bundled version, the QML module URI derives from the Cargo package name: leading numeric characters are removed and non-alphanumeric characters become underscores. The starter package `hello_world` therefore uses `import hello_world`; update the import when renaming the package.
- Prefer Qt Quick's model/view patterns for collections. Consult the crate's `QListModel`, `QTableModel`, item traits, and examples for the pinned version before implementing a custom collection interface.

## Respect ownership, borrowing, and threads

- QtBridge QObjects are shared between Rust and QML. The documented implementation uses `Rc<RefCell<T>>`; QML references can share the same object, while Rust's shared/exclusive borrow rules are enforced at runtime.
- Keep borrows short and avoid re-entering an object through a fresh independent borrow while a mutable borrow is active. A borrow conflict can panic at runtime, especially when a callback or interrupted call chain re-enters the same object. Prefer a clear sequence that releases the borrow before invoking code that may call back into it.
- A `&self` slot/read uses a shared borrow; a mutable property write or `&mut self` slot needs an exclusive borrow. Choose mutability based on actual state changes and trace callback paths when diagnosing borrow panics.
- Do not assume a QObject is `Send` or `Sync` because its handle is shared. For worker threads or async runtimes, use the documented thread-safe `QmlMethodInvoker` mechanism to invoke slots/signals on the Qt side, following the matching example. Keep Qt/QML object access on the supported thread/context.

## Build and diagnose

- QtBridge is built with Cargo but currently needs a compatible C++ toolchain, Qt development components (including private development headers where required), and `qmake` on `PATH`. Platform libraries and Qt plugin/module paths may also need configuration.
- Distinguish Rust compilation failures, generated bridge/C++ build failures, Qt discovery/link failures, and QML runtime/import errors. Use the first relevant diagnostic and verify it against the pinned crate's build requirements.
- For QML runtime failures, check module imports, QML type registration, resource paths, and Qt Quick module installation. Rust IDE analysis and QML language-server type recognition may not share generated type information; an IDE warning alone does not prove the application fails to build.
- Use the repository's established build/run commands and existing examples as the baseline. When asked to verify a change, build the affected application and launch it if the environment supports GUI execution; report any unavailable platform prerequisite precisely.
- Only enable optional crate features when their behavior is needed. In the current docs, `linkme` enables automatic QML type registration and `serde_json` enables JSON values across properties/signals/slots; confirm feature names and semantics for the pinned release.

## Source references

- [QtBridge source](https://github.com/qt/qtbridge-rust/tree/dev) and [in-tree Hello World](https://github.com/qt/qtbridge-rust/tree/dev/apps/hello_world) — implementation and examples for development versions. The bundled example records an immutable revision; do not assume the moving `dev` branch matches a released crate.
- [Qt Bridge Rust crate documentation](https://doc-snapshots.qt.io/qtbridge-rust/qtbridge/index.html) — API overview, setup, features, examples, ownership, threading, and model traits. The snapshot version may advance; check the project version.
- [A Cross-Platform Rust UI Framework via Qt’s Bridging Technology](https://www.qt.io/blog/rust-ui-framework-via-bridging-technology) — design intent, compile-time wrapper generation, division between Rust logic and QML UI, and beta status at publication.
- [Qt Bridge Rust examples](https://github.com/qt/qtbridge-rust-examples) — runnable patterns; compare examples with the dependency version in use.
