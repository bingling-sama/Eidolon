# 快速开始

本指南帮助你构建 Eidolon、渲染第一张图片，并在窗口中预览皮肤。

## 前置条件

- Rust（建议使用最新稳定版）
- 支持 Vulkan、Metal 或 DX12 的 GPU
- 或安装可用版本的 libOSMesa（用于无 GPU 的无头环境）

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

## 窗口预览

```bash
cargo run -- preview --skin-type classic
```

## 说明

- 单层皮肤（宽度 = 高度 × 2）在加载时会自动转换为双层。
- 使用 `cargo run -- render --help` 和 `cargo run -- preview --help` 查看完整选项列表。
