# Getting Started

This guide helps you build Eidolon, render your first image, and preview a skin in a window.

## Prerequisites

- Rust (latest stable recommended)
- A GPU with Vulkan, Metal, or DX12 support
- A working `wgpu` backend. Headless rendering still needs an adapter; on some platforms that can
  be a configured software backend such as OSMesa.

## Build

```bash
cargo build
```

## Render an Image (Headless)

The minimal invocation uses all defaults (classic arms, stand pose, 800×600 PNG):

```bash
cargo run -- render resources/bingling_sama.png
```

Output defaults to `output.png`. To render a WebP:

```bash
cargo run -- render resources/bingling_sama.png out.webp
```

The format is inferred from the output filename extension — `.png` or `.webp`.

### Slim Arms

Use `--slim` for Alex-style (3px) arm geometry. Default is classic (Steve, 4px):

```bash
cargo run -- render resources/bingling_sama.png --slim
```

### Posture Presets

Choose from `stand` (default), `wave`, `walking`, `running`:

```bash
cargo run -- render resources/bingling_sama.png --posture wave
cargo run -- render resources/bingling_sama.png --posture running --slim
```

### Camera Control

Adjust the camera angle with `--cam-yaw`, `--cam-pitch`, and `--cam-zoom`:

```bash
cargo run -- render resources/bingling_sama.png \
  --cam-yaw 210 \
  --cam-pitch 80 \
  --cam-zoom 1.5 \
  --width 1024 \
  --height 1024
```

`--cam-zoom` must be > 0. Larger values move the camera closer (orbit radius: 4.0 / zoom).

## Preview in a Window

```bash
cargo run -- preview resources/bingling_sama.png
cargo run -- preview resources/bingling_sama.png --slim --cam-zoom 2.0
```

The preview uses the same scene, camera, posture, and viewport options as `render`.

## Convert a Legacy Skin

Legacy single-layer skins such as 64×32 can be converted to a square double-layer atlas:

```bash
cargo run -- convert resources/SSSSSteven.png converted.png
```

## Notes

- Single-layer skins (width = 2 × height) are automatically expanded to double-layer on load.
- Use `-h` for a concise option list or `--help` for the full list including advanced overrides.
- See `troubleshooting.md` if the renderer cannot find a GPU adapter or if a skin path fails to load.
