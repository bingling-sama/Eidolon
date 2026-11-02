# eidolon-cli

[![Crates.io](https://img.shields.io/crates/v/eidolon-cli.svg)](https://crates.io/crates/eidolon-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Command-line interface for **Eidolon**, a Minecraft skin 3D renderer written in Rust.

`eidolon-cli` provides the executable `eidolon` to render skins off-screen, preview skins interactively in a window, convert legacy skins, and inspect texture files.

## Installation

### Via Cargo

```bash
cargo install eidolon-cli
```

### Pre-built Binaries

Pre-compiled binary releases for Windows, macOS (Intel & Apple Silicon), and Linux (x86_64 & aarch64) are available on [GitHub Releases](https://github.com/bingling-sama/eidolon/releases).

## Subcommands

- `render`: Render a skin to a static image (PNG/WebP) or animation file (GIF/WebP/APNG/AVIF) headless.
- `preview`: Open an interactive window with live orbit/pan/zoom camera controls and animation playback.
- `convert`: Losslessly convert legacy single-layer (64×32) skins to modern double-layer (64×64) format.
- `identify`: Inspect texture files and classify them as Classic/Slim skins, capes, elytra, or legacy single-layer.

## Usage & Examples

### 1. Headless Rendering (`render`)

```bash
# Minimal: default classic arms, idle pose, 800×600 PNG
eidolon render skin.png

# Slim arms (Alex 3px arms), wave posture, saved to WebP
eidolon render skin.png out.webp --slim --posture wave

# Custom camera orbit, zoom, and dimensions
eidolon render skin.png --cam-yaw 210 --cam-pitch 85 --cam-zoom 1.5 --width 1024 --height 1024

# Render with a player nametag
eidolon render skin.png --nametag "Steve"

# Render with back equipment (Cape or Elytra)
eidolon render skin.png --cape cape.png --cape-pitch 25
eidolon render skin.png --elytra elytra.png --elytra-pitch 15 --elytra-spread 45

# Render animated walk cycle to GIF or animated WebP
eidolon render skin.png walk.gif --animate --posture walk --fps 30
eidolon render skin.png run.webp --animate --posture run --speed 1.2
```

### 2. Interactive Windowed Preview (`preview`)

```bash
# Preview skin interactively
eidolon preview skin.png

# Preview with slim geometry, cape, and nametag
eidolon preview skin.png --slim --cape cape.png --nametag "Alex"
```

**Controls in Preview Mode:**
- **Right-click + Drag**: Orbit camera (yaw / pitch)
- **Left-click + Drag**: Pan camera
- **Scroll Wheel**: Zoom in / out
- **Space**: Pause / Resume animation playback
- **Arrow Keys**: Scrub animation timeline

### 3. Skin Conversion (`convert`)

```bash
# Convert a legacy 64x32 skin to modern 64x64 format
eidolon convert legacy_skin.png modern_skin.png
```

### 4. Texture Inspection (`identify`)

```bash
# Detect texture type, resolution, arm geometry recommendation, and single/double layer
eidolon identify skin.png

# Output machine-readable JSON
eidolon identify skin.png --json
```

## Options Reference

Run `eidolon <SUBCOMMAND> --help` for the complete list of flags and options:

```bash
eidolon --help
eidolon render --help
eidolon preview --help
eidolon convert --help
eidolon identify --help
```

Detailed command-line documentation is available in the [CLI Reference Guide](https://github.com/bingling-sama/eidolon/blob/main/docs/cli.md).

## Related Crates

- [`eidolon-core`](https://crates.io/crates/eidolon-core): Core rendering engine library.
- [`@eidolonmc/wasm`](https://www.npmjs.com/package/@eidolonmc/wasm): WebAssembly bindings for browser applications.

## License

Licensed under the [MIT License](https://github.com/bingling-sama/eidolon/blob/main/LICENSE).
