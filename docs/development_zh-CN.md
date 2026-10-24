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
cargo clippy --workspace --all-targets --all-features
cargo test -p eidolon-core --lib -- --test-threads=1
cargo test -p eidolon-cli
cargo run -- render resources/bingling_sama.png
cargo run -- preview resources/bingling_sama.png
cargo bench -p eidolon-core
pnpm --dir examples/wasm run test:e2e
```

`cargo bench` 会把基准测试渲染输出写入 `.bench/`。

## 测试说明

- **核心库单元测试** (`crates/core`)：包含 107 个单元测试，全面覆盖相机矩阵计算、皮肤图集转换、纹理分析与格式分类、动画预设与时间轴播放、nametag 光栅化、模型加载与多格式输出编码。使用 `cargo test -p eidolon-core --lib -- --test-threads=1` 运行。
- **CLI 参数测试** (`crates/cli`)：包含 55 个针对命令行参数解析、默认值及子命令选项的自动化测试。使用 `cargo test -p eidolon-cli` 运行。
- **WASM 浏览器端 E2E 测试** (`examples/wasm`)：包含 13 个 Playwright 端到端测试，验证真实浏览器中 WebGPU 画布呈现、动画循环、响应式控件与多格式导出。使用 `pnpm --dir examples/wasm run test:e2e` 运行。

可以使用命令行进行手动 smoke test：

```bash
cargo run -- render resources/bingling_sama.png
cargo run -- render resources/bingling_sama.png out.webp --slim --posture wave
cargo run -- identify resources/bingling_sama.png
```

修改渲染行为时，建议对比修改前后的输出图片。`resources/` 中的示例资源覆盖 classic、slim 几何，以及单层皮肤转换。

## 添加资源

皮肤文件：

- 使用 PNG。
- 使用正方形双层皮肤，例如 `64x64`；或旧版单层皮肤，即 `width == height * 2`，例如 `64x32`。
- 纤细手臂（Alex 风格，3px）添加 `--slim` 参数，经典手臂（Steve 风格，4px）省略即可。

OBJ 模型：

- 保留 `architecture_zh-CN.md` 中列出的必需对象名。
- 保持 UV 与 Minecraft 皮肤图集坐标对齐。
- 确认内层和外层 mesh 都能正确渲染。
