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
