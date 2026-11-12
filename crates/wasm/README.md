# @eidolonmc/wasm

[![NPM Version](https://img.shields.io/npm/v/@eidolonmc/wasm.svg)](https://www.npmjs.com/package/@eidolonmc/wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

WebAssembly bindings for **Eidolon**, a Minecraft skin 3D renderer written in Rust.

`@eidolonmc/wasm` enables hardware-accelerated 3D Minecraft skin rendering directly in the browser via WebGPU and WebGL.

## Installation

```bash
npm install @eidolonmc/wasm
# or
pnpm add @eidolonmc/wasm
# or
yarn add @eidolonmc/wasm
```

## Features

- **In-Browser 3D Rendering**: Render directly onto an HTML `<canvas>` element using WebGPU or WebGL.
- **Classic & Slim Player Models**: Accurate Steve and Alex geometry with dual-layer overlay support.
- **Interactive Controls**: Camera orbit, zoom, and procedural animations (`idle`, `walk`, `run`, `wave`, `fly`).
- **Texture Analysis**: In-memory inspection and classification of Minecraft skin textures.
- **Body Visibility**: Independently toggle the inner and outer mesh of each body part.

## Quick Start

```typescript
import { CanvasRenderer, init_logger, inspect_texture } from '@eidolonmc/wasm';

async function main() {
  // Optional: Initialize logging to browser developer console
  init_logger();

  // Inspect skin texture metadata
  const skinBytes = await fetch('/skin.png').then((res) => res.arrayBuffer());
  const report = inspect_texture(new Uint8Array(skinBytes));
  console.log('Texture Type:', report.texture_type); // "SkinClassic" | "SkinSlim"
  console.log('Is Single Layer:', report.is_single_layer);

  // Initialize CanvasRenderer onto an existing HTML canvas
  const canvas = document.getElementById('skin-canvas') as HTMLCanvasElement;
  const renderer = await new CanvasRenderer(canvas);

  // Load character body
  await renderer.load_body(new Uint8Array(skinBytes), report.skin_type === 'Slim');

  // Set pose and camera
  renderer.set_posture('wave');
  renderer.set_camera(180, 85, 1.2); // yaw, pitch, zoom

  // Render a frame
  renderer.render();
}

main();
```

## Related Packages

- [`eidolon-core`](https://crates.io/crates/eidolon-core): Rust core rendering engine.
- [`eidolon-cli`](https://crates.io/crates/eidolon-cli): Desktop CLI executable.

## License

Licensed under the [MIT License](https://github.com/bingling-sama/eidolon/blob/main/LICENSE).
