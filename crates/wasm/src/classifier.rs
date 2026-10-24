use wasm_bindgen::prelude::*;

/// JS-accessible representation of a texture inspection report.
#[wasm_bindgen]
pub struct JsTextureReport {
    pub(crate) kind: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) hd_scale: u32,
    pub(crate) is_hd: bool,
    pub(crate) can_auto_upgrade: bool,
    pub(crate) description: String,
}

#[wasm_bindgen]
impl JsTextureReport {
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[wasm_bindgen(getter)]
    pub fn hd_scale(&self) -> u32 {
        self.hd_scale
    }

    #[wasm_bindgen(getter)]
    pub fn is_hd(&self) -> bool {
        self.is_hd
    }

    #[wasm_bindgen(getter)]
    pub fn can_auto_upgrade(&self) -> bool {
        self.can_auto_upgrade
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.description.clone()
    }
}

/// Inspect and classify a Minecraft texture from raw image bytes (e.g. PNG).
#[wasm_bindgen]
pub fn inspect_texture(bytes: &[u8]) -> Result<JsTextureReport, JsValue> {
    let rep = eidolon_core::classifier::inspect_texture(bytes)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsTextureReport {
        kind: rep.kind.as_str().to_string(),
        width: rep.width,
        height: rep.height,
        hd_scale: rep.hd_scale,
        is_hd: rep.is_hd,
        can_auto_upgrade: rep.can_auto_upgrade,
        description: rep.description,
    })
}
