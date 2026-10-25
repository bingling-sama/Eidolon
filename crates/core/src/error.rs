//! Typed error for the public API.

use std::fmt;

/// All errors the library can surface.
#[derive(Debug)]
pub enum EidolonError {
    /// Wrapped I/O error (file not found, permission denied, etc.).
    Io(std::io::Error),
    /// Wrapped image decoding/processing error.
    Image(image::ImageError),
    /// GPU adapter, device, or pipeline failure.
    Gpu(String),
    /// OBJ model missing required named objects.
    Model(String),
    /// Skin texture load or conversion failure.
    Texture(String),
    /// Single→double layer conversion failure.
    Conversion(String),
    /// Path contains null bytes or is otherwise invalid.
    InvalidPath(String),
    /// Output path, format, encoding, or write failure.
    Output(String),
    /// Window surface acquisition or presentation failure.
    Surface(SurfaceError),
}

/// Window surface acquisition or presentation failure kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceError {
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
    OutOfMemory,
    Other,
}

impl fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "Surface timed out"),
            Self::Occluded => write!(f, "Surface is occluded"),
            Self::Outdated => write!(f, "Surface is outdated"),
            Self::Lost => write!(f, "Surface was lost"),
            Self::Validation => write!(f, "Surface validation error"),
            Self::OutOfMemory => write!(f, "Surface out of memory"),
            Self::Other => write!(f, "Unknown surface error"),
        }
    }
}

impl std::error::Error for SurfaceError {}

impl fmt::Display for EidolonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Image(e) => write!(f, "Image error: {e}"),
            Self::Gpu(msg) => write!(f, "GPU error: {msg}"),
            Self::Model(msg) => write!(f, "Model error: {msg}"),
            Self::Texture(msg) => write!(f, "Texture error: {msg}"),
            Self::Conversion(msg) => write!(f, "Conversion error: {msg}"),
            Self::InvalidPath(msg) => write!(f, "Invalid path: {msg}"),
            Self::Output(msg) => write!(f, "Output error: {msg}"),
            Self::Surface(error) => write!(f, "Surface error: {error}"),
        }
    }
}

