use std::fmt;
use std::sync::Arc;

use crate::texture::Texture;

/// GPU-backed visual atlas shared by cape and elytra rendering.
#[derive(Clone)]
pub struct BackEquipmentAtlas {
    pub(crate) texture: Arc<Texture>,
    pub(crate) context_id: u64,
}

impl BackEquipmentAtlas {
    pub(crate) fn new(texture: Texture, context_id: u64) -> Self {
        Self {
            texture: Arc::new(texture),
            context_id,
        }
    }
}

impl fmt::Debug for BackEquipmentAtlas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackEquipmentAtlas").finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackEquipmentKind {
    Cape,
    Elytra,
}

/// One back-equipment atlas and the geometry used to present it.
#[derive(Clone, Debug)]
pub struct BackEquipment {
    atlas: BackEquipmentAtlas,
    kind: BackEquipmentKind,
}

impl BackEquipment {
    pub fn new(atlas: BackEquipmentAtlas, kind: BackEquipmentKind) -> Self {
        Self { atlas, kind }
    }

    pub fn kind(&self) -> BackEquipmentKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: BackEquipmentKind) {
        self.kind = kind;
    }

    pub fn atlas(&self) -> &BackEquipmentAtlas {
        &self.atlas
    }
}
