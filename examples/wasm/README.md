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

Open the URL printed by Vite (usually `http://localhost:5173`), pick a Minecraft skin PNG, and click **Render**.

## How It Works

1. `wasm-pack` compiles the Rust crate (`src/lib.rs`) to a WASM module + JS glue in `pkg/`
2. Vite serves the HTML/JS and bundles the WASM module via `vite-plugin-wasm`
3. JS calls `SkinRenderer.render(skinBytes)` which:
   - Uploads the skin PNG to WebGPU as a texture
   - Rigs it onto the 3D player model
   - Renders an 800×600 offscreen frame
   - Encodes the result as PNG and returns the bytes
4. JS wraps the PNG bytes in a `Blob` and displays it via `<img>`

## Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust crate (cdylib) depending on `eidolon` with `wasm` feature |
| `src/lib.rs` | `#[wasm_bindgen]` bindings: `SkinRenderer` class |
| `index.html` | UI — file picker, render button, result display |
| `main.js` | Vanilla JS — calls WASM, handles DOM |
| `vite.config.js` | Vite config with WASM plugins |
| `package.json` | npm scripts + Vite deps |
