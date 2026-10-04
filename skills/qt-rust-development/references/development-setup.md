# Native development prerequisites

These are host development tools and SDK components, not Cargo `[dev-dependencies]`. QtBridge belongs in `[dependencies]`; generated C++ needs a native compiler even if application code is entirely Rust.

The bundled source revision requires stable Rust 1.88 or newer and Qt 6.10 or newer. Its README lists Linux x86_64, Windows x64, and experimental macOS arm64. These are snapshot requirements, not permanent defaults for new applications. Follow [dependency selection](examples.md#start-a-new-project) before choosing prerequisites for a new project. Qt's own platform support is broader; do not infer QtBridge support from Qt support alone. Recheck the selected QtBridge release and the supported compiler/OS matrix for the selected Qt version before installing tools.

Use rustup for Rust. Install Qt Base, its private development headers, Qt Declarative/Quick, and the QML modules imported by the application. The bundled example imports `QtQuick` and `QtQuick.Controls.Fusion`. Keep Qt headers, libraries, tools, and plugins from the same installation and version. Prefer environment changes local to the development shell and prepend the chosen kit's `bin` directory to `PATH` so another Qt installation does not win discovery.

## Windows: MSVC (preferred starting point)

1. Install Visual Studio or Build Tools with **Desktop development with C++**, the MSVC x64/x86 tools, and a Windows SDK. Qt Creator is optional; it does not supply MSVC.
2. Install a compatible Qt MSVC x64 kit using the Qt installer, including the required modules. Match the compiler to the kit and Qt's version-specific support matrix.
3. Use an x64 Developer PowerShell or Native Tools environment with `cl.exe`, the linker, SDK include paths, and SDK library paths initialized. Merely putting `cl.exe` on `PATH` is insufficient.
4. Use the MSVC Rust target and select the Qt kit for this shell. Replace the sample path with the installed kit:

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
$qtKit = 'C:\Qt\6.12.0\msvc2022_64'
$env:PATH = "$qtKit\bin;$env:PATH"
Get-Command cl, link, qmake
qmake -query QT_VERSION
qmake -query QT_INSTALL_PREFIX
rustc +stable-x86_64-pc-windows-msvc -Vv
cargo +stable-x86_64-pc-windows-msvc build --target x86_64-pc-windows-msvc
cargo +stable-x86_64-pc-windows-msvc run --target x86_64-pc-windows-msvc
```

The kit's `bin` directory also provides Qt DLLs at runtime. Use a separate shell or build directory when switching kits to avoid reusing incompatible native outputs.

## Windows: MinGW alternative

Use a Qt MinGW x64 kit, the exact matching MinGW compiler supplied for that Qt kit, and Rust's `x86_64-pc-windows-gnu` toolchain/target. Both the Qt kit's `bin` and the matching MinGW `bin` must be on `PATH`; verify `g++ --version`, `qmake -query`, and `rustc -Vv`. Build with:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
$qtKit = 'C:\Qt\6.12.0\mingw_64'
$mingwBin = 'C:\Qt\Tools\<matching-mingw-directory>\bin'
$env:PATH = "$qtKit\bin;$mingwBin;$env:PATH"
Get-Command g++, qmake
g++ --version
qmake -query
rustc +stable-x86_64-pc-windows-gnu -Vv
cargo +stable-x86_64-pc-windows-gnu build --target x86_64-pc-windows-gnu
cargo +stable-x86_64-pc-windows-gnu run --target x86_64-pc-windows-gnu
```

MSVC and MinGW are alternatives: keep the Rust target, generated C++ compiler, and Qt binaries ABI-compatible. Qt supporting MinGW does not establish that every QtBridge dependency/revision has been verified with it; confirm the chosen revision builds before recommending this route as validated. Avoid mixing arbitrary MSYS2 compilers with installer Qt kits or using an MSVC Qt kit with the GNU Rust target.

## macOS

Install the Xcode version and macOS SDK required by the selected Qt release, launch Xcode to finish setup, and verify `xcode-select -p` and `xcrun clang++ --version`. Install a compatible Qt macOS kit. For the documented QtBridge arm64 platform, use native Apple Silicon Rust (`aarch64-apple-darwin`) and Qt libraries containing that architecture.

For an installer kit, replace the path below:

```sh
export QT_KIT="$HOME/Qt/6.12.0/macos"
export PATH="$QT_KIT/bin:$PATH"
export DYLD_FRAMEWORK_PATH="$QT_KIT/lib${DYLD_FRAMEWORK_PATH:+:$DYLD_FRAMEWORK_PATH}"
qmake -query
rustc -Vv
cargo build
cargo run
```

Homebrew Qt is an alternative when its current version and private headers satisfy the chosen bridge; obtain its prefix with `brew --prefix qt` rather than hardcoding Intel or Apple Silicon paths. Verify prerequisites before choosing it. Do not assume command line tools alone satisfy Qt's full Xcode requirements.

## Linux

Install a native C++ toolchain (GCC/G++ or Clang), Rust, and a compatible Qt development installation. Use distro packages only when their Qt version meets the selected bridge's minimum.

The upstream README lists these Qt development packages for Ubuntu 26.04 / Debian Testing (Forky):

```sh
sudo apt install qt6-base-dev qt6-base-private-dev qt6-declarative-dev
```

For Fedora 43/44 it lists:

```sh
sudo dnf install qt6-qtbase-devel qt6-qtbase-private-devel qt6-qtdeclarative-devel
```

Check repository versions first. Install the distro's C++ compiler tools and runtime QML packages separately as needed, including Qt Quick Controls and its Fusion style for this example; development package installation does not guarantee all runtime imports are available. On older distros, use a compatible installer kit or source build instead of lowering the bridge's requirements.

For a separate Qt installation:

```sh
export QT_KIT="$HOME/Qt/6.12.0/gcc_64"
export PATH="$QT_KIT/bin:$PATH"
export LD_LIBRARY_PATH="$QT_KIT/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
qmake -query
c++ --version
rustc -Vv
cargo build
cargo run
```

Distro tools may be named `qmake6` or live outside the default `PATH`; make the Qt 6 `qmake` discoverable as required by the selected build utilities. A desktop session and working platform plugin/graphics libraries are needed to launch the GUI.

## Diagnose the first failing stage

- Discovery: verify the chosen `qmake` executable, Qt version, prefix, and installed development/private headers.
- Native compilation or linking: verify compiler initialization, target architecture and ABI; inspect stale `CC`/`CXX` overrides when changing kits.
- Startup: check Qt DLL/shared library discovery, platform plugins, and QML imports. Set `QT_DEBUG_PLUGINS=1` or `QML_IMPORT_TRACE=1` temporarily when the corresponding diagnostics are needed. Avoid global plugin path overrides that mix installations.

Build the affected app after changes. Distinguish a successful build from a successful GUI launch, and report missing prerequisites or unverified platforms precisely.

## Authoritative references

- [Pinned QtBridge README](https://github.com/qt/qtbridge-rust/blob/f7fe523d10357751bc90c334263188610c0382cf/README.md)
- [Qt Windows requirements](https://doc.qt.io/qt-6/windows.html)
- [Qt macOS requirements](https://doc.qt.io/qt-6/macos.html)
- [Qt Linux requirements](https://doc.qt.io/qt-6/linux.html)
- [Rust MSVC prerequisites](https://rust-lang.github.io/rustup/installation/windows-msvc.html)
