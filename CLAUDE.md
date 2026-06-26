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

This is a **Minecraft skin off-screen renderer** — it loads a player skin PNG and 3D OBJ model, renders them via wgpu to an off-screen framebuffer, and saves the result as PNG/WebP.

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
├── texture.rs      # Skin texture: loads PNG, auto-converts single→double layer
├── converter.rs    # single2double(): mirrors right-leg/arm regions to left side
├── character.rs    # Character: skin_type, posture (8 joint angles, 0° = neutral), position, rotation
└── renderer/
    ├── mod.rs      # Renderer: headless + windowed, shared pipeline
    ├── pipeline.rs # Render pipeline creation from WGSL shader
    ├── readback.rs # GPU → CPU buffer copy and RGBA mapping
    └── uniforms.rs # Per-body-part uniform computation, PART_CONFIGS with PartId enum
```

### Data flow

1. **CLI** (`main.rs`) parses subcommand → creates `Renderer` (one-time, expensive — initializes wgpu device + compiles shaders + loads both OBJs)
2. `Renderer::load_texture()` → `Texture::load_from_file()` loads PNG, auto-detects single-layer (width == height×2) and converts to double-layer via `converter::single2double()`
3. `Renderer::render()` sets up off-screen framebuffer, selects slim/classic model, iterates body parts (head, body, arms, legs) applying pivot-point rotations from `character.posture`, draws with shaders, reads pixels back as `ImageBuffer`
4. `Renderer::render_to_image()` wraps `render()` and saves with `image` crate (PNG or WebP). Output filename extension is auto-adjusted to match the requested format.

### Key design details

- **Two OBJ models** loaded at startup: `resources/slim.obj` (Alex, 3px arms) and `resources/classic.obj` (Steve, 4px arms). Each named object in the OBJ has a main mesh and a "Layer" mesh (for jacket/hat overlay).
- **Pivot-point articulation**: Each limb rotates around a hardcoded pivot (e.g., right arm pivot at `(0.3125, 1.375, 0.0)`). The transform formula is `base × translate(pivot) × rotate × translate(-pivot)`.
- **Posture angles**: 0° = neutral for all joints (no rotation from bind pose). Positive yaw = turn right, positive pitch = look up / swing forward.
- **Single→double layer conversion**: `converter::single2double()` mirrors right-side arm/leg regions horizontally to create left-side overlays in the bottom half of a square texture. Source regions defined as pixel rectangles for 64px reference, scaled by an HD ratio for larger skins.
- **Output formats**: PNG and WebP via `OutputFormat` enum. Format is inferred from the output filename extension.
- **Error handling**: Public API returns `EidolonError` (typed enum: `Io`, `Gpu`, `Model`, `Texture`, `Conversion`, `InvalidPath`). Skin texture is passed as `&Texture` to render methods — the compiler guarantees it's loaded before rendering.
- **Logging**: Uses `env_logger` (`RUST_LOG` env var controls level).

## Agent skills

### Issue tracker

GitHub Issues on `bingling-sama/Eidolon` via `gh` CLI. External PRs are a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Default canonical labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context repo — one `CONTEXT.md` + `docs/adr/` at root. See `docs/agents/domain.md`.
