# 命令行参考

Eidolon 提供一个可执行程序，包含三个子命令：`render`、`preview`、`convert`。

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
- `--yaw <DEG>` 相机 yaw。默认：`180`
- `--pitch <DEG>` 相机 pitch。默认：`90`
- `--scale <FLOAT>` 相机距离缩放（必须 > 0）。默认：`1.0`
- `--posture <stand|wave|walking|running>` 姿势预设。默认：`stand`
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
```

如果使用 `--format webp` 且保持默认 `--filename output.png`，CLI 会把输出路径改为
`output.webp`。自定义文件名会按原样使用。

`--filename` 不能包含 `..` 路径片段，避免渲染输出通过目录穿越写到预期目录之外。

## Preview

打开实时预览窗口。

```bash
cargo run -- preview [OPTIONS]
```

`preview` 支持与 `render` 相同的场景、相机、姿势和视口选项，但不包含 `--filename` 与 `--format`。

示例：

```bash
cargo run -- preview --skin-type slim --texture resources/bingling_sama.png --yaw 200
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
