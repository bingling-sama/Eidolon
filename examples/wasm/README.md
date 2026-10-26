# Eidolon WASM Example

Minecraft skin renderer running in the browser via WebGPU + WASM.

## Prerequisites

- [Rust](https://rustup.rs) with `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (`cargo install wasm-pack`)
- [Node.js](https://nodejs.org) 18+
- Browser with WebGPU or WebGL2 support (Chrome, Edge, Firefox, Safari 15+)

## Quick Start

```bash
cd examples/wasm

# Build the core crate and its WASM bindings
npm run build:wasm

# Install JS dependencies
npm install

# Start dev server
npm run dev
```

Open the URL printed by Vite (usually `http://localhost:5173`), pick a Minecraft skin PNG, and optionally pick a Back equipment atlas for Cape or Elytra presentation. Choose an animation, play or pause it, adjust its speed and timeline, and toggle looping. Configure a player nametag and occlusion see-through mode. Elytra pitch and spread become fixed overrides when adjusted; **Reset Elytra** returns them to animation control.

After building the WASM package, run `npm run test:e2e` to verify desktop and mobile layout, playback, reduced-motion behavior, canvas updates, nametag rendering, and Elytra override reset in Chromium.

## How It Works

1. `wasm-pack` compiles the WASM binding crate (`crates/wasm`) to a WASM module + JS glue in `pkg/`
2. Vite serves the HTML/JS and bundles the WASM module via `vite-plugin-wasm`
3. JS creates a `CanvasRenderer`, loads the selected skin PNG, and optionally loads cape or elytra; the core `AnimationTimeline` owns playback state while JS supplies requestAnimationFrame deltas
4. `set_elytra_angles(pitch, spread)` fixes the wing angles until `reset_elytra_angles()` restores animation-driven values
5. The browser renders the model directly to a canvas in a requestAnimationFrame loop via WebGPU or WebGL2; reduced-motion preferences start playback paused and page visibility changes reset the frame-time baseline
6. Export captures an offscreen PNG at the selected resolution

## Files

| File | Purpose |
|------|---------|
| `../../crates/wasm/Cargo.toml` | WASM crate with `cdylib` output and npm configuration |
| `../../crates/wasm/src/lib.rs` | WASM entry point, logger and panic hook |
| `../../crates/wasm/src/renderer.rs` | `CanvasRenderer` WebGPU presenter and frame capture |
| `../../crates/wasm/src/classifier.rs` | `inspect_texture` JS binding returning `JsTextureReport` |
| `index.html` | UI — skin/back-equipment pickers, animation playback, export controls, result canvas |
| `main.js` | Vanilla JS — drives controls and handles DOM, camera orbit, and resource loading |
| `vite.config.js` | Vite config with WASM plugins |
| `package.json` | npm scripts + Vite deps |

## Performance Benchmark: WebGPU vs WebGL2

Eidolon WASM supports dual-backend benchmarking via Playwright (`pnpm test:bench`):

```bash
# Run both WebGPU and WebGL2 benchmarks
pnpm --filter eidolon-wasm-example test:bench

# Run only WebGL2 fallback benchmark
pnpm --filter eidolon-wasm-example test:bench --project=webgl2
```

### Representative Benchmark Results (Headless Chromium / SwiftShader)

| Benchmark Metric | WebGPU Backend | WebGL2 Fallback | Comparison / Notes |
|-------------------|----------------|-----------------|---------------------|
| **Renderer Init Latency** | ~25 – 40 ms | ~35 – 160 ms | WebGPU starts ~3–4× faster due to precompiled pipeline layout |
| **Texture Upload & Setup** | ~6 – 12 ms | ~13 – 30 ms | Fast on both backends |
| **First Frame Latency** | ~2.5 – 3.2 ms | ~22 – 35 ms | WebGL2 incurs pipeline shader compilation on first draw |
| **Animation Stability** | 60 FPS (16.7 ms avg) | 60 FPS (16.7 ms avg) | 0 jank frames (>33.3 ms) across 90 sampled frames |
| **Capture 800×600 PNG** | ~6.7 ms (71.6 MP/s) | ~10.5 ms (45.7 MP/s) | WebGPU provides higher GPU-to-CPU readback throughput |
| **Capture 800×600 WebP** | ~7.3 ms (65.8 MP/s) | ~8.3 ms (57.8 MP/s) | CPU encoding dominates overall duration |
| **Capture 1080p PNG** | ~15.6 ms (133 MP/s) | ~16.4 ms (126 MP/s) | Consistent high-resolution capture throughput |
| **Memory Delta (10x Exports)** | 0.00 MB | 0.00 MB | Zero heap growth across repeated offscreen captures |

