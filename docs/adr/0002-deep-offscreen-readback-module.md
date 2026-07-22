# Deep offscreen readback module

Status: accepted

Eidolon will keep render-target creation and render-pass encoding in Renderer, while the private readback module owns the complete GPU-to-CPU transfer after rendering. Its `read_texture_to_rgba` interface receives a device, queue, completed render texture, and dimensions; it validates the layout, allocates the mapped buffer, submits the texture copy, waits for mapping through the platform adapter, removes row padding, and returns an RGBA image. Native keeps synchronous `Renderer::render`, while wasm keeps asynchronous `Renderer::render_async`; both use the same readback semantics and shared layout and unpadding implementation.

## Considered options

- Keep buffer allocation, copy encoding, queue submission, mapping, and unpadding split between Renderer and readback. Rejected because native and wasm Renderer paths must both understand and repeat the full readback protocol.
- Let readback create the render target or encode the render pass. Rejected because render-target format, pipelines, Character, and Camera belong to Renderer rather than the GPU-to-CPU readback seam.
- Keep render and copy commands in one command buffer. Rejected because it requires exposing a multi-step readback protocol to Renderer. The chosen interface uses a second queue submission so readback can own the copy command and buffer lifecycle end to end.
- Make all rendering asynchronous. Rejected because platform-specific waiting is internal and does not justify breaking the native `Renderer::render` interface.

## Consequences

- Offscreen rendering performs one render submission followed by one ordered readback-copy submission.
- The readback module remains private; library callers continue to use Renderer.
- Row layout and padding removal are pure internal implementation with GPU-independent tests.
- Readback dimensions and failures continue to use `EidolonError::Gpu`.
