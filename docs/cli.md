# Command Line Reference

Eidolon ships a single binary with three subcommands: `render`, `preview`, and `convert`.

## Render

Render a skin to a static image or animation file (headless).

```bash
eidolon render [OPTIONS] <SKIN> [OUTPUT]
```

**Positional arguments:**

| Arg | Description | Default |
|-----|-------------|---------|
| `<SKIN>` | Path to the skin PNG file | *(required)* |
| `[OUTPUT]` | Output path. Static: `.png`/`.webp`; animated: `.gif`/`.webp`/`.apng`/`.avif` | `output.png`, or `output.webp` with `--animate` |

**Options:**

| Flag | Description | Default |
|------|-------------|---------|
| `--width <PX>` | Output width in pixels | `800` |
| `--height <PX>` | Output height in pixels | `600` |
| `--animate` | Render a complete animation cycle instead of one frame | *(disabled)* |
| `--fps <1..60>` | Animation frame rate; requires `--animate` | `30` |
| `--speed <0.25..4.0>` | Animation playback speed; requires `--animate` | `1.0` |
| `--loop-count <N>` | Animation loop metadata; `0` means infinite; requires `--animate` | `0` |
| `--slim` | Use slim arm geometry (Alex-style, 3px arms) | *(classic, 4px)* |
| `--nametag <TEXT>` | Render a Minecraft-style player name above the model | *(hidden)* |
| `--disable-nametag-see-through` | Fully hide nametag pixels occluded by scene geometry | *(translucent when occluded)* |
| `--cape <PNG>` | Render a Cape using the specified Back equipment atlas | *(no cape)* |
| `--elytra <PNG>` | Render elytra using a 2:1 cape atlas (64x32 or HD) | *(no elytra)* |
| `--cape-pitch <DEG>` | Override the selected posture's cape angle | *(posture value)* |
| `--elytra-pitch <DEG>` | Override the selected posture's elytra pitch | *(posture value)* |
| `--elytra-spread <DEG>` | Override the selected posture's elytra opening angle | *(posture value)* |
| `--cam-yaw <DEG>` | Camera orbit yaw in degrees | `180` |
| `--cam-pitch <DEG>` | Camera orbit pitch in degrees | `90` |
| `--cam-zoom <FLOAT>` | Camera zoom; higher = closer (orbit radius: 4.0 / zoom). Must be > 0 | `1.0` |
| `--posture <ACTION>` | Action: `idle`, `walk`, `run`, `wave`, `fly` | `idle` |

**Power-user options** (show in `--help` but not `-h`):

| Flag | Description |
|------|-------------|
| `--head-yaw <DEG>` | Override head yaw (0° = forward) |
| `--head-pitch <DEG>` | Override head pitch (0° = level) |
| `--left-arm-roll <DEG>` | Override left arm roll |
| `--left-arm-pitch <DEG>` | Override left arm pitch |
| `--right-arm-roll <DEG>` | Override right arm roll |
| `--right-arm-pitch <DEG>` | Override right arm pitch |
| `--left-leg-pitch <DEG>` | Override left leg pitch (0° = straight down) |
| `--right-leg-pitch <DEG>` | Override right leg pitch (0° = straight down) |
| `--pos-x <FLOAT>` | Character position X | `0` |
| `--pos-y <FLOAT>` | Character position Y | `0` |
| `--pos-z <FLOAT>` | Character position Z | `0` |
| `--rot-x <DEG>` | Character rotation X | `0` |
| `--rot-y <DEG>` | Character rotation Y | `0` |
| `--rot-z <DEG>` | Character rotation Z | `0` |

### Examples

