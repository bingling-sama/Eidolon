# 作为库使用

Eidolon 可以作为 Rust 库使用。包名为 `eidolonmc`，库 crate 名为 `eidolon`。

## 添加依赖

本地开发可使用 path 依赖：

```toml
[dependencies]
eidolonmc = { path = "../eidolon" }
```

然后以 `eidolon` 引入库。

## 最小示例

```rust
use eidolon::{camera::Camera, character::Character, renderer::{OutputFormat, Renderer}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;

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

## 说明

- 单层皮肤在加载时会自动转换为双层。
- `OutputFormat` 支持 `Png` 与 `WebP`。
- 如需更精细的姿势、相机和变换控制，可直接调整 `Character` 与 `Camera` 的字段。
