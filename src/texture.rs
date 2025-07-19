//! 纹理模块
//!
//! 这个模块负责加载和处理 Minecraft 皮肤纹理。
//! 支持从 PNG 文件加载纹理，并创建 OpenGL 纹理对象。

use glium::backend::glutin::headless::Headless;
use glium::texture::{RawImage2d, Texture2d};
use image::ImageFormat;
use std::fs::File;
use std::io::BufReader;

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
