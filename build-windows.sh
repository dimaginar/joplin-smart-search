#!/usr/bin/env bash
# Cross-compile Windows NSIS installer from Linux using cargo-xwin + MSVC target.
#
# Requires:
#   cargo install cargo-xwin
#   rustup target add x86_64-pc-windows-msvc
#   sudo pacman -S nsis lld llvm   (or apt install nsis lld llvm on Ubuntu)
#
# Note: do NOT pass --bundles nsis — Tauri CLI rejects it on Linux.
# With bundle.targets = "all" in tauri.conf.json, Tauri auto-selects NSIS
# when the target triple is x86_64-pc-windows-msvc.
set -euo pipefail

BUNDLE_DIR="src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis"

npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc

# Tauri uses productName verbatim in the filename (spaces included).
# Rename to a clean lowercase filename: "Joplin Smart Search_x.x.x_x64-setup.exe"
#                                      → "joplin-smart-search_x.x.x_x64-setup.exe"
src=$(ls "$BUNDLE_DIR"/*.exe 2>/dev/null | head -1)
if [[ -n "$src" ]]; then
  base=$(basename "$src")
  dst="$BUNDLE_DIR/$(echo "$base" | tr '[:upper:]' '[:lower:]' | tr ' ' '-')"
  [[ "$src" != "$dst" ]] && mv "$src" "$dst"
  src="$dst"
fi

echo ""
echo "Done:"
echo "  $src"
echo "  src-tauri/target/x86_64-pc-windows-msvc/release/joplin-smart-search.exe"
