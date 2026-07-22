# 命令行参考

Eidolon 提供一个可执行程序，包含四个子命令：`render`、`animate`、`preview`、`convert`。

## Render

将皮肤渲染为图片文件（无窗口）。

```bash
cargo run -- render [OPTIONS]
```

选项：

- `--filename <PATH>` 输出文件路径。默认：`output.png`
- `--format <png|webp>` 输出格式。默认：`png`
- `--width <PX>` 输出宽度（像素）。默认：`800`
- `--height <PX>` 输出高度（像素）。默认：`600`
- `--texture <PATH>` 皮肤 PNG 路径。默认：`resources/bingling_sama.png`
- `--skin-type <classic|slim>` 手臂类型，必填
- `--nametag <TEXT>` 在模型头顶渲染 Minecraft 风格玩家名；默认不显示
- `--disable-nametag-see-through` 让被场景几何遮挡的 nametag 像素完全隐藏；默认以低透明度显示
- `--cape <PNG>` 使用指定的后背装备图集渲染披风；没有默认图集，省略时不渲染
- `--elytra <PNG>` 使用 2:1 披风图集（64×32 或等比例高清图集）渲染鞘翅；与 `--cape` 互斥
- `--cape-pitch <DEG>` 覆盖所选姿势的披风角度；默认使用姿势自带角度
- `--elytra-pitch <DEG>` 覆盖所选姿势的鞘翅 pitch；默认使用姿势自带角度
- `--elytra-spread <DEG>` 覆盖所选姿势的鞘翅开合角；默认使用姿势自带角度
- `--yaw <DEG>` 相机 yaw。默认：`180`
- `--pitch <DEG>` 相机 pitch。默认：`90`
- `--scale <FLOAT>` 相机距离缩放（必须 > 0）。默认：`1.0`
- `--posture <stand|wave|walking|running|flying>` 动作类型。`flying` 会展开左右鞘翅。默认：`stand`
- `--head-yaw <DEG>` 覆盖头部 yaw
- `--head-pitch <DEG>` 覆盖头部 pitch
- `--left-arm-roll <DEG>` 覆盖左臂 roll
- `--left-arm-pitch <DEG>` 覆盖左臂 pitch
- `--right-arm-roll <DEG>` 覆盖右臂 roll
- `--right-arm-pitch <DEG>` 覆盖右臂 pitch
- `--left-leg-pitch <DEG>` 覆盖左腿 pitch
- `--right-leg-pitch <DEG>` 覆盖右腿 pitch
- `--position-x <FLOAT>` 角色位置 X。默认：`0`
- `--position-y <FLOAT>` 角色位置 Y。默认：`0`
- `--position-z <FLOAT>` 角色位置 Z。默认：`0`
- `--rotation-x <DEG>` 角色绕 X 旋转。默认：`0`
- `--rotation-y <DEG>` 角色绕 Y 旋转。默认：`0`
- `--rotation-z <DEG>` 角色绕 Z 旋转。默认：`0`

示例：

```bash
cargo run -- render --skin-type classic --texture resources/bingling_sama.png \
  --width 1024 --height 768 --yaw 210 --pitch 90 --scale 1.2 --format png

cargo run -- render resources/bingling_sama.png --cape cape.png --cape-pitch 25
cargo run -- render resources/bingling_sama.png --elytra cape.png --elytra-pitch 18 --elytra-spread 45
cargo run -- render resources/bingling_sama.png --nametag Steve
cargo run -- render resources/bingling_sama.png --nametag Steve --disable-nametag-see-through
```

如果使用 `--format webp` 且保持默认 `--filename output.png`，CLI 会把输出路径改为
`output.webp`。自定义文件名会按原样使用。

`--filename` 不能包含 `..` 路径片段，避免渲染输出通过目录穿越写到预期目录之外。

`--posture` 是静态渲染、动画导出和窗口预览共用的动作选择器：`stand` 对应 Idle、
`wave` 对应 Wave、`walking` 对应 Walk、`running` 对应 Run、`flying` 对应 Fly。
静态渲染采用该动画的代表帧；Walk 和 Run 采用四分之一周期，其他动作采用相位 0。

## Animate

将一个完整的程序化动画周期导出为 GIF 或 Animated WebP：

```bash
eidolon animate <皮肤> <输出.gif|输出.webp> [选项]
```

- `--posture <stand|wave|walking|running|flying>` 动作类型，默认 `stand`
- `--fps <1..60>` 帧率，默认 `30`
- `--speed <0.25..4.0>` 播放速度，默认 `1.0`
- `--loop-count <N>` 循环元数据；`0` 表示无限循环，默认 `0`
- `--width <1..2048>`、`--height <1..2048>` 输出尺寸，默认 `800×600`

该命令也支持 `render` 的场景、相机、后背装备和关节覆盖选项。覆盖值会在每帧采样后应用，
因此显式设置的关节、披风或鞘翅角度保持固定，其余部位继续动画。
Animated WebP 使用无损 RGBA 编码；GIF 量化为 256 色，透明度表现有限。

```bash
eidolon animate skin.png walk.gif --posture walking
eidolon animate skin.png fly.webp --posture flying --elytra cape.png --fps 60 --speed 1.5
```

## Preview

打开实时预览窗口。

```bash
cargo run -- preview [OPTIONS]
```

`preview` 支持与 `render` 相同的场景、相机、姿势和视口选项，但不包含输出路径与格式。
窗口初始显示 `--posture` 对应动画的代表帧并保持暂停；按 `Space` 后会从同一帧开始播放该动作。

在预览窗口中，按住鼠标左键拖拽可平移，按住右键拖拽可旋转，滚动滚轮可缩放。
`Space` 播放/暂停，左右方向键按 30fps 单帧 seek，数字键 `1–5` 切换 Idle、Walk、Run、Wave、Fly，
`-`/`+` 调整速度。切换动作后会回到相位 0。

示例：

```bash
cargo run -- preview --skin-type slim --texture resources/bingling_sama.png --yaw 200
cargo run -- preview resources/bingling_sama.png --cape cape.png
cargo run -- preview resources/bingling_sama.png --elytra cape.png
cargo run -- preview resources/bingling_sama.png --nametag Steve
cargo run -- preview resources/bingling_sama.png --posture walking
```

## Convert

将旧版单层皮肤图集转换为正方形双层图集。

```bash
cargo run -- convert <INPUT> [OUTPUT]
```

参数：

- `<INPUT>` 输入 PNG（宽度必须是高度的两倍）
- `[OUTPUT]` 输出 PNG 路径。默认：`output.png`

示例：

```bash
cargo run -- convert old_skin.png new_skin.png
```

## 帮助

使用 `--help` 查看最新的完整选项列表。

排查渲染或资源加载问题时，可以启用日志：

```bash
RUST_LOG=info cargo run -- render --skin-type classic
```

常见错误见 `troubleshooting_zh-CN.md`。
