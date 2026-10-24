mod args;
mod commands;
mod preview;
mod scene;

use clap::Parser;

use crate::args::{Args, Command};
use crate::commands::{run_convert, run_identify, run_preview, run_render};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    match args.command {
        Command::Render {
            skin,
            output,
            viewport,
            scene,
            animation,
        } => run_render(skin, output, viewport, scene, animation),
        Command::Preview {
            skin,
            viewport,
            scene,
        } => run_preview(skin, viewport, scene),
        Command::Convert { input, output } => run_convert(input, output),
        Command::Identify { texture, json } => run_identify(texture, json),
    }
}
