# Joplin Smart Search

Semantic search for your [Joplin](https://joplinapp.org/) notes. Type a concept or idea — not just a keyword — and find the notes that actually match.

Built with Tauri 2 + Rust. Fully local and offline after first run. No cloud, no GPU required.

## 🔍 How It Works

- Notes are embedded using [bge-small-en-v1.5](https://huggingface.co/BAAI/bge-small-en-v1.5) — a small, fast ONNX model (~33 MB, downloaded once on first run)
- Embeddings are stored in a local HNSW vector index on your machine
- New and edited notes are picked up automatically within ~15 seconds
- Click a result to open the note directly in Joplin

## 🔒 Transparency & Safety

This project was developed with the assistance of AI coding tools. The full source code is public for community audit.

- **Read-only**: The app reads Joplin's local SQLite database but never modifies it
- **Fully local**: No data leaves your machine — no cloud, no telemetry
- **Verified code**: Feel free to audit the Rust backend (`src-tauri/src/`) and React frontend (`src/`)

## 🚀 Quick Start

1. Make sure [Joplin desktop](https://joplinapp.org/help/install) is installed and has synced at least once
2. Download the file for your platform from the [Releases](../../releases) page:

| Platform | File |
|---|---|
| Linux (Ubuntu, Fedora, Mint, …) | `joplin-smart-search_x.x.x_amd64.AppImage` |
| Linux (Arch, CachyOS, Manjaro, …) | `joplin-smart-search_x86_64-linux.tar.gz` |
| Windows | `Joplin Smart Search_x.x.x_x64-setup.exe` |

3. Run the app — see platform instructions below
4. On first launch the embedding model downloads (~33 MB). After that the app works fully offline.

## 📦 Install & Run

### Linux — AppImage

```bash
chmod +x "joplin-smart-search_x.x.x_amd64.AppImage"
./"joplin-smart-search_x.x.x_amd64.AppImage"
```

Double-clicking in your file manager also works on most distros.

### Linux — tar.gz (Arch, CachyOS, Manjaro)

The AppImage bundles an Ubuntu-built WebKit that is incompatible with Arch's Mesa/EGL layout and crashes on launch. The tar.gz uses your system's `webkit2gtk-4.1` instead.

**Requirement:** install `webkit2gtk-4.1` if not already present:

```bash
sudo pacman -S webkit2gtk-4.1
```

**Extract and run:**

```bash
tar -xzf joplin-smart-search_x86_64-linux.tar.gz
chmod +x joplin-smart-search
./joplin-smart-search
```

### Windows

Run the `.exe` installer. No admin rights required — installs to your user profile.

**Windows SmartScreen warning?** Click **More info** → **Run anyway**. The warning appears because the app is not signed with a commercial code signing certificate.

## 🛠️ Troubleshooting

### AppImage: "fuse: failed to open /dev/fuse" (Ubuntu 24.04)

Ubuntu 24.04 ships with FUSE3 only. Install FUSE2:

```bash
sudo apt-get install libfuse2
```

Or run without FUSE:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./"joplin-smart-search_x.x.x_amd64.AppImage"
```

### AppImage: "Could not create default EGL display" (Arch / CachyOS)

The AppImage bundles an Ubuntu-built WebKit which is incompatible with Arch's Mesa layout. Use the `tar.gz` download instead — see [Linux — tar.gz](#linux--targz-arch-cachyos-manjaro) above.

### App starts but Joplin database is not found

The app looks for Joplin's SQLite database in the default location. If you've installed Joplin in a non-standard location or use a portable install, click the folder icon in the app to locate the database manually.

The database is typically at:
- **Linux:** `~/.config/joplin-desktop/database.sqlite`
- **Windows:** `%APPDATA%\joplin-desktop\database.sqlite`

### First run: model download fails

The embedding model (~33 MB) is downloaded from HuggingFace on first launch. Make sure you have an internet connection for this one-time step. After that the app works fully offline.

### Taskbar icon shows generic icon (Linux, tar.gz)

Without a `.desktop` entry the taskbar may show a generic icon. To fix it, create one pointing to wherever you placed the binary:

```bash
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/joplin-smart-search.desktop << 'EOF'
[Desktop Entry]
Name=Joplin Smart Search
Exec=/home/YOUR_USER/path/to/joplin-smart-search
Icon=applications-other
Type=Application
Categories=Utility;
StartupWMClass=joplin-smart-search
EOF
update-desktop-database ~/.local/share/applications
```

Replace the `Exec=` path with the actual location of the binary.

## ☕ Support Development

If Joplin Smart Search is useful to you, consider supporting its development. Donations help fund a code signing certificate to remove the Windows SmartScreen warning and make the app more trusted for everyone.

[Donate with PayPal](https://www.paypal.com/donate/?business=Q4JJUB58QT7SN) · [Donate with iDEAL](https://betaalverzoek.rabobank.nl/betaalverzoek/?id=MiDjVyNBSN-Qy288Zb0sJg)

## 🛠️ Tech Stack

- [Tauri 2](https://v2.tauri.app/) + Rust — backend, file watching, SQLite access
- React 19 + TypeScript — frontend
- Zustand — state management
- Tailwind CSS 4 — styling
- [fastembed](https://github.com/Anush008/fastembed-rs) — ONNX embedding inference, no Python, no GPU
- [bge-small-en-v1.5](https://huggingface.co/BAAI/bge-small-en-v1.5) — embedding model
- HNSW vector index — fast approximate nearest-neighbour search
