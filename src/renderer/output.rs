use image::ImageFormat;

/// 输出图片格式
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// PNG 格式
    Png,
    /// WebP 格式
    WebP,
}

impl OutputFormat {
    pub fn as_image_format(&self) -> ImageFormat {
        match self {
            OutputFormat::Png => ImageFormat::Png,
            OutputFormat::WebP => ImageFormat::WebP,
        }
    }
}
