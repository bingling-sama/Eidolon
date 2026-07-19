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
use eidolon::{
    camera::Camera,
    character::{Character, SkinType},
    renderer::{OutputFormat, Renderer},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;

    let mut character = Character::new();
    character.skin_type = SkinType::Classic;
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
- 先调用 `Renderer::load_cape_texture()` 并设置 `Character::cape` 才会渲染披风；没有内置
  默认披风贴图。`Character::cape_pitch` 控制披风绕肩部背面顶边旋转，与 skinview3d
  一致的默认值为 `10.8` 度。
- 默认姿势包含建议的披风角度：`STAND`、`WAVE`、`WALKING` 为 `10.8` 度，
  `RUNNING` 为 `54` 度。CLI 选择姿势时会自动应用该角度。
- 渲染前必须设置 `Character::skin`；否则渲染器会以 `No skin texture available` panic。
- `Renderer::new` 和 `Renderer::new_windowed` 会从 `resources/` 加载两套内置 OBJ 模型，因此建议从仓库根目录运行，或确保这些资源路径可用。

## 自定义姿势示例

```rust
use eidolon::{
    camera::Camera,
    character::{Character, DefaultPostures, SkinType},
    renderer::{OutputFormat, Renderer},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = Renderer::new()?;

    let mut character = Character::new();
    character.skin_type = SkinType::Slim;
    character.posture = DefaultPostures::WAVE;
    character.rotation.y = 20.0;
    character.cape = true;
    character.cape_pitch = 10.8;
    renderer.load_cape_texture("cape.png")?;
    character.skin = Some(renderer.load_texture("resources/bingling_sama.png")?);

    let camera = Camera {
        yaw: 210.0,
        pitch: 90.0,
        scale: 1.2,
    };

    renderer.render_to_image(
        &character,
        &camera,
        "wave.png",
        (1024, 1024),
        OutputFormat::Png,
    )?;

    Ok(())
}
```
