# Library Usage

Eidolon can be used as a Rust library. The core rendering engine is provided by the `eidolon-core` package (library crate name `eidolon_core`).

## Add Dependency

For local development or within this monorepo, add a path dependency:

```toml
[dependencies]
eidolon_core = { path = "crates/core" }
```

Or alias it to `eidolon`:

```toml
[dependencies]
eidolon = { package = "eidolon-core", path = "crates/core" }
```

Then import types from `eidolon` (or `eidolon_core`).

## Minimal Example

```rust
use eidolon::{
    camera::Camera,
    character::{Character, SkinType},
    output::{write_output, OutputIntent, StaticFormat},
    renderer::Renderer,
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;

    let body = renderer.load_body("skin.png", SkinType::Classic)?;
    let character = Character::from_body(body);           // stand posture by default
    let camera = Camera::new();                            // yaw=180, pitch=90, scale=1.0

    write_output(
        "output.png",
        OutputIntent::Static {
            format: StaticFormat::Png,
            size: (800, 600),
        },
        |_: Duration| renderer.render(&character, &camera, 800, 600),
    )?;

    Ok(())
}
```

Key points:
- `Character` owns its per-character visual resources. It must contain a `Body` or Back equipment.
- `Body` combines a Skin with `SkinType`; single-layer Skins are expanded automatically when loaded.
- Cloning a `Character`, `Body`, or `BackEquipmentAtlas` shares immutable GPU resources. Resources can only be rendered by the `Renderer` that loaded them; load them again for another Renderer.
- `Camera::new()` defaults: yaw 180° (front view), pitch 90° (level), scale 1.0.
- `StaticFormat` supports `Png` and `WebP`. `OutputIntent` makes the requested format explicit; the output module normalizes the filename extension and returns the final path.
- `AnimationFormat` supports `Gif`, animated `WebP`, `Apng`, and `Avif`. `OutputIntent::Animation` accepts an `AnimationPreset`, FPS, speed multiplier, loop count, and frame producer, then returns the final path.

## Error Handling

The public API returns `EidolonError`, a typed enum:

```rust
use eidolon::{character::SkinType, error::EidolonError};

match renderer.load_body("skin.png", SkinType::Classic) {
    Ok(body) => { /* ... */ }
    Err(EidolonError::Io(e)) => eprintln!("File error: {e}"),
    Err(EidolonError::Texture(msg)) => eprintln!("Texture error: {msg}"),
    Err(e) => eprintln!("Other error: {e}"),
}
```

Variants: `Io`, `Gpu`, `Model`, `Texture`, `Conversion`, `InvalidPath`, `Output`, and `Surface`. The `Io` variant wraps `std::io::Error`; `Output` covers output validation, encoding, and write failures; `Surface` preserves `wgpu::SurfaceError` so a window loop can recover from `Lost` and exit on `OutOfMemory`.

## Custom Pose Example

```rust
use eidolon::{
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, DefaultPostures, SkinType, Posture},
    output::{write_output, OutputIntent, StaticFormat},
    renderer::Renderer,
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;
    let body = renderer.load_body("skin.png", SkinType::Slim)?;
    let atlas = renderer.load_back_equipment_atlas("cape.png")?;
    let mut character = Character::from_body(body);
    character.nametag = Some("Steve".to_owned());
    character.nametag_see_through = false; // optional: fully hide occluded pixels
    character.posture = DefaultPostures::WAVE;
    character.rotation.y = 20.0;    // 20° around Y axis
    character.posture.cape_pitch = 10.8;
    character.equip(BackEquipment::new(atlas, BackEquipmentKind::Cape));

    let camera = Camera {
        yaw: 210.0,
        pitch: 90.0,
        scale: 1.2,
    };

    write_output(
        "wave.png",
        OutputIntent::Static {
            format: StaticFormat::Png,
            size: (1024, 1024),
        },
        |_: Duration| renderer.render(&character, &camera, 1024, 1024),
    )?;

    Ok(())
}
```

## Custom Posture

Build a `Posture` directly. All angles are degrees, 0° = neutral (no rotation from bind pose):

```rust
let posture = Posture {
    cape_pitch: 10.8,        // recommended cape angle for this posture
    body_pitch: 0.0,         // whole-character pitch around its center
    elytra_pitch: 15.0,      // shared Elytra pitch
    elytra_yaw: 0.5729578,   // left-side yaw; mirrored on the right
    elytra_roll: 15.0,       // left-side roll; mirrored on the right
    head_yaw: 0.0,           // 0° = facing forward, positive = turn right
    head_pitch: 0.0,         // 0° = level, positive = look up
    left_arm_roll: 0.0,      // 0° = neutral
    left_arm_pitch: 0.0,     // 0° = arm down, negative = swing forward
    right_arm_roll: 0.0,
    right_arm_pitch: 0.0,
    left_leg_pitch: 0.0,     // 0° = straight down
    right_leg_pitch: 0.0,
};
```

Presets are available via `DefaultPostures`:

