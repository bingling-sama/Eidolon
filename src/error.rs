//! Typed error for the public API.

use std::fmt;

/// All errors the library can surface.
#[derive(Debug)]
pub enum EidolonError {
    /// Wrapped I/O error (file not found, permission denied, etc.).
    Io(std::io::Error),
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
}

impl fmt::Display for EidolonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Gpu(msg) => write!(f, "GPU error: {msg}"),
            Self::Model(msg) => write!(f, "Model error: {msg}"),
            Self::Texture(msg) => write!(f, "Texture error: {msg}"),
            Self::Conversion(msg) => write!(f, "Conversion error: {msg}"),
            Self::InvalidPath(msg) => write!(f, "Invalid path: {msg}"),
        }
    }
}

impl std::error::Error for EidolonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EidolonError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
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
    fn from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: EidolonError = io.into();
        assert!(matches!(e, EidolonError::Io(_)));
        assert!(e.to_string().contains("I/O error"));
    }

    #[test]
    fn error_source_returns_inner_io() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let e = EidolonError::Io(io);
        let source = std::error::Error::source(&e);
        assert!(source.is_some());
        assert_eq!(
            source.unwrap().to_string(),
            "denied"
        );
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
    }

    #[test]
    fn display_all_variants() {
        let io = std::io::Error::new(std::io::ErrorKind::Other, "oops");
        assert_eq!(
            EidolonError::Io(io).to_string(),
            "I/O error: oops"
        );
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
