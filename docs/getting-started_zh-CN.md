# 快速开始

本指南帮助你构建 Eidolon、渲染第一张图片，并在窗口中预览皮肤。

## 前置条件

- Rust（建议使用最新稳定版）
- 支持 Vulkan、Metal 或 DX12 的 GPU
- 可用的 `wgpu` 后端。无窗口渲染同样需要 adapter；部分平台可使用配置好的 OSMesa 等软件后端。

## 构建

```bash
cargo build
```

## 渲染图片（无窗口）

必须指定 `--skin-type`（`classic` 或 `slim`）。默认纹理为 `resources/bingling_sama.png`。

```bash
cargo run -- render --skin-type classic
```

默认输出为 `output.png`。如需 WebP：

```bash
cargo run -- render --skin-type classic --format webp
```

如果同时使用 `--filename output.png` 与 `--format webp`，输出文件名会自动调整为 `output.webp`。

渲染自定义皮肤和相机角度：

```bash
cargo run -- render \
  --skin-type slim \
  --texture resources/bingling_sama.png \
  --filename preview.png \
  --width 1024 \
  --height 1024 \
  --yaw 210 \
  --pitch 90 \
  --scale 1.2
```

## 窗口预览

```bash
cargo run -- preview --skin-type classic
```

预览窗口支持与 `render` 相同的场景、相机、姿势和视口参数。

## 转换旧版皮肤

旧版单层皮肤（例如 `64x32`）可以转换为正方形双层图集：

```bash
cargo run -- convert resources/SSSSSteven.png converted.png
```

## 说明

- 单层皮肤（宽度 = 高度 × 2）在加载时会自动转换为双层。
- 使用 `cargo run -- render --help` 和 `cargo run -- preview --help` 查看完整选项列表。
- 如果渲染器找不到 GPU adapter 或皮肤路径加载失败，请参考 `troubleshooting_zh-CN.md`。
