#!/usr/bin/env bash
# Cross-compile Windows exe from Linux using cargo-xwin + MSVC target.
# Requires: cargo-xwin (cargo install cargo-xwin), x86_64-pc-windows-msvc rust target
#
# Usage:
#   ./build-windows.sh          — raw .exe only (no installer)
#   ./build-windows.sh --nsis   — .exe + NSIS installer (requires nsis package)
set -euo pipefail

if [[ "${1:-}" == "--nsis" ]]; then
    echo "Building Windows exe + NSIS installer..."
    npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
    echo ""
    echo "Installer:"
    find src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis -name "*.exe" 2>/dev/null || true
else
    echo "Building Windows exe (no installer)..."
    npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle
fi

echo ""
echo "Exe:"
echo "  src-tauri/target/x86_64-pc-windows-msvc/release/joplin-smart-search.exe"
