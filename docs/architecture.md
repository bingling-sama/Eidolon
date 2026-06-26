# Architecture

Eidolon is split into a small CLI binary and a reusable rendering library. The binary parses scene
options and delegates to the library; the library owns GPU setup, skin loading, model loading, and
image output.

## Crate Layout

- `src/main.rs` defines the `eidolon` binary and the `render`, `preview`, and `convert` subcommands.
- `src/lib.rs` exposes the library modules.
- `src/error.rs` defines `EidolonError` — the typed error enum returned by all public APIs.
- `src/camera.rs` computes view and projection matrices for an orbit camera.
- `src/character.rs` defines skin geometry selection, posture presets (0° = neutral for all joints),
  and character transforms.
- `src/model.rs` loads the classic and slim OBJ meshes from `resources/`. Supports loading from
  file path or in-memory bytes.
- `src/texture.rs` loads PNG skins and expands legacy single-layer skins when required.
- `src/converter.rs` implements single-layer to double-layer skin atlas conversion.
- `src/renderer/` contains the shared `wgpu` renderer split into submodules:
  - `mod.rs` — `Renderer` struct: headless + windowed creation, render pass encoding, public API.
  - `pipeline.rs` — wgpu render pipeline creation from the embedded WGSL shader.
  - `readback.rs` — GPU → CPU buffer copy with row-padding, mapped readback to `ImageBuffer`.
  - `uniforms.rs` — per-body-part uniform data, `PART_CONFIGS` with `PartId` enum for draw-order safety.
- `benches/performance_benchmark.rs` renders a fixed batch of images for Criterion benchmarks.

## Render Flow

For `cargo run -- render skin.png`, the flow is:

1. `src/main.rs` parses CLI options into `SceneArgs`.
2. `character_and_camera_from_scene` builds a `Character` and `Camera`.
3. `Renderer::new` creates a headless `wgpu` device, queue, render pipeline, and loads both OBJ
   model variants.
4. `Renderer::load_texture` reads the PNG skin. If the image has `width == height * 2`, it is
   converted to a square double-layer atlas before upload via `converter::single2double`.
5. `Renderer::render_to_image` renders to an off-screen `Rgba8Unorm` texture, copies the padded
   GPU rows into a CPU buffer, strips padding, and saves the image as PNG or WebP. The output
   filename extension is auto-adjusted to match the format.

For `preview`, the same scene data and pipeline are used, but `Renderer::new_windowed` creates a
window surface and `render_frame` presents each frame to the swapchain.

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
- **Character rotation**: Euler rotation in X, then Y, then Z order, applied before per-joint matrices.
- **Pivot articulation**: each limb rotates around a hardcoded joint pivot (e.g., right arm at `(0.3125, 1.375, 0.0)`). The transform is `base × translate(pivot) × rotate × translate(-pivot)`. The body has no pivot and uses the base transform directly.

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

Internal `pub(crate)` functions in `readback.rs` and `uniforms.rs` also use `EidolonError`.

## Current Limits

- Skin textures are loaded from PNG files only.
- The renderer expects a usable `wgpu` backend. Headless rendering still creates a GPU adapter.
- Model loading requires the exact object names listed above.
