# Command Line Reference

Eidolon ships a single binary with three subcommands: `render`, `preview`, and `convert`.

## Render

Render a skin to an image file (headless).

```bash
cargo run -- render [OPTIONS]
```

Options:

- `--filename <PATH>` Output file path. Default: `output.png`
- `--format <png|webp>` Output format. Default: `png`
- `--width <PX>` Output width in pixels. Default: `800`
- `--height <PX>` Output height in pixels. Default: `600`
- `--texture <PATH>` Skin PNG path. Default: `resources/bingling_sama.png`
- `--skin-type <classic|slim>` Required arm geometry
- `--yaw <DEG>` Camera yaw. Default: `180`
- `--pitch <DEG>` Camera pitch. Default: `90`
- `--scale <FLOAT>` Camera distance scale (must be > 0). Default: `1.0`
- `--posture <stand|wave|walking|running>` Posture preset. Default: `stand`
- `--head-yaw <DEG>` Override head yaw
- `--head-pitch <DEG>` Override head pitch
- `--left-arm-roll <DEG>` Override left arm roll
- `--left-arm-pitch <DEG>` Override left arm pitch
- `--right-arm-roll <DEG>` Override right arm roll
- `--right-arm-pitch <DEG>` Override right arm pitch
- `--left-leg-pitch <DEG>` Override left leg pitch
- `--right-leg-pitch <DEG>` Override right leg pitch
- `--position-x <FLOAT>` Character position X. Default: `0`
- `--position-y <FLOAT>` Character position Y. Default: `0`
- `--position-z <FLOAT>` Character position Z. Default: `0`
- `--rotation-x <DEG>` Character rotation X. Default: `0`
- `--rotation-y <DEG>` Character rotation Y. Default: `0`
- `--rotation-z <DEG>` Character rotation Z. Default: `0`

Example:

```bash
cargo run -- render --skin-type classic --texture resources/bingling_sama.png \
  --width 1024 --height 768 --yaw 210 --pitch 90 --scale 1.2 --format png
```

## Preview

Open a live preview window.

```bash
cargo run -- preview [OPTIONS]
```

The `preview` command accepts the same scene, camera, posture, and viewport options as `render`, except for `--filename` and `--format`.

Example:

```bash
cargo run -- preview --skin-type slim --texture resources/bingling_sama.png --yaw 200
```

## Convert

Convert a legacy single-layer skin atlas to a square double-layer atlas.

```bash
cargo run -- convert <INPUT> [OUTPUT]
```

Arguments:

- `<INPUT>` Input PNG (width must be twice the height)
- `[OUTPUT]` Output PNG path. Default: `output.png`

Example:

```bash
cargo run -- convert old_skin.png new_skin.png
```

## Help

Use `--help` on any command for the most up-to-date option list.
