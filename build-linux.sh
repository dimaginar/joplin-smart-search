#!/usr/bin/env bash
# Local Linux build helper.
# Passes APPIMAGE_EXTRACT_AND_RUN=1 so appimagetool works without FUSE2.
# Usage: ./build-linux.sh
set -euo pipefail

export APPIMAGE_EXTRACT_AND_RUN=1

echo "Building frontend..."
npm run build

echo "Building Tauri app (AppImage)..."
npm run tauri -- build --target x86_64-unknown-linux-gnu --bundles appimage

echo ""
echo "Output:"
find src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/appimage -name "*.AppImage" 2>/dev/null || true
