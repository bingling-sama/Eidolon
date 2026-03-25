use image::ImageFormat;

/// Image format for [`crate::renderer::Renderer::render_to_image`].
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Png,
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
