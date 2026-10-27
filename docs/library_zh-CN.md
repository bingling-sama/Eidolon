# 作为库使用

Eidolon 可以作为 Rust 库使用。核心渲染引擎由 `eidolon-core` 提供（库 crate 名为 `eidolon_core`）。

## 添加依赖

本地开发或在 Monorepo 内使用 path 依赖：

```toml
[dependencies]
eidolon_core = { path = "crates/core" }
```

或别名为 `eidolon`：

```toml
[dependencies]
eidolon = { package = "eidolon-core", path = "crates/core" }
```

然后以 `eidolon`（或 `eidolon_core`）引入库。

## 最小示例

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

    let body = renderer.load_body("resources/bingling_sama.png", SkinType::Classic)?;
    let character = Character::from_body(body);

    let camera = Camera::new();

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

## 说明

- `Character` 持有角色自己的可视资源，并且必须至少包含 `Body` 或后背装备。
- `Body` 将 Skin 与 `SkinType` 组合在一起。单层 Skin 在加载时会自动转换为双层。
- 克隆 `Character`、`Body` 或 `BackEquipmentAtlas` 会共享不可变 GPU 资源。这些资源只能由创建它们的 Renderer 渲染；另一个 Renderer 必须重新加载资源。
- `StaticFormat` 支持 `Png` 与 `WebP`。`OutputIntent` 显式指定格式，输出模块会统一扩展名并返回最终路径。
- `AnimationFormat` 支持 `Gif`、Animated `WebP`、`Apng` 与 `Avif`。`OutputIntent::Animation` 接受 `AnimationPreset`、FPS、速度倍率、循环次数和 frame producer，并返回最终路径。
- 公共错误 `EidolonError::Output` 表示输出校验、编码或写入失败。
- 如需更精细的姿势、相机和变换控制，可直接调整 `Character` 与 `Camera` 的字段。
- 将 `Character::nametag` 设为 `Some(name)` 可在玩家头顶显示始终朝向相机的标签。标签使用
  内置 Minecraft 像素字体与 Unifont 回退字体、白字深色阴影、半透明黑底和场景深度测试；仅支持单行，最多渲染
  64 个字符；字体不包含的字形会显示字体自身的缺字方框。
- 标签先变换玩家碰撞箱并计算场景空间 AABB，定位在 AABB 水平中心的最高点上方，再沿场景垂直
  Y 轴增加间距。碰撞箱高度为 1.8 个世界单位，默认间距为 0.5 个单位。被遮挡的像素默认以 25% 透明度绘制，然后叠加正常深度测试层；将
  `Character::nametag_see_through` 设为 `false` 可完全隐藏被遮挡的像素。
- 使用 `Renderer::load_back_equipment_atlas()` 加载披风与鞘翅共享的 2:1 图集，再用
  `BackEquipment::new(atlas, BackEquipmentKind::Cape)` 或 `Elytra` 选择呈现形式。修改
  `BackEquipment::kind` 不需要重新加载图集。
- Character 可以只有后背装备而没有 Body，因此披风和鞘翅都可以在玩家坐标系中独立渲染。
  `Posture::cape_pitch` 控制披风绕肩部背面顶边旋转，与 skinview3d 一致的默认值为
  `10.8` 度。鞘翅使用同一图集，姿势中的
  `elytra_pitch`、`elytra_yaw`、`elytra_roll` 控制左侧鞘翅；左右两片绕中央上背连接点旋转，
  右侧会镜像 yaw 和 roll；
  `DefaultPostures::FLYING` 会将玩家整体转至水平，并使用 `20°` pitch 和 `60°` 开合角。
  应用可以用相同字段构造其它开合快照。`BackEquipmentKind` 保证一次只选择一种呈现形式。

![从背面渲染的鞘翅](elytra-preview.png)
- 默认姿势包含建议的披风角度：`STAND`、`WAVE`、`WALKING` 为 `10.8` 度，
  `RUNNING` 为 `54` 度。CLI 选择姿势时会自动应用该角度。
