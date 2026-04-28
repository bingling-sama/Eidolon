# 开发指南

本指南记录修改 Eidolon 时常用的本地命令。

## 环境准备

安装最新稳定版 Rust toolchain，然后在仓库根目录构建：

```bash
cargo build
```

渲染器会从 `resources/` 加载模型和示例纹理，因此建议从仓库根目录执行命令，除非传入绝对路径。

## 常用命令

```bash
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test
cargo run -- render --skin-type classic
cargo run -- preview --skin-type classic
cargo bench
```

`cargo bench` 会把基准测试渲染输出写入 `.bench/`。

## 测试说明

当前测试主要覆盖 `src/utils/converter.rs` 中的皮肤图集转换逻辑。渲染路径会初始化真实的
`wgpu` adapter，因此依赖机器图形环境，更适合用 smoke test 验证：

```bash
cargo run -- render --skin-type classic --filename output.png
cargo run -- render --skin-type slim --texture resources/bingling_sama.png --format webp
```

修改渲染行为时，建议对比修改前后的输出图片。`resources/` 中的示例资源覆盖 classic、slim
几何，以及单层皮肤转换。

## 添加资源

皮肤文件：

- 使用 PNG。
- 使用正方形双层皮肤，例如 `64x64`；或旧版单层皮肤，即 `width == height * 2`，例如 `64x32`。
- 根据皮肤手臂宽度传入匹配的 `--skin-type`。

OBJ 模型：

- 保留 `architecture_zh-CN.md` 中列出的必需对象名。
- 保持 UV 与 Minecraft 皮肤图集坐标对齐。
- 确认内层和外层 mesh 都能正确渲染。
