# Eidolon

[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

Eidolon is a Minecraft skin renderer written in Rust. It can render a 3D model of a player skin and save it as a PNG image.

## Features

- Load and render Minecraft player models
- Texture mapping support
- 3D rendering
- Save rendering result as a PNG image
- Configurable camera and output size

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- **Note:** This project relies on `osmesa`, which is included in `mesa` versions prior to 25.10. Please ensure you have a compatible version of `mesa` installed.

## Build and Run

1.  Clone the repository:
    ```bash
    git clone https://github.com/bingling-sama/SkinViewer.git
    cd SkinViewer
    ```

2.  Run the project:

    To render and save the image directly:
    ```bash
    cargo run -- [OPTIONS] [FILENAME]
    ```

    **Arguments:**
    - `[FILENAME]` (optional): Output image filename. Defaults to `output.png`.

    **Options:**
    - `--width <WIDTH>`: Output image width. Defaults to `800`.
    - `--height <HEIGHT>`: Output image height. Defaults to `600`.
    - `--texture <TEXTURE>`: Path to the PNG texture file. Defaults to `resources/player.png`.
    - `--yaw <YAW>`: Camera yaw. Defaults to `20.0`.
    - `--pitch <PITCH>`: Camera pitch. Defaults to `20.0`.
    - `--scale <SCALE>`: Camera scale. Defaults to `1.0`.

    **Examples:**
    ```bash
    # Save with default settings
    cargo run

    # Specify output filename
    cargo run -- my_skin.png

    # Save with custom size and camera settings
    cargo run -- my_skin.png --width 1024 --height 768 --yaw 30 --pitch -15 --scale 1.2
    ```

## Dependencies

This project uses the following main crates:

- `glium`: For OpenGL rendering.
- `glutin`: For windowing and input.
- `image`: For image processing.
- `cgmath`: For 3D math.
- `tobj`: For loading `.obj` files.
- `clap`: For command-line argument parsing.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.


## Thanks

- Thanks to [@tnqzh123](https://github.com/tnqzh123) for project feature design.
- Thanks to [@beanflame](https://github.com/beanflame) for opengl technical support.
- Thanks to [@sunjunnan79](https://github.com/sunjunnan79) for deleting `.DS_Store`. (Seriously)


