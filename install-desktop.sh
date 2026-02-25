#!/usr/bin/env bash
# Installs a .desktop entry so KDE Plasma shows the correct taskbar icon.
# Run once after building. Re-run if you move the binary.

BINARY="$(cd "$(dirname "$0")" && pwd)/src-tauri/target/release/joplin-smart-search"
ICON="$(cd "$(dirname "$0")" && pwd)/src-tauri/icons/128x128.png"
DESKTOP_DIR="$HOME/.local/share/applications"
DESKTOP_FILE="$DESKTOP_DIR/joplin-smart-search.desktop"

mkdir -p "$DESKTOP_DIR"

cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Name=Joplin Smart Search
Exec=$BINARY
Icon=$ICON
Type=Application
Categories=Utility;
StartupWMClass=joplin-smart-search
EOF

# Refresh the desktop database so KDE picks it up immediately.
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
kbuildsycoca6 --noincremental 2>/dev/null || kbuildsycoca5 --noincremental 2>/dev/null || true

echo "Installed: $DESKTOP_FILE"
echo "Launch the app from the .desktop entry (or via krunner/app menu) for the icon to stick."
echo "If the icon still shows wrong in the taskbar, right-click the taskbar entry while the app"
echo "is running and choose 'Pin to Task Manager' — KDE will then associate the .desktop entry."