impl std::error::Error for EidolonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Image(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EidolonError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<image::ImageError> for EidolonError {
    fn from(e: image::ImageError) -> Self {
        Self::Image(e)
    }
}

impl From<SurfaceError> for EidolonError {
    fn from(error: SurfaceError) -> Self {
        Self::Surface(error)
    }
}

// wgpu errors carry a string description — convert explicitly, not via From
impl EidolonError {
    pub fn gpu(msg: impl Into<String>) -> Self {
        Self::Gpu(msg.into())
    }

    pub fn model(msg: impl Into<String>) -> Self {
        Self::Model(msg.into())
    }

    pub fn texture(msg: impl Into<String>) -> Self {
        Self::Texture(msg.into())
    }

    pub fn conversion(msg: impl Into<String>) -> Self {
        Self::Conversion(msg.into())
    }

    pub fn invalid_path(msg: impl Into<String>) -> Self {
        Self::InvalidPath(msg.into())
    }

    pub fn output(msg: impl Into<String>) -> Self {
        Self::Output(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_constructor() {
        let e = EidolonError::gpu("adapter not found");
        assert!(matches!(e, EidolonError::Gpu(_)));
        assert_eq!(e.to_string(), "GPU error: adapter not found");
    }

    #[test]
    fn model_constructor() {
        let e = EidolonError::model("missing part: Head");
        assert!(matches!(e, EidolonError::Model(_)));
        assert_eq!(e.to_string(), "Model error: missing part: Head");
    }

    #[test]
    fn texture_constructor() {
        let e = EidolonError::texture("decode failed");
        assert!(matches!(e, EidolonError::Texture(_)));
        assert_eq!(e.to_string(), "Texture error: decode failed");
    }

    #[test]
    fn conversion_constructor() {
        let e = EidolonError::conversion("bad aspect ratio");
        assert!(matches!(e, EidolonError::Conversion(_)));
        assert_eq!(e.to_string(), "Conversion error: bad aspect ratio");
    }

    #[test]
    fn invalid_path_constructor() {
        let e = EidolonError::invalid_path("null byte in path");
        assert!(matches!(e, EidolonError::InvalidPath(_)));
        assert_eq!(e.to_string(), "Invalid path: null byte in path");
    }

    #[test]
    fn output_constructor() {
        let e = EidolonError::output("encoder failed");
        assert!(matches!(e, EidolonError::Output(_)));
        assert_eq!(e.to_string(), "Output error: encoder failed");
    }

    #[test]
    fn from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: EidolonError = io.into();
        assert!(matches!(e, EidolonError::Io(_)));
        assert!(e.to_string().contains("I/O error"));
    }

    #[test]
    fn from_image_error() {
        let img_err = image::ImageError::Decoding(image::error::DecodingError::new(
            image::error::ImageFormatHint::Unknown,
            "corrupt",
        ));
        let e: EidolonError = img_err.into();
        assert!(matches!(e, EidolonError::Image(_)));
        assert!(std::error::Error::source(&e).is_some());
    }

    #[test]
    fn surface_error_conversion_preserves_recovery_kind() {
        let error: EidolonError = SurfaceError::Lost.into();
        assert!(matches!(
            error,
            EidolonError::Surface(SurfaceError::Lost)
        ));
    }

    #[test]
    fn error_source_returns_inner_io() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let e = EidolonError::Io(io);
        let source = std::error::Error::source(&e);
        assert!(source.is_some());
        assert_eq!(source.unwrap().to_string(), "denied");
    }

    #[test]
    fn error_source_none_for_non_io() {
        let e = EidolonError::gpu("boom");
        assert!(std::error::Error::source(&e).is_none());

        let e = EidolonError::model("boom");
        assert!(std::error::Error::source(&e).is_none());

        let e = EidolonError::texture("boom");
        assert!(std::error::Error::source(&e).is_none());

        let e = EidolonError::conversion("boom");
        assert!(std::error::Error::source(&e).is_none());

        let e = EidolonError::invalid_path("boom");
        assert!(std::error::Error::source(&e).is_none());

        let e = EidolonError::output("boom");
        assert!(std::error::Error::source(&e).is_none());
    }

    #[test]
    fn display_all_variants() {
        let io = std::io::Error::other("oops");
        let img = image::ImageError::Decoding(image::error::DecodingError::new(
            image::error::ImageFormatHint::Unknown,
            "corrupt",
        ));
        assert_eq!(EidolonError::Io(io).to_string(), "I/O error: oops");
        assert_eq!(EidolonError::Image(img).to_string(), "Image error: Format error decoding `Unknown`: corrupt");
        assert_eq!(
            EidolonError::Gpu("shader compile failed".into()).to_string(),
            "GPU error: shader compile failed"
        );
        assert_eq!(
            EidolonError::Model("bad obj".into()).to_string(),
            "Model error: bad obj"
        );
        assert_eq!(
            EidolonError::Texture("bad png".into()).to_string(),
            "Texture error: bad png"
        );
        assert_eq!(
            EidolonError::Conversion("bad size".into()).to_string(),
            "Conversion error: bad size"
        );
        assert_eq!(
            EidolonError::InvalidPath("nul".into()).to_string(),
            "Invalid path: nul"
        );
        assert_eq!(
            EidolonError::Output("write failed".into()).to_string(),
            "Output error: write failed"
        );
    }

    #[test]
    fn debug_format_includes_variant() {
        let e = EidolonError::model("test_msg");
        let debug = format!("{:?}", e);
        assert!(debug.contains("Model"));
        assert!(debug.contains("test_msg"));
    }

    #[test]
    fn constructor_accepts_string_and_str() {
        let from_str = EidolonError::gpu("str");
        let from_string = EidolonError::gpu("string".to_string());
        assert!(matches!(from_str, EidolonError::Gpu(_)));
        assert!(matches!(from_string, EidolonError::Gpu(_)));
    }
}
