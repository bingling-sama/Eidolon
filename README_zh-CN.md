# Eidolon

[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

[English](README.md)

Eidolon 是一个使用 Rust 编写的 Minecraft 皮肤渲染器。它可以把玩家皮肤渲染为 3D 模型图片，
也可以在窗口中实时预览。

## 功能

- 加载并渲染 Minecraft 玩家模型
- 支持皮肤贴图映射
- 使用 wgpu 进行 3D 渲染
- 输出 PNG 或 WebP 图片
- 可配置相机、姿势和输出尺寸
- 支持无窗口离屏渲染和窗口预览
- 跨平台支持 Windows、macOS、Linux

## 文档

完整文档位于 `docs/`。

- `docs/getting-started_zh-CN.md` 构建、首次渲染和预览流程
- `docs/cli_zh-CN.md` 完整命令行参考
- `docs/library_zh-CN.md` 库 API 使用方式
- `docs/architecture_zh-CN.md` 项目结构和渲染管线概览
- `docs/development_zh-CN.md` 本地开发、测试和基准测试
- `docs/troubleshooting_zh-CN.md` 常见运行与资源问题排查

## 快速开始

### 前置条件

- Rust（建议使用最新稳定版）
- 支持 Vulkan、Metal 或 DX12 的 GPU
- 可用的 `wgpu` 后端。无头环境可能需要配置 OSMesa 等软件后端，具体取决于平台。

### 构建

```bash
cargo build
```

### 渲染图片

```bash
cargo run -- render --skin-type classic
```

### 窗口预览

```bash
cargo run -- preview --skin-type classic
```

更多 CLI 选项见 `docs/cli_zh-CN.md`。

## 作为库使用

Eidolon 可以作为 Rust 库使用。最小示例：

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

更多说明见 `docs/library_zh-CN.md`。

## 贡献

欢迎贡献。请参考 `docs/contributing_zh-CN.md`。

## 许可证

见 `LICENSE`。

## 致谢

见 `docs/credits_zh-CN.md` 和 `docs/resources_zh-CN.md`。
