# Architecture

Eidolon is split into a small CLI binary and a reusable rendering library. The binary parses scene
options and delegates to the library; the library owns GPU setup, skin loading, model loading, and
image output.

## Crate Layout

- `src/main.rs` defines the `eidolon` binary and the `render`, `preview`, and `convert` subcommands.
- `src/lib.rs` exposes the library modules.
- `src/camera.rs` computes view and projection matrices for an orbit camera.
- `src/character.rs` defines skin geometry selection, posture presets, and character transforms.
- `src/model.rs` loads the classic and slim OBJ meshes from `resources/`.
- `src/texture.rs` loads PNG skins and expands legacy single-layer skins when required.
- `src/renderer/` contains the shared `wgpu` renderer, output encoding, readback, pipeline, and
  uniform logic.
- `src/utils/converter.rs` implements single-layer to double-layer skin atlas conversion.
- `benches/performance_benchmark.rs` renders a fixed batch of images for Criterion benchmarks.

## Render Flow

For `cargo run -- render ...`, the flow is:

1. `src/main.rs` parses CLI options into `SceneArgs`.
2. `character_and_camera_from_scene` builds a `Character` and `Camera`.
3. `Renderer::new` creates a headless `wgpu` device, queue, render pipeline, and loads both OBJ
   model variants.
4. `Renderer::load_texture` reads the PNG skin. If the image has `width == height * 2`, it is
   converted to a square double-layer atlas before upload.
5. `Renderer::render_to_image` renders to an off-screen `Rgba8Unorm` texture, copies the padded
   GPU rows into a CPU buffer, strips padding, and saves the image as PNG or WebP.

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

## Coordinate And Angle Notes

- Camera yaw and pitch are degrees.
- `Camera::scale` is a positive zoom factor. Larger values move the camera closer and also scale the
  model in the current uniform path.
- Character rotation is Euler rotation in X, then Y, then Z order.
- Posture fields are degrees. Some fields are interpreted relative to the neutral Minecraft pose,
  for example head and leg pitch values are offset by `90` degrees in the renderer.

## Current Limits

- Skin textures are loaded from PNG files only.
- `Character::cape` and `Character::nametag` are reserved fields and are not rendered yet.
- The renderer expects a usable `wgpu` backend. Headless rendering still creates a GPU adapter.
