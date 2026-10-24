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
cargo clippy --workspace --all-targets --all-features
cargo test -p eidolon-core --lib -- --test-threads=1
cargo test -p eidolon-cli
cargo run -- render resources/bingling_sama.png
cargo run -- preview resources/bingling_sama.png
cargo bench -p eidolon-core
pnpm --dir examples/wasm run test:e2e
```

`cargo bench` writes rendered benchmark output under `.bench/`.

## Testing Notes

- **Core Engine Unit Tests** (`crates/core`): 107 unit tests covering camera matrix computation, skin conversion, texture inspection/classification, animation presets and timelines, nametag rasterization, model loading, and output encoding. Run with `cargo test -p eidolon-core --lib -- --test-threads=1`.
- **CLI Tests** (`crates/cli`): 55 integration tests verifying argument parsing, default options, and subcommand flags. Run with `cargo test -p eidolon-cli`.
- **WASM Browser E2E Tests** (`examples/wasm`): 13 Playwright tests verifying WebGPU canvas rendering, playback, responsive controls, and format export in a browser environment. Run with `pnpm --dir examples/wasm run test:e2e`.

Smoke-test rendering manually:

```bash
cargo run -- render resources/bingling_sama.png
cargo run -- render resources/bingling_sama.png out.webp --slim --posture wave
cargo run -- identify resources/bingling_sama.png
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
