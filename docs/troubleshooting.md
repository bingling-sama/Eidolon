# Troubleshooting

## `--skin-type` Is Required

Both `render` and `preview` require a skin geometry:

```bash
cargo run -- render --skin-type classic
cargo run -- preview --skin-type slim
```

Use `classic` for wide Steve-style arms and `slim` for Alex-style arms.

## Unsupported Output Format

`render` supports only PNG and WebP:

```bash
cargo run -- render --skin-type classic --format png
cargo run -- render --skin-type classic --format webp
```

If `--format webp` is used with the default `--filename output.png`, the CLI writes `output.webp`.
If you provide a custom filename, make sure the extension matches the format you want.

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

The `render` command rejects `--filename` values containing `..` path components:

```bash
cargo run -- render --skin-type classic --filename ../output.png
```

Choose a path inside the working directory or pass a direct filename such as `output.png`.

## No GPU Adapter Or Window Creation Failure

Eidolon uses `wgpu`. The machine must expose a compatible Vulkan, Metal, DX12, or other supported
backend. On CI or servers, headless rendering can still fail if no adapter is available. A configured
software backend such as OSMesa may work on platforms where `wgpu` supports it.

Useful checks:

- Run the same command locally with `RUST_LOG=info` to see adapter and loading progress.
- On Linux, ensure graphics drivers and Vulkan loader packages are installed.
- For `preview`, confirm a desktop session or window server is available.

## Output Looks Cropped Or Too Small

Adjust camera and viewport options:

```bash
cargo run -- render --skin-type classic --width 1024 --height 1024 --yaw 210 --pitch 90 --scale 1.2
```

`--scale` must be greater than `0`. Larger values make the character appear closer.

## Skin Arms Look Wrong

Use the geometry that matches the skin:

- `--skin-type classic` for 4-pixel-wide arms.
- `--skin-type slim` for 3-pixel-wide arms.

The renderer cannot infer this from the PNG.
