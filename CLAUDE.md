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

# Run tests
cargo test

# Run benchmarks (renders 20 images with varying camera angles → .bench/)
cargo bench
```

## Environment

This project uses **OSMesa** for headless software OpenGL rendering — no GPU or display server needed. The `.env` file sets critical env vars:

```
LIBGL_ALWAYS_SOFTWARE=1
MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
```

**System dependency:** `libosmesa6-dev` (Ubuntu). Requires Mesa < 25.10 (osmesa was removed in later versions). Source these env vars before running if your shell doesn't auto-load `.env`.

## Architecture

This is a **Minecraft skin off-screen renderer** — it loads a player skin PNG and 3D OBJ model, renders them via wgpu to an off-screen framebuffer, and writes static or animated PNG/WebP/GIF output.

### Crate identity

- Crate name: `eidolonmc` (Cargo.toml `[package]`)
- Library name: `eidolon` (Cargo.toml `[lib]`)
- Bin name: `eidolon` (Cargo.toml `[[bin]]`)

### Module layout

```
src/
├── lib.rs          # Re-exports all public modules
├── main.rs         # CLI binary — clap with `render`, `preview`, and `convert` subcommands
├── error.rs        # EidolonError — typed errors for the public API
├── constants.rs    # WGSL vertex + fragment shaders
├── camera.rs       # Camera: yaw/pitch/scale → view + projection matrices
├── model.rs        # OBJ loader: parses named objects into BodyParts (main + layer meshes)
├── body.rs         # Body: Renderer-bound Skin + classic/slim geometry selection
├── back_equipment.rs # Shared atlas + Cape/Elytra presentation selection
├── texture.rs      # Private PNG upload and single→double conversion
├── converter.rs    # single2double(): mirrors right-leg/arm regions to left side
├── character.rs    # Character: Body/Back equipment ownership, posture, position, rotation
└── renderer/
    ├── mod.rs      # Renderer: headless + windowed, shared pipeline
    ├── pipeline.rs # Render pipeline creation from WGSL shader
    ├── readback.rs # GPU → CPU buffer copy and RGBA mapping
    └── uniforms.rs # Per-body-part uniform computation, PART_CONFIGS with PartId enum
```

### Data flow

1. **CLI** (`main.rs`) parses subcommand → creates `Renderer` (one-time, expensive — initializes wgpu device + compiles shaders + loads both OBJs)
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
