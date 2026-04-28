# 常见问题排查

## `--skin-type` 是必填项

`render` 和 `preview` 都需要指定皮肤几何：

```bash
cargo run -- render --skin-type classic
cargo run -- preview --skin-type slim
```

`classic` 用于 Steve 风格的宽手臂，`slim` 用于 Alex 风格的窄手臂。

## 输出格式不支持

`render` 仅支持 PNG 和 WebP：

```bash
cargo run -- render --skin-type classic --format png
cargo run -- render --skin-type classic --format webp
```

如果使用 `--format webp` 且保持默认 `--filename output.png`，CLI 会写入 `output.webp`。如果
传入自定义文件名，请自行确保扩展名和期望格式一致。

## 纹理加载失败

请检查：

- 路径是相对仓库根目录的路径，或是绝对路径。
- 文件格式为 PNG。
- 旧版单层皮肤满足 `width == height * 2`。
- 双层皮肤是正方形。

可以显式转换旧版皮肤：

```bash
cargo run -- convert resources/SSSSSteven.png converted.png
```

## 输出路径被拒绝

`render` 命令会拒绝包含 `..` 路径片段的 `--filename`：

```bash
cargo run -- render --skin-type classic --filename ../output.png
```

请改用工作目录内的路径，或直接传入 `output.png` 这样的文件名。

## 找不到 GPU Adapter 或窗口创建失败

Eidolon 使用 `wgpu`。机器需要提供兼容的 Vulkan、Metal、DX12 或其他受支持后端。在 CI 或服务器
上，即使是无窗口渲染，也可能因为没有 adapter 而失败。部分平台可使用配置好的 OSMesa 等软件后端。

可先检查：

- 使用 `RUST_LOG=info` 在本地运行同一命令，查看 adapter 和资源加载进度。
- Linux 环境中确认已安装显卡驱动和 Vulkan loader。
- 使用 `preview` 时确认存在桌面会话或窗口服务。

## 输出被裁切或角色太小

调整相机和视口参数：

```bash
cargo run -- render --skin-type classic --width 1024 --height 1024 --yaw 210 --pitch 90 --scale 1.2
```

`--scale` 必须大于 `0`。数值越大，角色看起来越近。

## 手臂贴图看起来不对

使用与皮肤匹配的几何类型：

- `--skin-type classic` 对应 4 像素宽手臂。
- `--skin-type slim` 对应 3 像素宽手臂。

渲染器无法从 PNG 自动推断手臂类型。
