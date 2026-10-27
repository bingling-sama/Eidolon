# Troubleshooting

## Skin Geometry

The renderer defaults to classic (Steve, 4px) arms. Use `--slim` for Alex-style (3px) skins:

```bash
cargo run -- render skin.png
cargo run -- render skin.png --slim
cargo run -- preview skin.png --slim
```

The renderer cannot infer arm width from the PNG — you must specify it.

## Output Format

Format is inferred from the output filename extension:

```bash
cargo run -- render skin.png output.png     # Static PNG
cargo run -- render skin.png output.webp    # Static WebP
cargo run -- render skin.png walk.gif --animate --posture walk   # Animated GIF
cargo run -- render skin.png walk.webp --animate --posture walk  # Animated WebP
cargo run -- render skin.png walk.apng --animate --posture walk  # APNG
cargo run -- render skin.png walk.avif --animate --posture walk  # Animated AVIF
```

For static renders, PNG (`.png`) and WebP (`.webp`) are supported. For animated renders (using `--animate`), GIF (`.gif`), Animated WebP (`.webp`), APNG (`.apng`), and AVIF (`.avif`) are supported. The filename extension is normalized to match the detected format.

## Texture Fails To Load

Check that:

- The path is relative to the repository root, or is an absolute path.
- The file is a PNG.
- Legacy single-layer skins have `width == height * 2`.
- Double-layer skins are square.

The converter can expand a legacy skin explicitly:

```bash
cargo run -- convert resources/SSSSSteven.png converted.png
```

## Output Path Is Rejected

The `render` command rejects output paths containing `..` components:

```bash
# Rejected:
cargo run -- render skin.png ../output.png

# OK:
cargo run -- render skin.png output.png
cargo run -- render skin.png subdir/output.png
```

Choose a path inside the working directory or pass a direct filename.

## No GPU Adapter Or Window Creation Failure

Eidolon uses `wgpu`. The machine must expose a compatible Vulkan, Metal, DX12, or other supported
backend. On CI or servers, headless rendering can still fail if no adapter is available. A configured
software backend such as OSMesa may work on platforms where `wgpu` supports it.

Useful checks:

- Run the same command locally with `RUST_LOG=info` to see adapter and loading progress.
- On Linux, ensure graphics drivers and Vulkan loader packages are installed.
- For `preview`, confirm a desktop session or window server is available.

## Web (WASM) Browser Compatibility & Fallback

The Web `CanvasRenderer` prioritizes the **WebGPU** backend for maximum performance. If the host environment or browser lacks WebGPU support, it automatically falls back to **WebGL2**.

### Browser Compatibility Matrix

| Browser Platform | Minimum Version | Graphics Backend | Status | Notes |
| :--- | :---: | :---: | :---: | :--- |
| **Chrome / Edge (Desktop)** | 113+ | **WebGPU** | 🟢 Full Support | WebGPU enabled by default, optimal performance |
| **Chrome / Edge (Desktop)** | 79 ~ 112 | **WebGL2** | 🟢 Fallback | Automatically falls back to WebGL2 |
| **Firefox (Desktop)** | 115+ | **WebGL2** | 🟢 Full Support | Stable WebGL2 execution |
| **Firefox Nightly** | Latest | **WebGPU** | 🟡 Experimental | Requires enabling `dom.webgpu.enabled` in `about:config` |
| **Safari (macOS / iPadOS)** | 17.0+ | **WebGL2** | 🟢 Full Support | Stable WebGL2 (WebGPU remains preview/experimental) |
| **Safari (iOS)** | iOS 15.0+ | **WebGL2** | 🟢 Full Support | Stable WebGL2 execution on mobile |
| **Chrome / Firefox (Android)** | Android 10+ | **WebGPU** / **WebGL2** | 🟢 Full Support | Select devices run WebGPU; others fall back to WebGL2 |
| **IE11 / Legacy Edge (EdgeHTML)** | Any | None | 🔴 Unsupported | Lacks WebAssembly and WebGL2 support |

### WebGPU vs WebGL2 Performance Profile

| Metric | WebGPU Backend | WebGL2 Fallback | Notes |
| :--- | :---: | :---: | :--- |
| **Renderer Init Latency** | ~25 – 40 ms | ~35 – 160 ms | WebGPU starts ~3–4× faster |
| **First Frame Latency** | ~2.5 – 3.2 ms | ~22 – 35 ms | WebGL2 incurs pipeline shader compilation on first draw |
| **Animation Framerate** | 60 FPS (16.7 ms avg) | 60 FPS (16.7 ms avg) | Zero jank frames across both backends |
| **Capture 800×600 PNG** | ~6.7 ms (71.6 MP/s) | ~10.5 ms (45.7 MP/s) | Higher GPU readback throughput on WebGPU |
| **Capture 1080p PNG** | ~15.6 ms (133 MP/s) | ~16.4 ms (126 MP/s) | High throughput sustained across resolutions |
| **Memory Leak / Delta** | 0.00 MB | 0.00 MB | Zero heap growth across repeated offscreen captures |

## Output Looks Cropped Or Too Small

Adjust camera and viewport options:

```bash
cargo run -- render skin.png --cam-yaw 210 --cam-pitch 80 --cam-zoom 1.2 --width 1024 --height 1024
```

`--cam-zoom` must be greater than `0`. Larger values magnify by narrowing the field of view;
  the orbit radius stays fixed.

## Skin Arms Look Wrong

Use the geometry that matches the skin:

- Default (classic) for 4-pixel-wide Steve arms.
- `--slim` for 3-pixel-wide Alex arms.
