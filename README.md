# mal-char-crop

A lightweight desktop tool for quickly preparing character images for MyAnimeList.
Crop, rotate, position, and save images in the required 9:14 aspect ratio.

## Download

Prebuilt binaries are published on the [Releases](https://github.com/yevgenii-v/mal_char_crop/releases) page.

| Platform | Archive |
|---|---|
| Linux x86_64 | `mal-char-crop-vX.Y.Z-linux-x86_64.tar.gz` |
| Windows x86_64 | `mal-char-crop-vX.Y.Z-windows-x86_64.zip` |
| macOS (Intel + Apple Silicon, universal) | `mal-char-crop-vX.Y.Z-macos-universal.tar.gz` |

Each release also has `SHA256SUMS.txt` for verifying the downloads.

**ARM builds for Linux and Windows are not published yet.** If you need one,
please [open an issue](https://github.com/yevgenii-v/mal_char_crop/issues) — or build from source (see below).

### macOS

The binary is not signed, so Gatekeeper blocks it on first launch. Remove the quarantine flag:

```sh
xattr -d com.apple.quarantine mal-char-crop
```

### Linux

The app runs through X11/XWayland by default, because file drag-and-drop does not work on native Wayland.
Set `MAL_CROP_WAYLAND=1` to use native Wayland anyway.

## Usage

| Shortcut | Action |
|---|---|
| Ctrl+O | Open |
| Ctrl+V | Paste image or copied files |
| Ctrl+S / Ctrl+Shift+S | Save / Save as |
| Ctrl+W | Close tab |
| Ctrl+Tab, Ctrl+PageDown / Ctrl+Shift+Tab, Ctrl+PageUp | Next / previous tab |
| Arrows (Shift: ×10) | Move the frame |
| `[` / `]` (Shift: fine step) | Rotate |
| F | Fit image to view |

The UI is available in English, Bahasa Indonesia, Português (Brasil), Deutsch, Español, Filipino,
Bahasa Melayu and Українська. Force a language with `MAL_CROP_LANG`, e.g. `MAL_CROP_LANG=uk`.

## Building from source

Requires a stable Rust toolchain.

```sh
cargo build --release
./target/release/mal-char-crop
```

## Releases

Every push to `main` builds and tests all platforms. A new GitHub Release is created
automatically when the `version` in `Cargo.toml` has no matching `vX.Y.Z` tag yet —
bump the version to publish a release.

## License

[MIT](LICENSE)
