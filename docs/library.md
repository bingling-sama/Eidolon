# Library Usage

Eidolon can be used as a Rust library. The package name is `eidolonmc`, and the library crate name is `eidolon`.

## Add Dependency

For local development, use a path dependency:

```toml
[dependencies]
eidolonmc = { path = "../eidolon" }
```

Then import the library crate as `eidolon`.

## Minimal Example

```rust
use eidolon::{
    camera::Camera,
    character::Character,
    renderer::{OutputFormat, Renderer},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;

    let character = Character::new();                     // classic arms, stand pose
    let skin = renderer.load_texture("skin.png")?;        // auto-converts single→double layer
    let camera = Camera::new();                           // yaw=180, pitch=90, scale=1.0

    renderer.render_to_image(
        &character,
        &skin,
        &camera,
        "output.png",
        (800, 600),
        OutputFormat::Png,
    )?;

    Ok(())
}
```

Key points:
- `Character` no longer holds the skin texture. Pass `&Texture` to render methods — the compiler guarantees it's loaded before rendering.
- `Camera::new()` defaults: yaw 180° (front view), pitch 90° (level), scale 1.0.
- Single-layer skins are expanded to double-layer automatically when loaded.
- `OutputFormat` supports `Png` and `WebP`. Format is inferred from the filename extension by `render_to_image`.

## Error Handling

The public API returns `EidolonError`, a typed enum:

```rust
use eidolon::error::EidolonError;

match renderer.load_texture("skin.png") {
    Ok(skin) => { /* ... */ }
    Err(EidolonError::Io(e)) => eprintln!("File error: {e}"),
    Err(EidolonError::Texture(msg)) => eprintln!("Texture error: {msg}"),
    Err(e) => eprintln!("Other error: {e}"),
}
```

Variants: `Io`, `Gpu`, `Model`, `Texture`, `Conversion`, `InvalidPath`. The `Io` variant wraps `std::io::Error` and exposes it via `Error::source()`.

## Custom Pose Example

```rust
use eidolon::{
    camera::Camera,
    character::{Character, DefaultPostures, SkinType, Posture},
    renderer::{OutputFormat, Renderer},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;

    let mut character = Character::new();
    character.skin_type = SkinType::Slim;
    character.posture = DefaultPostures::WAVE;
    character.rotation.y = 20.0;    // 20° around Y axis

    let skin = renderer.load_texture("skin.png")?;

    let camera = Camera {
        yaw: 210.0,
        pitch: 90.0,
        scale: 1.2,
    };

    renderer.render_to_image(
        &character,
        &skin,
        &camera,
        "wave.png",
        (1024, 1024),
        OutputFormat::Png,
    )?;

    Ok(())
}
```

## Custom Posture

Build a `Posture` directly. All angles are degrees, 0° = neutral (no rotation from bind pose):

```rust
let posture = Posture {
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
| `DefaultPostures::STAND` | Neutral standing pose (all 0°) |
| `DefaultPostures::WAVE` | Left arm raised in a wave |
| `DefaultPostures::WALKING` | Arms and legs in walking swing |
| `DefaultPostures::RUNNING` | Arms and legs in running swing |

## Skin Conversion

Convert legacy single-layer skins to double-layer without rendering:

```rust
use eidolon::converter;

let img = image::open("old_skin.png")?;
let double_layer = converter::single2double(&img)?;
double_layer.save("new_skin.png")?;
```

The input must have `width == height * 2` (e.g., 64×32). Returns `EidolonError::Conversion` on invalid input.

## Windowed Preview

For interactive preview, use `Renderer::new_windowed` with a `winit` window:

```rust
use std::sync::Arc;
use winit::window::Window;

let window = Arc::new(/* winit Window */);
let renderer = Renderer::new_windowed(window)?;
let skin = renderer.load_texture("skin.png")?;

// In your event loop:
renderer.render_frame(&character, &skin, &camera)?;
```

## Notes

- `Renderer::new()` and `Renderer::new_windowed()` load both bundled OBJ models from `resources/`, so run from the repository root or keep those resource paths available.
- `Model::load_from_obj_bytes()` is available for loading OBJ data from memory.
- `Camera::scale` (renamed `cam_zoom` in the CLI) controls orbit distance: `distance = 4.0 / scale`. Larger values move the camera closer.
- Character rotation is Euler: X first, then Y, then Z.
