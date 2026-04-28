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

You must specify `--skin-type` (`classic` or `slim`). The default texture is `resources/bingling_sama.png`.

```bash
cargo run -- render --skin-type classic
```

Output defaults to `output.png`. To render a WebP:

```bash
cargo run -- render --skin-type classic --format webp
```

If you keep `--filename output.png` with `--format webp`, the output name is auto-adjusted to `output.webp`.

Render a custom skin and camera angle:

```bash
cargo run -- render \
  --skin-type slim \
  --texture resources/bingling_sama.png \
  --filename preview.png \
  --width 1024 \
  --height 1024 \
  --yaw 210 \
  --pitch 90 \
  --scale 1.2
```

## Preview in a Window

```bash
cargo run -- preview --skin-type classic
```

The preview uses the same scene, camera, posture, and viewport options as `render`.

## Convert a Legacy Skin

Legacy single-layer skins such as `64x32` can be converted to a square double-layer atlas:

```bash
cargo run -- convert resources/SSSSSteven.png converted.png
```

## Notes

- Single-layer skins (width = 2 × height) are automatically expanded to double-layer on load.
- Use `cargo run -- render --help` and `cargo run -- preview --help` for the full option list.
- See `troubleshooting.md` if the renderer cannot find a GPU adapter or if a skin path fails to load.
