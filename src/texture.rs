//! 纹理模块
//!
//! 这个模块负责加载和处理 Minecraft 皮肤纹理。
//! 支持从 PNG 文件加载纹理，并创建 OpenGL 纹理对象。

use glium::backend::glutin::headless::Headless;
use glium::texture::{RawImage2d, Texture2d};
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat};
use std::fs::File;
use std::io::BufReader;
use crate::utils::converter::single2double_image;

/// 纹理结构体
///
/// 封装了 OpenGL 纹理对象，提供了纹理加载和管理功能。
/// 支持从文件加载 PNG 格式的纹理。
pub struct Texture {
    /// OpenGL 纹理对象
    pub texture: Texture2d,
}

impl Texture {
    /// 从文件加载纹理
    ///
    /// 从指定路径加载 PNG 格式的纹理文件，创建 OpenGL 纹理对象。
    /// 支持 RGBA 格式的图像，自动处理图像格式转换。
    ///
    /// # 参数
    ///
    /// * `display` - OpenGL 显示上下文
    /// * `path` - 纹理文件路径
    ///
    /// # 返回
    ///
    /// 成功时返回 `Texture` 实例，失败时返回错误信息
    ///
    /// # 错误
    ///
    /// - 文件不存在或无法读取
    /// - 图像格式不支持
    /// - OpenGL 纹理创建失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use skinviewer::texture::Texture;
    ///
    /// let texture = Texture::load_from_file(&display, "resources/player.png")?;
    /// ```
    pub fn load_from_file(
        display: &Headless,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Loading texture: {}", path);
        // 加载图像
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let image = image::load(reader, ImageFormat::Png)?.to_rgba8();
        match Self::load_texture(display, &DynamicImage::ImageRgba8(image)) {
            Ok(texture) => Ok(texture),
            Err(e) => Err(e),
        }
    }

    fn load_texture(
        display: &Headless,
        image: &DynamicImage,
    ) -> Result<Texture, Box<dyn std::error::Error>> {
        let (width, height) = image.dimensions();
        println!(
            "Texture dimensions: {}x{}",
            width, height
        );

        // 判断是否为单层皮肤（宽=高×2），如是则转换为双层
        let image = if width == height * 2 {
            println!("Single-layer skin detected, converting to double-layer...");
            match single2double_image(image) {
                Ok(img) => img,
                Err(e) => return Err(format!("Failed to convert single-layer to double-layer: {}", e).into()),
            }
        } else {
            image.clone()
        };

        let image_dimensions = image.dimensions();
        // 创建 OpenGL 纹理
        let image_rgba = image.to_rgba8();
        let image = RawImage2d::from_raw_rgba_reversed(image_rgba.as_raw(), image_dimensions);
        let texture = Texture2d::new(display, image)?;
        println!("Texture loaded into GPU");
        Ok(Texture { texture })
    }
}
