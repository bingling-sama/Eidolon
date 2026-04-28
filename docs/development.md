# Development

This guide covers the local commands used while changing Eidolon.

## Setup

Install the latest stable Rust toolchain, then build the workspace:

```bash
cargo build
```

The renderer loads models and sample textures from `resources/`, so run commands from the repository
root unless you pass absolute paths.

## Useful Commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test
cargo run -- render --skin-type classic
cargo run -- preview --skin-type classic
cargo bench
```

`cargo bench` writes rendered benchmark output under `.bench/`.

## Testing Notes

The current test coverage focuses on the skin atlas converter in `src/utils/converter.rs`. Rendering
paths initialize a real `wgpu` adapter, so they depend on the machine's graphics stack and may be
better validated with smoke tests:

```bash
cargo run -- render --skin-type classic --filename output.png
cargo run -- render --skin-type slim --texture resources/bingling_sama.png --format webp
```

When changing rendering behavior, compare output images before and after the change. The sample
assets in `resources/` cover both classic and slim geometry plus single-layer conversion.

## Adding Assets

For skins:

- Use PNG files.
- Use square double-layer skins such as `64x64`, or legacy single-layer skins where
  `width == height * 2`, such as `64x32`.
- Pass the matching `--skin-type` for the skin's arm geometry.

For OBJ models:

- Preserve the required object names documented in `architecture.md`.
- Keep UVs aligned to Minecraft skin atlas coordinates.
- Confirm both inner and layer meshes render correctly.
