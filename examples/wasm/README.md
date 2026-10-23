# Eidolon WASM Example

Minecraft skin renderer running in the browser via WebGPU + WASM.

## Prerequisites

- [Rust](https://rustup.rs) with `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (`cargo install wasm-pack`)
- [Node.js](https://nodejs.org) 18+
- Browser with WebGPU support (Chrome 113+, Edge 113+)

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

1. `wasm-pack` compiles the core crate (`../../src/wasm.rs`, enabled by the `wasm` feature) to a WASM module + JS glue in `pkg/`
2. Vite serves the HTML/JS and bundles the WASM module via `vite-plugin-wasm`
3. JS creates a `CanvasRenderer`, loads the selected skin PNG, and optionally loads cape or elytra; the core `AnimationTimeline` owns playback state while JS supplies requestAnimationFrame deltas
4. `set_elytra_angles(pitch, spread)` fixes the wing angles until `reset_elytra_angles()` restores animation-driven values
5. The browser renders the model directly to a WebGPU canvas in a requestAnimationFrame loop; reduced-motion preferences start playback paused and page visibility changes reset the frame-time baseline
6. Export captures an offscreen PNG at the selected resolution

## Files

| File | Purpose |
|------|---------|
| `../../Cargo.toml` | Core crate with the opt-in `wasm` feature and `cdylib` output |
| `../../src/wasm.rs` | `#[wasm_bindgen]` bindings: `CanvasRenderer` class |
| `index.html` | UI — skin/back-equipment pickers, animation playback, export controls, result canvas |
| `main.js` | Vanilla JS — drives controls and handles DOM, camera orbit, and resource loading |
| `vite.config.js` | Vite config with WASM plugins |
| `package.json` | npm scripts + Vite deps |
