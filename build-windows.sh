#!/usr/bin/env bash
# Cross-compile Windows exe from Linux using cargo-xwin + MSVC target.
# Produces a standalone exe — no installer. The NSIS installer must be
# built on a Windows machine: npm run tauri build -- --bundles nsis
#
# Requires: cargo-xwin (cargo install cargo-xwin), x86_64-pc-windows-msvc rust target
set -euo pipefail

echo "Building Windows exe (no installer)..."
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle

echo ""
echo "Done:"
echo "  src-tauri/target/x86_64-pc-windows-msvc/release/joplin-smart-search.exe"