```bash
# Minimal — classic arms, idle pose, 800×600 PNG
eidolon render skin.png

# Slim arms, WebP output
eidolon render skin.png out.webp --slim

# Minecraft-style nametag
eidolon render skin.png --nametag Steve

# Fully hide nametag pixels behind the player or scene geometry
eidolon render skin.png --nametag Steve --disable-nametag-see-through

# Render Cape with an explicit Back equipment atlas
eidolon render skin.png --cape cape.png --cape-pitch 25

# Render elytra with custom pitch and opening angle (mutually exclusive with --cape)
eidolon render skin.png --elytra cape.png --elytra-pitch 18 --elytra-spread 45

# Wave pose, zoomed in, 1024×1024
eidolon render skin.png big.webp --posture wave --cam-zoom 1.5 --width 1024 --height 1024

# Custom camera angle
eidolon render skin.png --cam-yaw 210 --cam-pitch 80 --cam-zoom 1.2

# Override individual joints on top of a posture preset
eidolon render skin.png --posture walk --head-pitch 15 --left-arm-roll 30

# Animated exports (GIF, Animated WebP, APNG, AVIF)
eidolon render skin.png walk.gif --animate --posture walk
eidolon render skin.png fly.webp --animate --posture fly --elytra cape.png --fps 60 --speed 1.5
eidolon render skin.png wave.gif --animate --posture wave --loop-count 3 --width 512 --height 512
eidolon render skin.png run.apng --animate --posture run
eidolon render skin.png dance.avif --animate --posture dance
```

Format is inferred from the output filename extension. Static output supports PNG (`.png`) and WebP (`.webp`). Animated output supports GIF (`.gif`), WebP (`.webp`), APNG (`.apng`), and AVIF (`.avif`).
The output path must not contain `..` components (directory traversal is rejected).

`--posture` uses exactly the `AnimationPreset` names and is shared by static rendering, animated
rendering, and preview. Static rendering samples a representative frame; Walk and Run use the
quarter-cycle frame, while the other actions use phase zero. With `--animate`, explicit joint,
Cape, and Elytra overrides are applied after every sample. Animated WebP preserves full alpha and
uses lossless encoding; APNG preserves 8-bit RGBA frames losslessly; AVIF encodes animated AV1 frames with alpha; GIF uses a 256-color RGBA quantization path. Animated output dimensions
must not exceed 2048×2048.

## Preview

Open a live preview window.

```bash
eidolon preview [OPTIONS] <SKIN>
```

Accepts the same scene, camera, posture, and viewport options as `render`, plus all power-user overrides. No output path or format argument. The selected posture is shown at its representative frame with playback paused; `Space` starts the matching animation from that same frame.

In the preview window, drag with the left mouse button to pan, drag with the right mouse button to rotate, and use the mouse wheel to zoom.
Use `Space` to play or pause, Left/Right to seek by one nominal 30fps frame, `1`–`5` to select
Idle/Walk/Run/Wave/Fly, and `-`/`+` to adjust playback speed. Switching presets resets phase to zero.

### Examples

```bash
eidolon preview skin.png
eidolon preview skin.png --slim --cam-zoom 2.0
eidolon preview skin.png --nametag Steve
eidolon preview skin.png --cape cape.png
eidolon preview skin.png --elytra cape.png
eidolon preview skin.png --posture run --width 1024 --height 768
eidolon preview skin.png --posture walk
```

## Convert

Convert a legacy single-layer skin atlas (`width == height × 2`) to a square double-layer atlas.

```bash
eidolon convert <INPUT> [OUTPUT]
```

| Arg | Description | Default |
|-----|-------------|---------|
| `<INPUT>` | Input PNG (width must be twice the height) | *(required)* |
| `[OUTPUT]` | Output PNG path | `output.png` |

### Example

```bash
eidolon convert old_skin.png new_skin.png
```

## Help

Use `-h` for a concise option summary or `--help` for the full list including power-user overrides:

```bash
eidolon render -h
eidolon render --help
```

Enable logs with `RUST_LOG=info` when diagnosing rendering or asset-loading problems:

```bash
RUST_LOG=info eidolon render skin.png
```

See `troubleshooting.md` for common errors.
