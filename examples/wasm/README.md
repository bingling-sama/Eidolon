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

# Build the Rust crate to WASM
wasm-pack build --target bundler --out-dir pkg

# Install JS dependencies
npm install

# Start dev server
npm run dev
```

Open the URL printed by Vite (usually `http://localhost:5173`), pick a Minecraft skin PNG, and optionally pick a cape or elytra texture. Elytra pitch and wing spread can be adjusted independently after choosing a posture.

## How It Works

1. `wasm-pack` compiles the Rust crate (`src/lib.rs`) to a WASM module + JS glue in `pkg/`
2. Vite serves the HTML/JS and bundles the WASM module via `vite-plugin-wasm`
3. JS creates a `CanvasRenderer`, loads the selected skin PNG, and optionally loads cape or elytra; `set_elytra_angles(pitch, spread)` overrides the selected posture, while `elytra_pitch()` and `elytra_spread()` expose the current values
4. The browser renders the model directly to a WebGPU canvas in a requestAnimationFrame loop
5. Export captures an offscreen PNG at the selected resolution

## Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust crate (cdylib) depending on `eidolon` with `wasm` feature |
| `src/lib.rs` | `#[wasm_bindgen]` bindings: `CanvasRenderer` class |
| `index.html` | UI — skin/back-equipment pickers, posture controls, export controls, result canvas |
| `main.js` | Vanilla JS — calls WASM, handles DOM, camera orbit, back-equipment loading |
| `vite.config.js` | Vite config with WASM plugins |
| `package.json` | npm scripts + Vite deps |
