# Command Line Reference

Eidolon ships a single binary with three subcommands: `render`, `preview`, and `convert`.

## Render

Render a skin to an image file (headless).

```bash
eidolon render [OPTIONS] <SKIN> [OUTPUT]
```

**Positional arguments:**

| Arg | Description | Default |
|-----|-------------|---------|
| `<SKIN>` | Path to the skin PNG file | *(required)* |
| `[OUTPUT]` | Output image path. Extension determines format (`.png` or `.webp`) | `output.png` |

**Options:**

| Flag | Description | Default |
|------|-------------|---------|
| `--width <PX>` | Output width in pixels | `800` |
| `--height <PX>` | Output height in pixels | `600` |
| `--slim` | Use slim arm geometry (Alex-style, 3px arms) | *(classic, 4px)* |
| `--cam-yaw <DEG>` | Camera orbit yaw in degrees | `180` |
| `--cam-pitch <DEG>` | Camera orbit pitch in degrees | `90` |
| `--cam-zoom <FLOAT>` | Camera zoom; higher = closer (orbit radius: 4.0 / zoom). Must be > 0 | `1.0` |
| `--posture <PRESET>` | Posture preset: `stand`, `wave`, `walking`, `running` | `stand` |

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
# Minimal — classic arms, stand pose, 800×600 PNG
eidolon render skin.png

# Slim arms, WebP output
eidolon render skin.png out.webp --slim

# Wave pose, zoomed in, 1024×1024
eidolon render skin.png big.webp --posture wave --cam-zoom 1.5 --width 1024 --height 1024

# Custom camera angle
eidolon render skin.png --cam-yaw 210 --cam-pitch 80 --cam-zoom 1.2

# Override individual joints on top of a posture preset
eidolon render skin.png --posture walking --head-pitch 15 --left-arm-roll 30
```

Format is inferred from the output filename extension. `output.png` → PNG, `output.webp` → WebP.
The output path must not contain `..` components (directory traversal is rejected).

## Preview

Open a live preview window.

```bash
eidolon preview [OPTIONS] <SKIN>
```

Accepts the same scene, camera, posture, and viewport options as `render`, plus all power-user overrides. No output path or format argument.

### Examples

```bash
eidolon preview skin.png
eidolon preview skin.png --slim --cam-zoom 2.0
eidolon preview skin.png --posture running --width 1024 --height 768
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
