# Architecture

Eidolon is split into a small CLI binary and a reusable rendering library. The binary parses scene
options and delegates to the library; the library owns GPU setup, skin loading, model loading, and
image output.

## Crate Layout

- `src/main.rs` defines the `eidolon` binary and the `render`, `animate`, `preview`, and `convert` subcommands.
- `src/animation.rs` provides stateless procedural `AnimationPreset` sampling into complete Postures.
- `src/animation_export.rs` is the CLI adapter for streaming GIF and Animated WebP encoding.
- `src/lib.rs` exposes the library modules.
- `src/error.rs` defines `EidolonError` — the typed error enum returned by all public APIs.
- `src/body.rs` defines `Body`, which owns a Skin and its classic/slim geometry selection.
- `src/back_equipment.rs` defines the shared Back equipment atlas and Cape/Elytra presentation.
- `src/camera.rs` computes view and projection matrices for an orbit camera.
- Private `src/cape.rs` and `src/elytra.rs` build Renderer-owned geometry using skinview3d atlas mappings.
- `src/character.rs` defines resource ownership, posture presets (0° = neutral for all joints),
  and character transforms. A Character always contains a Body or Back equipment.
- `src/model.rs` loads the classic and slim OBJ meshes from `resources/`. Supports loading from
  file path or in-memory bytes.
- Private `src/texture.rs` uploads PNG atlases and expands legacy single-layer Skins when required.
- `src/converter.rs` implements single-layer to double-layer skin atlas conversion.
- `src/renderer/` contains the shared `wgpu` renderer split into submodules:
  - `mod.rs` — `Renderer` struct: headless + windowed creation, render pass encoding, public API.
  - `nametag.rs` — Monocraft text rasterization, cached label geometry, and billboard transforms.
  - `pipeline.rs` — wgpu render pipeline creation from the embedded WGSL shader.
  - `readback.rs` — GPU → CPU buffer copy with row-padding, mapped readback to `ImageBuffer`.
  - `uniforms.rs` — per-body-part uniform data, `PART_CONFIGS` with `PartId` enum for draw-order safety.
- `benches/performance_benchmark.rs` renders a fixed batch of images for Criterion benchmarks.

## Render Flow

For `cargo run -- render skin.png`, the flow is:

1. `src/main.rs` parses CLI options into `SceneArgs`.
2. `scene_state_and_camera_from_scene` resolves resource-independent scene state and a `Camera`.
3. `Renderer::new` creates a headless `wgpu` device, queue, render pipeline, and loads both OBJ
   model variants.
4. `Renderer::load_body` reads the PNG Skin, converts a legacy atlas when needed, uploads it, and
   returns a Renderer-context-bound `Body`. The CLI then constructs a `Character` and optionally
   equips a shared Back equipment atlas as Cape or Elytra.
5. `Renderer::render_to_image` renders to an off-screen `Rgba8Unorm` texture, copies the padded
   GPU rows into a CPU buffer, strips padding, and saves the image as PNG or WebP. The output
   filename extension is auto-adjusted to match the format.

When `Character::nametag` is set, the Renderer rasterizes a single-line label texture and stores
the generated GPU resource in that Character. Character clones share the cached resource via `Arc`.
The anchor transforms the player collision box, computes its scene-space AABB, and places the label
at the AABB's horizontal center just above its highest point. An unlit billboard pipeline draws it
after the player mesh. The default path draws a 25%-opacity depth-independent pass followed by the
normal depth-tested pass; the collision box is 1.8 world units tall and the default label gap is
0.5 units (both scale with `Camera::scale`); `nametag_see_through = false` skips the first pass.

For `preview`, the same scene data and pipeline are used, but `Renderer::new_windowed` creates a
window surface and `render_frame` presents each frame to the swapchain.

For `animate`, the CLI samples one natural `AnimationPreset` cycle uniformly, applies explicit
Posture overrides, renders each frame, and immediately submits its RGBA buffer to the selected
encoder. GIF delays and WebP timestamps use cumulative rounding so fractional frame durations do
not drift. The Renderer remains responsible only for individual frames.

## Model And Skin Assumptions

The bundled OBJ files are expected to contain these object names:

- `Head`, `Hat Layer`
- `Body`, `Body Layer`
- `Right Arm`, `Right Arm Layer`
- `Left Arm`, `Left Arm Layer`
- `Right Leg`, `Right Leg Layer`
- `Left Leg`, `Left Leg Layer`

`SkinType::Classic` uses `resources/classic.obj`; `SkinType::Slim` uses `resources/slim.obj`.
Textures are sampled with nearest filtering so Minecraft skin pixels stay crisp.

## Coordinate And Angle Conventions

- **Camera**: yaw and pitch are degrees. `yaw` orbits around the Y axis (0° = front-right, 180° = front). `pitch` is measured from horizontal (90° = level). The eye orbits a fixed look-at point at `(0, 1, 0)` with radius `4.0 / scale`.
- **Camera scale** (CLI: `--cam-zoom`): positive. Larger = closer. Also scales the model matrix.
- **Posture**: all joint angles are degrees. 0° = neutral (no rotation from the model's bind pose). Positive yaw turns the head right. Positive pitch tilts the head up / swings limbs forward.
- **Elytra posture**: the mirrored pair rotates around the central upper-back attachment at `(0, 1.5, 0.125)`. Pitch is shared; yaw and roll describe the left side and are mirrored on the right. Local XYZ Euler angles use the upstream `Rz × Ry × Rx` matrix order. `FLYING` uses 20° pitch and a 60° spread.
- **Character rotation**: Euler rotation in X, then Y, then Z order, applied before per-joint matrices.
- **Pivot articulation**: `body_pitch` rotates the full character around `(0, 1, 0)`. Each limb then rotates around a hardcoded joint pivot (e.g., right arm at `(0.3125, 1.375, 0.0)`). The transform is `base × translate(pivot) × rotate × translate(-pivot)`.

## Body Part Ordering

The renderer draws six body parts in a fixed order: Head → Right Arm → Left Arm → Right Leg → Left Leg → Body. Both the uniform computation and the draw loop consume `PART_CONFIGS` in `uniforms.rs`, which carries a `PartId` enum per entry. This guarantees the uniform upload and vertex buffer selection stay in sync — reordering `PART_CONFIGS` automatically reorders both.

## Error Handling

The public API returns `EidolonError` (not `Box<dyn Error>`). Variants:

| Variant | Source |
|---------|--------|
| `Io(std::io::Error)` | File I/O failures |
| `Gpu(String)` | Adapter, device, pipeline, or buffer failures |
| `Model(String)` | Missing or malformed OBJ model parts |
| `Texture(String)` | Skin PNG decode or upload failures |
| `Conversion(String)` | Single→double layer conversion errors |
| `InvalidPath(String)` | Null bytes or unresolvable paths |
| `Surface(wgpu::SurfaceError)` | Window surface acquisition or presentation failures |

Internal `pub(crate)` functions in `readback.rs` and `uniforms.rs` also use `EidolonError`.

## Current Limits

- Skins and Back equipment atlases are loaded from PNG paths or PNG bytes.
- Cape and Elytra share one 2:1 Back equipment atlas (64x32 or an HD multiple) and are selected by
  `BackEquipmentKind`. Either presentation can render without a Body.
- Character GPU resources are bound to the Renderer that loaded them. Clones share immutable GPU
  allocations through `Arc`; a different Renderer must load its own resources.
- The renderer expects a usable `wgpu` backend. Headless rendering still creates a GPU adapter.
- Model loading requires the exact object names listed above.
- Nametags are single-line and render at most 64 Unicode characters.
