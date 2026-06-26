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
cargo run -- render resources/bingling_sama.png
cargo run -- preview resources/bingling_sama.png
cargo bench
```

`cargo bench` writes rendered benchmark output under `.bench/`.

## Testing Notes

Unit tests cover:
- Camera matrix computation (`src/camera.rs`) — 4 tests
- Skin converter (`src/converter.rs`) — 5 tests

Model loading tests are `#[ignore]` — they require a GPU adapter. Rendering integration tests
are in `tests/` and are also `#[ignore]`. Smoke-test rendering manually:

```bash
cargo run -- render resources/bingling_sama.png
cargo run -- render resources/bingling_sama.png out.webp --slim --posture wave
```

When changing rendering behavior, compare output images before and after the change. The sample
assets in `resources/` cover both classic and slim geometry plus single-layer conversion.

## Adding Assets

For skins:

- Use PNG files.
- Use square double-layer skins such as `64x64`, or legacy single-layer skins where
  `width == height * 2`, such as `64x32`.
- Use `--slim` for 3px arm skins; omit it for classic 4px arm skins.

For OBJ models:

- Preserve the required object names documented in `architecture.md`.
- Keep UVs aligned to Minecraft skin atlas coordinates.
- Confirm both inner and layer meshes render correctly.
