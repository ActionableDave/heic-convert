# HEIC Convert

A tiny cross-platform desktop app that converts HEIC/HEIF photos (the iPhone default)
to JPEG or PNG. Built with [Tauri](https://tauri.app) — small binary, small RAM footprint.

Instead of bundling a decoder library, it uses each OS's native image codecs:

| OS | Decoder | Notes |
|---|---|---|
| Windows | WIC (Windows Imaging Component) | Needs the "HEIF Image Extensions" + "HEVC Video Extensions" Store codecs — preinstalled on Windows 11 |
| macOS | `sips` (built into macOS) | No dependencies at all |

Features:

- Drag & drop (or file picker), batch conversion
- JPEG (with quality slider) or PNG output
- Applies EXIF orientation so photos don't come out sideways
- Preserves file modified times so date-sorted folders stay in order
- Output to the same folder or a chosen folder; never overwrites (appends `(1)`, `(2)`, …)
- Optional Explorer right-click menu (Windows): "Convert to JPEG" / "Convert to PNG" on
  `.heic`/`.heif` files — enable it from inside the app. Per-user registry entries, no admin
  needed. On Windows 11 the entries live under "Show more options".

## Command line

```
heic-convert.exe --quick <jpeg|png> <file> [more files...]   # silent conversion (quality 85)
heic-convert.exe --register-context-menu                     # add Explorer right-click entries
heic-convert.exe --unregister-context-menu                   # remove them
```

## Development

Prerequisites: [Node.js](https://nodejs.org), [Rust](https://rustup.rs), and on Windows the
MSVC C++ build tools (matching your CPU architecture — ARM64 tools on ARM64 machines).

```bash
npm install
npm run dev      # run the app with hot reload
npm run build    # build installers into src-tauri/target/release/bundle/
```

## Releases

Pushing a tag like `v0.1.0` (or manually running the Build workflow) builds
macOS (Apple Silicon), Windows ARM64, and Windows x64 installers via GitHub Actions
and attaches them to a draft release.
