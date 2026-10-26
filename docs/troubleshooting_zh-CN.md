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

## Web (WASM) 浏览器兼容性与回退

Web 端的 `CanvasRenderer` 优先检测并选用 **WebGPU** 后端；当运行环境或浏览器不支持 WebGPU 时，会自动无缝回退至 **WebGL2** 后端。

### 浏览器兼容性支持列表

| 浏览器平台 | 最低版本要求 | 运行渲染后端 | 支持状态 | 关键依赖 / 注意事项 |
| :--- | :---: | :---: | :---: | :--- |
| **Chrome / Edge (Desktop)** | 113+ | **WebGPU** | 🟢 完全支持 | 默认原生开启 WebGPU，性能最优 |
| **Chrome / Edge (Desktop)** | 79 ~ 112 | **WebGL2** | 🟢 兼容回退 | 自动降级至 WebGL2 |
| **Firefox (Desktop)** | 115+ | **WebGL2** | 🟢 完全支持 | 稳定运行 WebGL2 |
| **Firefox Nightly** | 最新版 | **WebGPU** | 🟡 实验支持 | 需在 `about:config` 中开启 `dom.webgpu.enabled` |
| **Safari (macOS / iPadOS)** | 17.0+ | **WebGL2** | 🟢 完全支持 | 稳定运行 WebGL2（WebGPU 仍在预览阶段） |
| **Safari (iOS)** | iOS 15.0+ | **WebGL2** | 🟢 完全支持 | 移动端良好兼容 WebGL2 |
| **Chrome / Firefox (Android)** | Android 10+ | **WebGPU** / **WebGL2** | 🟢 完全支持 | 部分支持 WebGPU，其余自动走 WebGL2 |
| **IE11 / 旧版 Edge (EdgeHTML)** | 任意版本 | 无 | 🔴 不支持 | 缺少 WebAssembly 及 WebGL2 支持 |

### WebGPU 与 WebGL2 性能基准对比

基于 Playwright 自动化双后端基准测试（运行环境为 Chromium / SwiftShader 渲染）：

| 性能指标 | WebGPU 后端 | WebGL2 回退后端 | 说明 |
| :--- | :---: | :---: | :--- |
| **渲染器初始化耗时** | ~25 – 40 ms | ~35 – 160 ms | WebGPU 管线初始化比 WebGL2 快 3–4 倍 |
| **首帧渲染呈现延迟** | ~2.5 – 3.2 ms | ~22 – 35 ms | WebGL2 首帧存在一次性的着色器编译与转换开销 |
| **动画帧率与稳定性** | 60 FPS (16.7 ms 均值) | 60 FPS (16.7 ms 均值) | 两者均达到满帧 60 FPS，连续 90 帧零卡顿（>33.3ms） |
| **800×600 PNG 离屏回读** | ~6.7 ms (71.6 MP/s) | ~10.5 ms (45.7 MP/s) | WebGPU 具备更高 GPU 到 CPU 的缓冲回读吞吐 |
| **1080p PNG 离屏回读** | ~15.6 ms (133 MP/s) | ~16.4 ms (126 MP/s) | 高分辨率下图像编码占比提高，吞吐保持稳定 |
| **连续导出内存增量 (10次)** | 0.00 MB | 0.00 MB | 连续离屏抓帧与编码无内存泄漏 |

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
