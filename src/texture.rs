use glium::glutin::surface::WindowSurface;
use glium::texture::{RawImage2d, Texture2d};
use glium::Display;
use image::ImageFormat;
use std::fs::File;
use std::io::BufReader;

pub struct Texture {
    pub texture: Texture2d,
}

impl Texture {
    pub fn load_from_file(
        display: &Display<WindowSurface>,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Loading texture: {}", path);

        // 加载图像
        let image = image::load(BufReader::new(File::open(path)?), ImageFormat::Png)?.to_rgba8();

        let image_dimensions = image.dimensions();
        println!(
            "Texture dimensions: {}x{}",
            image_dimensions.0, image_dimensions.1
        );

        // 创建 OpenGL 纹理
        let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        let texture = Texture2d::new(display, image)?;
        println!("Texture loaded into GPU");

        Ok(Texture { texture })
    }
}
