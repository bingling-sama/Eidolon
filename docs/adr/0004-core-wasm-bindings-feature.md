# Core WASM bindings feature

Status: accepted

Eidolon will own its browser bindings in the core crate behind an explicit `wasm` Cargo feature. When targeting `wasm32` with that feature enabled, `src/wasm.rs` exposes the existing `CanvasRenderer` class through `wasm-bindgen`; the crate produces both `rlib` and `cdylib` outputs. The browser example keeps its HTML, JavaScript, CSS, and e2e tests, but no longer maintains a second Rust crate or bindings implementation.

## Considered options

- Keep `CanvasRenderer` in `examples/wasm` and let it depend on core. Rejected because the browser adapter then cannot share core timeline behavior without a second ownership seam, and the bindings can drift from the library.
- Expose wasm bindings on every target. Rejected because `wasm-bindgen` and browser types are optional integration concerns; native builds should not compile or expose them.
- Redesign the JS class while moving crates. Rejected for the migration commit; the existing class and methods remain stable so ownership migration is independently verifiable.

## Consequences

- Build the browser package with `wasm-pack build --target bundler --features wasm`.
- `examples/wasm` is a frontend/demo surface, not a second Rust package.
- The `wasm` feature remains explicit and only exposes `src::wasm` on `wasm32`.
