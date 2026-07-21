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
- 将 `Character::nametag` 设为 `Some(name)` 可在玩家头顶显示始终朝向相机的标签。标签使用
  内置 Monocraft 像素字体、白字深色阴影、半透明黑底和场景深度测试；仅支持单行，最多渲染
  64 个字符；字体不包含的字形会显示字体自身的缺字方框。
- 标签先变换玩家碰撞箱并计算场景空间 AABB，定位在 AABB 水平中心的最高点上方，再沿场景垂直
  Y 轴增加间距。碰撞箱高度为 1.8 个世界单位，默认间距为 0.5 个单位，二者都会随
  `Camera::scale` 缩放。被遮挡的像素默认以 25% 透明度绘制，然后叠加正常深度测试层；将
  `Character::nametag_see_through` 设为 `false` 可完全隐藏被遮挡的像素。
- 先调用 `Renderer::load_cape_texture()` 并设置 `Character::cape` 才会渲染披风；没有内置
  默认披风贴图。`Character::cape_pitch` 控制披风绕肩部背面顶边旋转，与 skinview3d
  一致的默认值为 `10.8` 度。
- 先调用 `Renderer::load_elytra_texture()` 并设置 `Character::elytra` 才会渲染鞘翅。
  鞘翅使用相同的 2:1 披风图集格式（64×32 或等比例高清图集）。姿势中的
  `elytra_pitch`、`elytra_yaw`、`elytra_roll` 控制左翼；两翼绕中央上背连接点旋转，
  右翼会镜像 yaw 和 roll；
  `DefaultPostures::FLYING` 会将玩家整体转至水平，并使用 `20°` pitch 和 `60°` 开合角。
  应用可以用相同字段构造其它开合快照，并应只启用披风或鞘翅中的一种。

![从背面渲染的鞘翅](elytra-preview.png)
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
    character.nametag = Some("Steve".to_owned());
    character.nametag_see_through = false; // 可选：完全隐藏被遮挡的部分
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
