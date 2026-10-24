# 常见问题排查

## 皮肤几何与手臂宽度

渲染器默认使用 classic（Steve 风格，4 像素宽手臂）。如果使用 Alex 风格（3 像素宽手臂），请传入 `--slim`：

```bash
cargo run -- render skin.png
cargo run -- render skin.png --slim
cargo run -- preview skin.png --slim
```

渲染器无法从 PNG 文件自动推断手臂宽度，需要使用者根据皮肤类型指定。

## 输出格式

输出格式由输出文件扩展名自动推断：

```bash
cargo run -- render skin.png output.png     # 静态 PNG
cargo run -- render skin.png output.webp    # 静态 WebP
cargo run -- render skin.png walk.gif --animate --posture walk   # 动画 GIF
cargo run -- render skin.png walk.webp --animate --posture walk  # 动态 WebP
cargo run -- render skin.png walk.apng --animate --posture walk  # APNG
cargo run -- render skin.png walk.avif --animate --posture walk  # 动画 AVIF
```

静态渲染支持 PNG (`.png`) 与 WebP (`.webp`)。添加 `--animate` 进行动画导出时，支持 GIF (`.gif`)、动态 WebP (`.webp`)、APNG (`.apng`) 与 AVIF (`.avif`)。扩展名会自动规范化为推断出的格式。

## 纹理加载失败

请检查：

- 路径是否相对当前运行目录，或为绝对路径。
- 文件是否为有效 PNG 图片。
- 旧版单层皮肤需满足 `width == height * 2`。
- 双层皮肤需为正方形。

可以使用 `identify` 命令提前检测文件类型：

```bash
cargo run -- identify resources/bingling_sama.png
```

也可以使用 `convert` 命令显式转换旧版单层皮肤：

```bash
cargo run -- convert resources/SSSSSteven.png converted.png
```

## 输出路径被拒绝

`render` 命令出于安全考虑会拒绝包含 `..` 路径片段的输出路径：

```bash
# 会被拒绝：
cargo run -- render skin.png ../output.png

# 正确用法：
cargo run -- render skin.png output.png
cargo run -- render skin.png subdir/output.png
```

请使用当前目录或子目录内的路径，或直接传入目标文件名。

## 找不到 GPU Adapter 或窗口创建失败

Eidolon 基于 `wgpu` 构建。机器必须提供兼容的 Vulkan、Metal、DX12 或其他受支持后端。在 CI 或无桌面服务器上，即使是无窗口离屏渲染，也需要可用的 GPU adapter。部分平台可以使用配置好的 OSMesa 或 llvmpipe 软件后端。

排查建议：

- 运行命令时加上环境变量 `RUST_LOG=info`，查看图形 adapter 协商与初始化日志。
- Linux 环境下确认已安装显卡驱动与 Vulkan loader 运行时（如 `libvulkan1`、`mesa-vulkan-drivers`）。
- 使用 `preview` 时确认存在桌面会话或活跃的显示服务（X11/Wayland）。

## 输出被裁切或角色太小

可调整视口大小与相机参数：

```bash
cargo run -- render skin.png preview.png \
  --width 1024 \
  --height 1024 \
  --cam-yaw 210 \
  --cam-pitch 90 \
  --cam-zoom 1.2
```

`--cam-zoom` 必须大于 `0`。数值越大，相机距离角色越近（相机轨道半径公式为 `4.0 / zoom`）。
