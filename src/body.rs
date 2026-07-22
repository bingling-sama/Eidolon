use std::fmt;
use std::sync::Arc;

use crate::texture::Texture;

/// Arm width variant used by a visible body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinType {
    /// Steve-style wide arms (`classic.obj`).
    Classic,
    /// Alex-style slim arms (`slim.obj`).
    Slim,
}

/// A visible player body: one skin atlas and its arm geometry selection.
#[derive(Clone)]
pub struct Body {
    pub(crate) skin: Arc<Texture>,
    skin_type: SkinType,
    pub(crate) context_id: u64,
}

impl Body {
    pub(crate) fn new(texture: Texture, skin_type: SkinType, context_id: u64) -> Self {
        Self {
            skin: Arc::new(texture),
            skin_type,
            context_id,
        }
    }

    pub fn skin_type(&self) -> SkinType {
        self.skin_type
    }

    pub fn set_skin_type(&mut self, skin_type: SkinType) {
        self.skin_type = skin_type;
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body")
            .field("skin_type", &self.skin_type)
            .finish_non_exhaustive()
    }
}
