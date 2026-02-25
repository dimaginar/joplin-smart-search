# Security Audit Notes

Last checked: 2026-02-25

## How to run

```bash
# Rust dependencies
cargo install cargo-audit
cargo audit --file src-tauri/Cargo.lock

# npm dependencies
npm audit
```

## Current status

### Rust — 0 vulnerabilities, 25 warnings

All warnings are `unmaintained` crates — no CVEs, nothing exploitable. They are indirect dependencies pulled in by Tauri/Wry and cannot be resolved until Tauri upstream updates them.

| Crate | Issue |
|---|---|
| `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gtk`, etc. | gtk-rs GTK3 bindings — no longer maintained (RUSTSEC-2024-0413/0416) |
| `bincode` 1.x and 2.x | Unmaintained |
| `fxhash` | Unmaintained |

**Action:** none needed. Monitor Tauri releases; these will be resolved when Tauri moves to GTK4 bindings.

### npm — 2 moderate warnings (dev-only, not a production risk)

| Package | Advisory | Severity |
|---|---|---|
| `esbuild ≤0.24.2` | GHSA-67mh-4wv8-2f99 | Moderate |
| `vite 0.11.0–6.1.6` | Depends on vulnerable esbuild | Moderate |

**What it means:** a website could send requests to the Vite dev server and read responses. This only applies when running `npm run dev`. The built AppImage/binary contains no vite or esbuild code.

**Fix when ready:**
```bash
npm install vite@latest
```
This upgrades to Vite 7 which includes the fix. May require minor config updates.

**Action:** low priority — only affects local dev environment, not end users.
