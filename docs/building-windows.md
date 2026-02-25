# Building the Windows Installer from Linux

Cross-compiling a Windows NSIS installer from a Linux host using `cargo-xwin`.

## Prerequisites

Install these once:

```bash
# Rust target for Windows MSVC
rustup target add x86_64-pc-windows-msvc

# cargo-xwin — cross-compile driver (downloads MSVC headers/libs automatically)
cargo install cargo-xwin

# NSIS tools — needed for the installer packaging step
sudo pacman -S nsis lld llvm    # Arch / CachyOS / Manjaro
# sudo apt install nsis lld llvm  # Ubuntu / Debian
```

## Build

```bash
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

**Do not** pass `--bundles nsis` — Tauri CLI on Linux rejects that flag regardless of version. With `bundle.targets = "all"` in `tauri.conf.json`, Tauri automatically selects NSIS when the target triple is Windows.

## Output

```
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/Joplin-Smart-Search_0.1.0_x64-setup.exe
src-tauri/target/x86_64-pc-windows-msvc/release/joplin-smart-search.exe
```

Tauri uses `productName` verbatim in the filename. The script renames it to remove spaces (e.g. `Joplin Smart Search_0.1.0_x64-setup.exe` → `Joplin-Smart-Search_0.1.0_x64-setup.exe`).

Upload the `_x64-setup.exe` to the GitHub release draft.

## Why MSVC and not MinGW?

The `ort-sys` crate (ONNX Runtime) ships pre-built binaries only for the MSVC target (`x86_64-pc-windows-msvc`). The GNU target (`x86_64-pc-windows-gnu`) has no matching pre-built binaries and would require building ONNX Runtime from source.

## Why `bundle.targets = "all"`?

Previously `tauri.conf.json` had `"targets": ["appimage"]`, which locked bundling to AppImage even when cross-compiling for Windows. `"all"` lets Tauri pick the right bundle formats for each target:

| Target triple | Bundles produced |
|---|---|
| `x86_64-unknown-linux-gnu` | AppImage (+ deb/rpm if tools present) |
| `x86_64-pc-windows-msvc` | NSIS installer + MSI |

## Code signing

No certificate is configured. Windows SmartScreen will warn on first run. Users can click **More info → Run anyway**. A commercial certificate would remove this warning.

## Troubleshooting

**`error: invalid value 'nsis' for '--bundles'`** — Remove the `--bundles nsis` flag. Let the config drive bundle selection.

**`makensis: command not found`** — Install NSIS: `sudo pacman -S nsis` or `sudo apt install nsis`.

**`No prebuilt ONNX Runtime binary`** — You are using the GNU target instead of MSVC. Use `--target x86_64-pc-windows-msvc`.