- `Renderer::new` 和 `Renderer::new_windowed` 会从 `resources/` 加载两套内置 OBJ 模型，因此建议从仓库根目录运行，或确保这些资源路径可用。
- `Camera::scale`（CLI 里是 `cam_zoom`）以镜头变焦实现放大：相机轨道半径固定，视场角随 `scale` 增大而变窄。

## 程序化动画

`AnimationPreset` 保持为无状态采样基础：调用方传入时间，得到完整 `Posture`。
`AnimationTimeline` 提供跨平台播放状态，`AnimationSchedule` 为离线输出提供确定性采样时刻。

```rust
use eidolon::animation::AnimationPreset;
use std::time::Duration;

let animation = AnimationPreset::Walk;
character.posture = animation.sample(Duration::from_millis(250));
assert_eq!(
    animation.sample(Duration::ZERO),
    animation.sample(animation.cycle_duration()),
);
```

内置动作包括 `Idle`（4 秒）、`Walk`（1 秒）、`Run`（0.6 秒）、`Wave`（1.2 秒）和
`Fly`（1.5 秒）。adapter 向 timeline 提供帧间隔，因此 native `Instant` 与浏览器动画时间戳
仍留在各自 adapter。timeline 负责速度、播放/暂停、seek、循环/结束，并忽略超过 250ms 的帧间隔。
每次 Renderer 调用前使用 `timeline.sample()` 更新 `Character::posture`。

## 自定义姿势示例

```rust
use eidolon::{
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, DefaultPostures, SkinType},
    output::{write_output, OutputIntent, StaticFormat},
    renderer::Renderer,
};
use std::time::Duration;

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

## 皮肤转换

无需初始化渲染器即可将旧版单层皮肤转换为双层皮肤：

```rust
use eidolon::converter;

let img = image::open("old_skin.png")?;
let double_layer = converter::single2double(&img)?;
double_layer.save("new_skin.png")?;
```

输入图像必须满足 `width == height * 2`（如 64×32）。输入不合法时返回 `EidolonError::Conversion`。

## 纹理检测与分类

从图片字节自动识别并分类 Minecraft 皮肤、披风或鞘翅纹理：

```rust
use eidolon::classifier::{inspect_texture, TextureKind};

let bytes = std::fs::read("skin.png")?;
let report = inspect_texture(&bytes)?;

println!("检测类型: {}", report.kind.as_str());
println!("尺寸: {}x{}", report.width, report.height);
println!("高清倍率: {}x (is_hd={})", report.hd_scale, report.is_hd);
println!("是否支持自动升级为双层: {}", report.can_auto_upgrade);
```

支持类型包括 `SkinDoubleLayer`、`SkinSingleLayer`、`Cape`、`Elytra`、`CapeAndElytra` 及 `Unknown`。

## 窗口预览

交互式预览使用 `Renderer::new_windowed` 配合 `winit` 窗口：

```rust
use std::sync::Arc;
use winit::window::Window;

let window = Arc::new(/* winit Window */);
let renderer = Renderer::new_windowed(window)?;
let body = renderer.load_body("skin.png", SkinType::Classic)?;
let character = Character::from_body(body);

// 在事件循环中：
renderer.render_frame(&character, &camera)?;
```

用 `with_transparent(true)` 创建的窗口依然通过 HWND 交换链呈现，在 Windows 上会忽略逐像素 alpha。若要在 Windows 上得到透明背景，需要创建不带重定向表面的窗口，并让渲染器呈现到 DirectComposition visual：

```rust
#[cfg(windows)]
let renderer = unsafe { Renderer::new_windowed_composition(visual_ptr, width, height)? };
```

`visual_ptr` 是 `IDCompositionVisual` 背后的裸 `*mut c_void`，生命周期必须长于渲染器。该构造函数强制使用 DX12 后端，它是唯一能呈现到 composition visual 的后端。