| Constant | Description |
|----------|-------------|
| `DefaultPostures::STAND` | Neutral standing pose; cape 10.8° |
| `DefaultPostures::WAVE` | Left arm raised in a wave; cape 10.8° |
| `DefaultPostures::WALKING` | Arms and legs in walking swing; cape baseline 10.8° |
| `DefaultPostures::RUNNING` | Arms and legs in running swing; cape baseline 54° |
| `DefaultPostures::FLYING` | Horizontal player with neutral limbs, 20° pitch, and 60° elytra spread |

## Procedural Animation

`AnimationPreset` remains the stateless sampling primitive: callers provide a time and receive a
complete `Posture`. `AnimationTimeline` adds portable playback state, while `AnimationSchedule`
provides deterministic sample times for offline output.

```rust
use eidolon::animation::AnimationPreset;
use std::time::Duration;

let animation = AnimationPreset::Walk;
let time = Duration::from_millis(250);
character.posture = animation.sample(time);

assert_eq!(
    animation.sample(Duration::ZERO),
    animation.sample(animation.cycle_duration()),
);
```

Available presets are `Idle` (4s), `Walk` (1s), `Run` (0.6s), `Wave` (1.2s), and `Fly` (1.5s).
Timeline callers provide frame deltas, so native `Instant` and browser animation timestamps remain
in their adapters. The timeline owns speed, play/pause, seeking, loop/end behavior, and ignores
frame gaps over 250ms. Update `Character::posture` from `timeline.sample()` before rendering.

## Skin Conversion

Convert legacy single-layer skins to double-layer without rendering:

```rust
use eidolon::converter;

let img = image::open("old_skin.png")?;
let double_layer = converter::single2double(&img)?;
double_layer.save("new_skin.png")?;
```

The input must have `width == height * 2` (e.g., 64×32). Returns `EidolonError::Conversion` on invalid input.

## Texture Inspection & Classification

Inspect and classify Minecraft skin, cape, or elytra textures from image bytes:

```rust
use eidolon::classifier::{inspect_texture, TextureKind};

let bytes = std::fs::read("skin.png")?;
let report = inspect_texture(&bytes)?;

println!("Detected kind: {}", report.kind.as_str());
println!("Dimensions: {}x{}", report.width, report.height);
println!("HD scale: {}x (is_hd={})", report.hd_scale, report.is_hd);
println!("Can auto-upgrade: {}", report.can_auto_upgrade);
```

Supported kinds include `SkinDoubleLayer`, `SkinSingleLayer`, `Cape`, `Elytra`, `CapeAndElytra`, and `Unknown`.

## Windowed Preview

For interactive preview, use `Renderer::new_windowed` with a `winit` window:

```rust
use std::sync::Arc;
use winit::window::Window;

let window = Arc::new(/* winit Window */);
let renderer = Renderer::new_windowed(window)?;
let body = renderer.load_body("skin.png", SkinType::Classic)?;
let character = Character::from_body(body);

// In your event loop:
renderer.render_frame(&character, &camera)?;
```

## Notes

- `Renderer::new()` and `Renderer::new_windowed()` load both bundled OBJ models from `resources/`, so run from the repository root or keep those resource paths available.
- `Model::load_from_obj_bytes()` is available for loading OBJ data from memory.
- `Camera::scale` (renamed `cam_zoom` in the CLI) magnifies with a lens zoom: the orbit radius is
  fixed and the field of view narrows as `scale` grows.
- Character rotation is Euler: X first, then Y, then Z.
- Set `Character::nametag` to `Some(name)` to draw an unlit, camera-facing label above the player.
  The label uses the bundled Minecraft pixel font (with Unifont fallback for Unicode), a white glyph shadow, a translucent black
  background, and scene depth testing. Nametags are single-line and limited to 64 characters;
  characters missing from the bundled font use its standard missing-glyph box.
- Nametag spacing is applied along the scene's vertical Y axis after finding the highest point of
  the transformed player collision box's scene-space AABB. The label is centered over that AABB.
  The collision box is 1.8 world units tall and the default gap is 0.5 units.
  Occluded pixels render at 25% opacity by default, followed by the normal depth-tested pass. Set
  `Character::nametag_see_through` to `false` to hide occluded pixels completely.
- Load a shared atlas with `Renderer::load_back_equipment_atlas()`, then construct
  `BackEquipment::new(atlas, BackEquipmentKind::Cape)` or `BackEquipmentKind::Elytra`. Changing
  `BackEquipment::kind` switches presentation without reloading the atlas.
- A Character may contain Back equipment without a Body, so Cape and Elytra can render alone in
  the same player coordinate system. `Posture::cape_pitch` rotates Cape around the top-back
  attachment edge; the skinview3d-compatible default is `10.8` degrees.
- Elytra use the same 2:1 Back equipment atlas format (64x32 or an HD multiple). Posture controls
  the mirrored Elytra pair through `elytra_pitch`, `elytra_yaw`, and `elytra_roll`; both sides
  rotate around the central upper-back attachment, with yaw and roll mirrored on the right.
  `DefaultPostures::FLYING`
  uses `20°` pitch and a `60°` spread; applications can construct other
  snapshots with the same fields. `BackEquipmentKind` makes Cape and Elytra mutually exclusive.

![Elytra rendered from the rear](elytra-preview.png)
