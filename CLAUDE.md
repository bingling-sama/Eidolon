# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
# Build
cargo build

# Release build
cargo build --release

# Render a skin (minimal — all defaults)
cargo run -- render resources/bingling_sama.png

# Render with custom options
cargo run -- render resources/bingling_sama.png out.webp --slim --posture wave --cam-zoom 1.5

# Preview in a window
cargo run -- preview resources/bingling_sama.png --slim

# Convert single-layer → double-layer skin
cargo run -- convert old_skin.png new_skin.png

# Identify / inspect texture format and HD status
cargo run -- identify resources/bingling_sama.png

# Run core tests (use --test-threads=1 to prevent wgpu parallel test timeout)
cargo test -p eidolon-core --lib -- --test-threads=1

# Run CLI tests
cargo test -p eidolon-cli

# Run WASM Playwright E2E tests
pnpm --dir examples/wasm run test:e2e

# Run benchmarks
cargo bench -p eidolon-core
```

## Environment

This project uses **OSMesa** for headless software OpenGL rendering — no GPU or display server needed. The `.env` file sets critical env vars:

```
LIBGL_ALWAYS_SOFTWARE=1
MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
```

**System dependency:** `libosmesa6-dev` (Ubuntu). Requires Mesa < 25.10 (osmesa was removed in later versions). Source these env vars before running if your shell doesn't auto-load `.env`.

## Architecture

This is a **Minecraft skin off-screen renderer** structured as a Cargo + pnpm monorepo — it loads a player skin PNG and 3D OBJ model, renders them via wgpu to an off-screen framebuffer or canvas, and writes static or animated PNG/WebP/GIF/APNG/AVIF output.

### Workspace packages

- **`crates/core`** (`eidolon-core`, lib `eidolon_core`): Pure rendering engine library.
- **`crates/cli`** (`eidolon-cli`, bin `eidolon`): CLI binary supporting `render`, `preview`, `convert`, `identify`.
- **`crates/wasm`** (`eidolon-wasm`, npm `@eidolonmc/wasm`): WebAssembly bindings for WebGPU in the browser.
- **`examples/rust`** (`eidolon-example`): Standalone Rust examples consuming `eidolon-core`.
- **`examples/wasm`**: Vite web studio application and Playwright E2E suite.

### Module layout

```
crates/
├── core/
│   ├── Cargo.toml
│   ├── benches/performance_benchmark.rs
│   └── src/
│       ├── lib.rs            # Public re-exports
│       ├── animation.rs      # Preset sampling, playback timeline, and offline frame schedules
│       ├── assets.rs         # Low-level asset loaders and single2double conversion
│       ├── back_equipment.rs # Shared atlas + Cape/Elytra presentation selection
│       ├── body.rs           # Body: Renderer-bound Skin + classic/slim geometry selection
│       ├── camera.rs         # Camera: yaw/pitch/scale → view + projection matrices
│       ├── character.rs      # Character: Body/Back equipment ownership, posture, position, rotation
│       ├── classifier.rs     # Texture classifier (skin/cape/elytra, HD detection)
│       ├── constants.rs      # WGSL vertex + fragment shaders
│       ├── converter.rs      # single2double() public conversion wrapper
│       ├── error.rs          # EidolonError — typed errors for the public API
│       ├── model.rs          # OBJ loader: parses named objects into BodyParts (main + layer meshes)
│       ├── output.rs         # Output intent, format validation, timing, encoding (PNG/WebP/GIF/APNG/AVIF)
│       ├── texture.rs        # Private PNG upload and single→double conversion
│       └── renderer/
│           ├── mod.rs        # Renderer: headless + windowed, shared pipeline
│           ├── nametag.rs    # Minecraft / Unifont text rasterization and billboard geometry
│           ├── pipeline.rs   # Render pipeline creation from WGSL shader
│           ├── readback.rs   # GPU → CPU buffer copy and RGBA mapping
│           └── uniforms.rs   # Per-body-part uniform computation, PART_CONFIGS with PartId enum
├── cli/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # Minimal 31-line entry point dispatching subcommands
│       ├── args.rs           # Clap CLI arguments (Command::Render/Preview/Convert/Identify)
│       ├── commands.rs       # Execution handlers for render, convert, identify
│       ├── preview.rs        # Winit desktop preview window loop
│       └── scene.rs          # Scene args and camera resolution
└── wasm/
    ├── Cargo.toml
    ├── package.json
    └── src/
        ├── lib.rs            # WASM entry point, logger and panic hook
        ├── classifier.rs     # inspect_texture JS binding returning JsTextureReport
        └── renderer.rs       # CanvasRenderer WebGPU presenter and offscreen capture
```

### Data flow

1. **CLI** (`crates/cli/src/main.rs`, `args.rs`, `commands.rs`) parses subcommand → creates `Renderer` (one-time, expensive — initializes wgpu device + compiles shaders + loads both OBJs)
2. `Renderer::load_body()` loads and uploads a Skin, auto-detects single-layer (width == height×2), converts it via `converter::single2double()`, and returns a Renderer-bound `Body`
3. The caller builds a non-empty `Character` from a Body or Back equipment; `Renderer::render()` draws its optional Body and Back equipment using `character.posture`, then reads pixels back as `ImageBuffer`
4. `output::write_output()` owns output intent, path resolution, animation timing, encoding, and file writing. It asks `Renderer::render()` for RGBA frames and normalizes the final extension while preserving parent directories.

### Key design details

- **Two OBJ models** loaded at startup: `resources/slim.obj` (Alex, 3px arms) and `resources/classic.obj` (Steve, 4px arms). Each named object in the OBJ has a main mesh and a "Layer" mesh (for jacket/hat overlay).
- **Pivot-point articulation**: Each limb rotates around a hardcoded pivot (e.g., right arm pivot at `(0.3125, 1.375, 0.0)`). The transform formula is `base × translate(pivot) × rotate × translate(-pivot)`.
- **Posture angles**: 0° = neutral for all joints (no rotation from bind pose). Positive yaw = turn right, positive pitch = look up / swing forward. `body_pitch` rotates the full model around its center; the Elytra pair shares a central upper-back pivot and mirrors yaw/roll on the right.
- **Single→double layer conversion**: `converter::single2double()` mirrors right-side arm/leg regions horizontally to create left-side overlays in the bottom half of a square texture. Source regions defined as pixel rectangles for 64px reference, scaled by an HD ratio for larger skins.
- **Output formats**: Static PNG/WebP and animated GIF/Animated WebP via `output::OutputIntent`. Explicit format is authoritative; filename extensions are normalized case-insensitively.
- **Resource ownership**: Character owns Body and optional Back equipment. Clones share immutable GPU resources through `Arc`; a different Renderer must reload them.
- **Error handling**: Public API returns `EidolonError` (typed enum: `Io`, `Gpu`, `Model`, `Texture`, `Conversion`, `InvalidPath`, `Output`, `Surface`).
- **Logging**: Uses `env_logger` (`RUST_LOG` env var controls level).

## Agent skills

### Issue tracker

GitHub Issues on `bingling-sama/Eidolon` via `gh` CLI. External PRs are a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Default canonical labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context repo — one `CONTEXT.md` + `docs/adr/` at root. See `docs/agents/domain.md`.
