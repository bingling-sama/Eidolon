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

    let body = renderer.load_body("resources/bingling_sama.png", SkinType::Classic)?;
    let character = Character::from_body(body);

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

- `Character` 持有角色自己的可视资源，并且必须至少包含 `Body` 或后背装备。
- `Body` 将 Skin 与 `SkinType` 组合在一起。单层 Skin 在加载时会自动转换为双层。
- 克隆 `Character`、`Body` 或 `BackEquipmentAtlas` 会共享不可变 GPU 资源。这些资源只能由创建它们的 Renderer 渲染；另一个 Renderer 必须重新加载资源。
- `OutputFormat` 支持 `Png` 与 `WebP`。
- 如需更精细的姿势、相机和变换控制，可直接调整 `Character` 与 `Camera` 的字段。
- 将 `Character::nametag` 设为 `Some(name)` 可在玩家头顶显示始终朝向相机的标签。标签使用
  内置 Monocraft 像素字体、白字深色阴影、半透明黑底和场景深度测试；仅支持单行，最多渲染
  64 个字符；字体不包含的字形会显示字体自身的缺字方框。
- 标签先变换玩家碰撞箱并计算场景空间 AABB，定位在 AABB 水平中心的最高点上方，再沿场景垂直
  Y 轴增加间距。碰撞箱高度为 1.8 个世界单位，默认间距为 0.5 个单位，二者都会随
  `Camera::scale` 缩放。被遮挡的像素默认以 25% 透明度绘制，然后叠加正常深度测试层；将
  `Character::nametag_see_through` 设为 `false` 可完全隐藏被遮挡的像素。
- 使用 `Renderer::load_back_equipment_atlas()` 加载披风与鞘翅共享的 2:1 图集，再用
  `BackEquipment::new(atlas, BackEquipmentKind::Cape)` 或 `Elytra` 选择呈现形式。修改
  `BackEquipment::kind` 不需要重新加载图集。
- Character 可以只有后背装备而没有 Body，因此披风和鞘翅都可以在玩家坐标系中独立渲染。
  `Posture::cape_pitch` 控制披风绕肩部背面顶边旋转，与 skinview3d 一致的默认值为
  `10.8` 度。鞘翅使用同一图集，姿势中的
  `elytra_pitch`、`elytra_yaw`、`elytra_roll` 控制左翼；两翼绕中央上背连接点旋转，
  右翼会镜像 yaw 和 roll；
  `DefaultPostures::FLYING` 会将玩家整体转至水平，并使用 `20°` pitch 和 `60°` 开合角。
  应用可以用相同字段构造其它开合快照。`BackEquipmentKind` 保证一次只选择一种呈现形式。

![从背面渲染的鞘翅](elytra-preview.png)
- 默认姿势包含建议的披风角度：`STAND`、`WAVE`、`WALKING` 为 `10.8` 度，
  `RUNNING` 为 `54` 度。CLI 选择姿势时会自动应用该角度。
- `Renderer::new` 和 `Renderer::new_windowed` 会从 `resources/` 加载两套内置 OBJ 模型，因此建议从仓库根目录运行，或确保这些资源路径可用。

## 自定义姿势示例

```rust
use eidolon::{
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, DefaultPostures, SkinType},
    renderer::{OutputFormat, Renderer},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;
    let body = renderer.load_body("resources/bingling_sama.png", SkinType::Slim)?;
    let atlas = renderer.load_back_equipment_atlas("cape.png")?;
    let mut character = Character::from_body(body);
    character.nametag = Some("Steve".to_owned());
    character.nametag_see_through = false; // 可选：完全隐藏被遮挡的部分
    character.posture = DefaultPostures::WAVE;
    character.rotation.y = 20.0;
    character.posture.cape_pitch = 10.8;
    character.equip(BackEquipment::new(atlas, BackEquipmentKind::Cape));

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
