# Eidolon WASM 示例

基于 WebGPU + WASM 在浏览器中运行的 Minecraft 皮肤渲染器。

## 环境要求

- [Rust](https://rustup.rs) 及 `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (`cargo install wasm-pack`)
- [Node.js](https://nodejs.org) 18+
- 支持 WebGPU 或 WebGL2 的现代浏览器（Chrome、Edge、Firefox、Safari 15+）

## 快速上手

```bash
cd examples/wasm

# 构建核心 crate 及其 WASM 绑定
npm run build:wasm

# 安装 JS 依赖
npm install

# 启动本地开发服务
npm run dev
```

打开 Vite 输出的本地地址（通常为 `http://localhost:5173`），选择 Minecraft 皮肤 PNG，并可选择背部装备贴图（披风或鞘翅）。可选择内置动画、播放/暂停、调节速度与时间轴并切换循环。支持配置玩家头顶名牌与半透明透视模式。鞘翅俯仰角度与展开弧度在调节时会锁定为固定覆盖值，点击 **Reset Elytra** 可恢复为动画控制。

构建 WASM 包后，运行 `npm run test:e2e` 可在 Chromium 中验证桌面端/移动端布局、动画播放、减弱动态偏好（reduced-motion）、Canvas 更新、名牌渲染以及鞘翅覆盖重置。

## 工作原理

1. `wasm-pack` 将 WASM 绑定 crate (`crates/wasm`) 编译为 WASM 模块并在 `pkg/` 生成 JS 胶水代码。
2. Vite 托管 HTML/JS 并通过 `vite-plugin-wasm` 打包 WASM 模块。
3. JS 创建 `CanvasRenderer`，加载所选皮肤 PNG（以及可选的披风或鞘翅贴图）；核心库 `AnimationTimeline` 管理播放状态，JS 提供 requestAnimationFrame 帧间隔增量。
4. `set_elytra_angles(pitch, spread)` 锁定翅膀角度，直到调用 `reset_elytra_angles()` 恢复由动画驱动。
5. 浏览器通过 WebGPU 或 WebGL2 在 requestAnimationFrame 循环中直接渲染至 Canvas；减弱动态偏好默认以暂停状态启动播放，页面可见性变更会重置基准帧时间。
6. 导出功能可在指定分辨率下抓取离屏 PNG。

## 文件结构

| 文件 | 作用 |
|------|------|
| `../../crates/wasm/Cargo.toml` | 带有 `cdylib` 输出及 npm 配置的 WASM crate |
| `../../crates/wasm/src/lib.rs` | WASM 入口、日志与 panic hook |
| `../../crates/wasm/src/renderer.rs` | `CanvasRenderer` WebGPU 画布呈现器与帧捕获 |
| `../../crates/wasm/src/classifier.rs` | `inspect_texture` JS 绑定，返回 `JsTextureReport` |
| `index.html` | 前端界面 — 皮肤/装备选择器、动画播放、导出控制与结果画布 |
| `main.js` | 原生 JavaScript — 驱动交互控制、处理 DOM、相机轨道旋转与资源加载 |
| `vite.config.js` | 集成 WASM 插件的 Vite 配置 |
