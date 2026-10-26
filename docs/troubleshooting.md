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

## Output Looks Cropped Or Too Small

Adjust camera and viewport options:

```bash
cargo run -- render skin.png --cam-yaw 210 --cam-pitch 80 --cam-zoom 1.2 --width 1024 --height 1024
```

`--cam-zoom` must be greater than `0`. Larger values make the character appear closer.

## Skin Arms Look Wrong

Use the geometry that matches the skin:

- Default (classic) for 4-pixel-wide Steve arms.
- `--slim` for 3-pixel-wide Alex arms.
