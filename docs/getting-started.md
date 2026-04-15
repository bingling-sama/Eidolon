# Getting Started

This guide helps you build Eidolon, render your first image, and preview a skin in a window.

## Prerequisites

- Rust (latest stable recommended)
- A GPU with Vulkan, Metal, or DX12 support
- Or an available version of libOSMesa installed for headless environments without a GPU

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

## Preview in a Window

```bash
cargo run -- preview --skin-type classic
```

## Notes

- Single-layer skins (width = 2 × height) are automatically expanded to double-layer on load.
- Use `cargo run -- render --help` and `cargo run -- preview --help` for the full option list.
