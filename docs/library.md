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
use eidolon::{camera::Camera, character::Character, renderer::{OutputFormat, Renderer}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new();

    let mut character = Character::new();
    character.skin = Some(renderer.load_texture("resources/bingling_sama.png")?);

    let camera = Camera::new();

    renderer.render_to_image(
        &character,
        &camera,
        "output.png",
        (800, 600),
        OutputFormat::Png,
    )?;

    Ok(())
}
```

## Notes

- Single-layer skins are expanded to double-layer automatically when loaded.
- `OutputFormat` supports `Png` and `WebP`.
- For full control over posture, camera, and transforms, adjust `Character` and `Camera` fields directly before rendering.
