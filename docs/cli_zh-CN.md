# 命令行参考

Eidolon 提供一个可执行程序，包含四个子命令：`render`、`preview`、`convert`、`identify`。

## Render

将皮肤渲染为静态图片或动画文件（无窗口）。

```bash
cargo run -- render [选项] <皮肤 PNG> [输出]
```

选项：

- `[输出]` 静态输出接受 `.png`/`.webp`；动画输出接受 `.gif`/`.webp`/`.apng`/`.avif`。静态默认
  `output.png`，`--animate` 时默认 `output.webp`
- `--width <PX>` 输出宽度（像素）。默认：`800`
- `--height <PX>` 输出高度（像素）。默认：`600`
- `--animate` 输出一个完整动画周期，而不是单帧
- `--fps <1..60>` 动画帧率，默认 `30`；需要 `--animate`
- `--speed <0.25..4.0>` 动画速度，默认 `1.0`；需要 `--animate`
- `--loop-count <N>` 循环元数据，`0` 表示无限循环；需要 `--animate`
- `--slim` 使用 Alex 风格 3px 手臂；默认使用 Steve 风格 4px 手臂
- `--nametag <TEXT>` 在模型头顶渲染 Minecraft 风格玩家名；默认不显示
- `--disable-nametag-see-through` 让被场景几何遮挡的 nametag 像素完全隐藏；默认以低透明度显示
- `--cape <PNG>` 使用指定的后背装备图集渲染披风；没有默认图集，省略时不渲染
- `--elytra <PNG>` 使用 2:1 披风图集（64×32 或等比例高清图集）渲染鞘翅；与 `--cape` 互斥
- `--cape-pitch <DEG>` 覆盖所选姿势的披风角度；默认使用姿势自带角度
- `--elytra-pitch <DEG>` 覆盖所选姿势的鞘翅 pitch；默认使用姿势自带角度
- `--elytra-spread <DEG>` 覆盖所选姿势的鞘翅开合角；默认使用姿势自带角度
- `--cam-yaw <DEG>` 相机 yaw。默认：`180`
- `--cam-pitch <DEG>` 相机 pitch。默认：`90`
- `--cam-zoom <FLOAT>` 相机缩放（必须 > 0），实现为镜头变焦（相机距离固定、视场角变化）。默认：`1.0`
- `--posture <idle|walk|run|wave|fly>` 动作类型。`fly` 会展开左右鞘翅。默认：`idle`
- `--head-yaw <DEG>` 覆盖头部 yaw
- `--head-pitch <DEG>` 覆盖头部 pitch
- `--left-arm-roll <DEG>` 覆盖左臂 roll
- `--left-arm-pitch <DEG>` 覆盖左臂 pitch
- `--right-arm-roll <DEG>` 覆盖右臂 roll
- `--right-arm-pitch <DEG>` 覆盖右臂 pitch
- `--left-leg-pitch <DEG>` 覆盖左腿 pitch
- `--right-leg-pitch <DEG>` 覆盖右腿 pitch
- `--pos-x <FLOAT>`、`--pos-y <FLOAT>`、`--pos-z <FLOAT>` 角色位置，默认 `0`
- `--rot-x <DEG>`、`--rot-y <DEG>`、`--rot-z <DEG>` 角色旋转，默认 `0`

示例：

```bash
cargo run -- render resources/bingling_sama.png output.png \
  --width 1024 --height 768 --cam-yaw 210 --cam-pitch 90 --cam-zoom 1.2

cargo run -- render resources/bingling_sama.png --cape cape.png --cape-pitch 25
cargo run -- render resources/bingling_sama.png --elytra cape.png --elytra-pitch 18 --elytra-spread 45
cargo run -- render resources/bingling_sama.png --nametag Steve
cargo run -- render resources/bingling_sama.png --nametag Steve --disable-nametag-see-through
```

输出格式由扩展名推断。输出路径不能包含 `..` 路径片段。

`--posture` 与 `AnimationPreset` 使用完全相同的名称，并由静态渲染、动画渲染和窗口预览共用。
静态渲染采用代表帧；Walk 和 Run 采用四分之一周期，其他动作采用相位 0。添加 `--animate`
后，输出必须为 `.gif`、`.webp`、`.apng` 或 `.avif`，尺寸不超过 `2048×2048`。显式关节、披风或鞘翅覆盖会在
每帧采样后应用。Animated WebP 与 APNG 使用无损 RGBA 编码；AVIF 编码带 alpha 的动画 AV1 序列；GIF 量化为 256 色。

```bash
eidolon render skin.png walk.gif --animate --posture walk
eidolon render skin.png fly.webp --animate --posture fly --elytra cape.png --fps 60 --speed 1.5
eidolon render skin.png run.apng --animate --posture run
eidolon render skin.png dance.avif --animate --posture dance
```

## Preview

打开实时预览窗口。

```bash
cargo run -- preview <皮肤 PNG> [选项]
```

`preview` 支持与 `render` 相同的场景、相机、姿势和视口选项，但不包含输出路径与格式。
窗口初始显示 `--posture` 对应动画的代表帧并保持暂停；按 `Space` 后会从同一帧开始播放该动作。

在预览窗口中，按住鼠标左键拖拽可平移，按住右键拖拽可旋转，滚动滚轮可缩放。
`Space` 播放/暂停，左右方向键按 30fps 单帧 seek，数字键 `1–5` 切换 Idle、Walk、Run、Wave、Fly，
`-`/`+` 调整速度。切换动作后会回到相位 0。

窗口默认使用不透明深色背景。加上 `--transparent` 会把背景改为逐像素透明：Windows 上走
DirectComposition 交换链，macOS 与 Wayland 使用窗口自带的透明合成，X11 需要合成器（无合成器时
透明区域显示为黑色）。

示例：

```bash
cargo run -- preview resources/bingling_sama.png --slim --cam-yaw 200
cargo run -- preview resources/bingling_sama.png --cape cape.png
cargo run -- preview resources/bingling_sama.png --elytra cape.png
cargo run -- preview resources/bingling_sama.png --nametag Steve
cargo run -- preview resources/bingling_sama.png --posture walk
cargo run -- preview resources/bingling_sama.png --transparent
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

## Identify

识别并分析 Minecraft 纹理（皮肤、披风、鞘翅），检测其尺寸、高清倍率以及是否支持自动升级。

```bash
cargo run -- identify [选项] <纹理 PNG>
```

参数与选项：

- `<纹理 PNG>` 待分析纹理的 PNG 文件路径
- `--json` 以 JSON 格式输出分析结果（默认输出易读文本）

示例：

```bash
cargo run -- identify resources/bingling_sama.png
```

输出示例：
```text
File: resources/bingling_sama.png
Kind: SkinDoubleLayer
Dimensions: 64x64
HD: false (scale 1x)
Auto-upgradeable: false
Description: Modern 1.8+ double-layer player skin (standard resolution)
```

以 JSON 格式输出：
```bash
cargo run -- identify resources/bingling_sama.png --json
```

## 帮助

使用 `--help` 查看最新的完整选项列表。

排查渲染或资源加载问题时，可以启用日志：

```bash
RUST_LOG=info cargo run -- render resources/bingling_sama.png
```

常见错误见 `troubleshooting_zh-CN.md`。
