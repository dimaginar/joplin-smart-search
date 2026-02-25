# Joplin Smart Search

Semantic search for your [Joplin](https://joplinapp.org/) notes. Type a concept or idea — not just a keyword — and find the notes that match.

Built with Tauri 2 + Rust. Fully local and offline after first run. No cloud, no GPU required.

## Download

Go to the [Releases](../../releases) page and download the file for your platform:

| Platform | File |
|---|---|
| Linux (Ubuntu, Fedora, Mint, …) | `Joplin Smart Search_x.x.x_amd64.AppImage` |
| Linux (Arch, CachyOS, Manjaro, …) | `joplin-smart-search_x86_64-linux.tar.gz` |
| Windows | `Joplin Smart Search_x.x.x_x64-setup.exe` |

## Install & Run

### Linux — AppImage (Ubuntu, Fedora, Mint, and most distros)

```bash
chmod +x "Joplin Smart Search_x.x.x_amd64.AppImage"
./"Joplin Smart Search_x.x.x_amd64.AppImage"
```

Double-clicking in your file manager also works on most distros.

---

### Linux — tar.gz (Arch, CachyOS, Manjaro, and other Arch-based distros)

The AppImage bundles an Ubuntu-built WebKit that is incompatible with Arch's Mesa/EGL layout and crashes on launch. Use the raw binary instead — it links against your system's `webkit2gtk-4.1`.

**Requirement:** `webkit2gtk-4.1` must be installed.

```bash
# Arch / CachyOS / Manjaro
sudo pacman -S webkit2gtk-4.1
```

**Extract and run:**

```bash
tar -xzf joplin-smart-search_x86_64-linux.tar.gz
chmod +x joplin-smart-search
./joplin-smart-search
```

**Optional — correct taskbar icon (KDE Plasma):**

Without a `.desktop` entry, the taskbar may show a generic icon. To fix it, create one pointing to wherever you placed the binary:

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

Replace `/home/YOUR_USER/path/to/joplin-smart-search` with the actual path to the binary.

---

### Windows

Run the `.exe` installer. No admin rights required — installs to your user profile.

## First Run

On first launch the app downloads the embedding model (~33 MB from HuggingFace). This happens once and is cached locally. After that the app works fully offline.

The app auto-detects your Joplin database. If it isn't found, you'll be prompted to locate it manually.

## How It Works

- Notes are embedded using [bge-small-en-v1.5](https://huggingface.co/BAAI/bge-small-en-v1.5) — a small, fast ONNX model
- Embeddings are stored in a local HNSW vector index
- New and edited notes sync automatically within ~15 seconds
- Click a result to open the note directly in Joplin

## Requirements

- [Joplin desktop](https://joplinapp.org/help/install) must be installed and have synced at least once (the app reads Joplin's local SQLite database — read-only, never modifies it)
