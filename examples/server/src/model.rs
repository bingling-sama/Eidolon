use image::{DynamicImage, GenericImageView};
use serde::{de, Deserialize, Deserializer};

/// Heuristic auto-detection for Alex (slim) skins based on arm transparency.
pub fn is_slim_skin(image: &DynamicImage) -> bool {
    let (width, height) = image.dimensions();
    if width != height || width < 64 {
        return false;
    }
    let scale = width / 64;
    // In Minecraft 64x64 skins:
    // Right arm in 64-base:
    // Classic arm is 4px wide, Alex is 3px wide.
    // For 3px arm, the extra column at x in [54..56], y in [20..32] is unused and fully transparent.
    // Check alpha in this region.
    for x in (54 * scale)..(56 * scale) {
        for y in (20 * scale)..(32 * scale) {
            let pixel = image.get_pixel(x, y);
            if pixel[3] > 0 {
                return false;
            }
        }
    }
    true
}

fn deserialize_flexible_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    struct FlexibleBool;

    impl<'de> de::Visitor<'de> for FlexibleBool {
        type Value = Option<bool>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or string representation of a boolean")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
            Ok(Some(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
            Ok(Some(value != 0))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
            Ok(Some(value != 0))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value.to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Ok(Some(true)),
                "false" | "0" | "no" | "off" => Ok(Some(false)),
                _ => Err(de::Error::unknown_variant(
                    value,
                    &["true", "false", "1", "0"],
                )),
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D2>(self, deserializer: D2) -> Result<Self::Value, D2::Error>
        where
            D2: Deserializer<'de>,
        {
            deserializer.deserialize_any(FlexibleBool)
        }
    }

    deserializer.deserialize_any(FlexibleBool)
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct RenderQuery {
    // Skin aliases
    #[serde(alias = "skinUrl", alias = "skin_url", alias = "skin")]
    pub skin_url: Option<String>,

    // Cape aliases
    #[serde(alias = "capeUrl", alias = "cape_url", alias = "cape")]
    pub cape_url: Option<String>,

    // Elytra aliases
    #[serde(alias = "elytraUrl", alias = "elytra_url", alias = "elytra")]
    pub elytra_url: Option<String>,

    // Slim flag (Alex model)
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    pub slim: Option<bool>,

    // Nametag
    #[serde(alias = "nameTag")]
    pub nametag: Option<String>,
    #[serde(
        default,
        alias = "disableNametagSeeThrough",
        alias = "disable_nametag_see_through",
        deserialize_with = "deserialize_flexible_bool"
    )]
    pub disable_nametag_see_through: Option<bool>,

    // Posture: idle, walk, run, wave, fly
    pub posture: Option<String>,

    // Overrides
    #[serde(alias = "capePitch", alias = "cape_pitch")]
    pub cape_pitch: Option<f32>,
    #[serde(alias = "elytraPitch", alias = "elytra_pitch")]
    pub elytra_pitch: Option<f32>,
    #[serde(alias = "elytraSpread", alias = "elytra_spread")]
    pub elytra_spread: Option<f32>,

    #[serde(alias = "headYaw", alias = "head_yaw")]
    pub head_yaw: Option<f32>,
    #[serde(alias = "headPitch", alias = "head_pitch")]
    pub head_pitch: Option<f32>,
    #[serde(alias = "leftArmRoll", alias = "left_arm_roll")]
    pub left_arm_roll: Option<f32>,
    #[serde(alias = "leftArmPitch", alias = "left_arm_pitch")]
    pub left_arm_pitch: Option<f32>,
    #[serde(alias = "rightArmRoll", alias = "right_arm_roll")]
    pub right_arm_roll: Option<f32>,
    #[serde(alias = "rightArmPitch", alias = "right_arm_pitch")]
    pub right_arm_pitch: Option<f32>,
    #[serde(alias = "leftLegPitch", alias = "left_leg_pitch")]
    pub left_leg_pitch: Option<f32>,
    #[serde(alias = "rightLegPitch", alias = "right_leg_pitch")]
    pub right_leg_pitch: Option<f32>,

    // Camera
    #[serde(alias = "camYaw", alias = "cam_yaw")]
    pub cam_yaw: Option<f32>,
    #[serde(alias = "camPitch", alias = "cam_pitch")]
    pub cam_pitch: Option<f32>,
    #[serde(alias = "camZoom", alias = "cam_zoom")]
    pub cam_zoom: Option<f32>,

    // Viewport & format
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,

    // SkinRenderMC options
    pub definition: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    pub transparent: Option<bool>,

    // Position / Rotation
    #[serde(alias = "posX", alias = "pos_x")]
    pub pos_x: Option<f32>,
    #[serde(alias = "posY", alias = "pos_y")]
    pub pos_y: Option<f32>,
    #[serde(alias = "posZ", alias = "pos_z")]
    pub pos_z: Option<f32>,
    #[serde(alias = "rotX", alias = "rot_x")]
    pub rot_x: Option<f32>,
    #[serde(alias = "rotY", alias = "rot_y")]
    pub rot_y: Option<f32>,
    #[serde(alias = "rotZ", alias = "rot_z")]
    pub rot_z: Option<f32>,
}

impl RenderQuery {
    pub fn resolved_skin_url(&self) -> Option<&str> {
        self.skin_url.as_deref().filter(|s| !s.trim().is_empty())
    }

    pub fn resolved_cape_url(&self) -> Option<&str> {
        self.cape_url.as_deref().filter(|s| !s.trim().is_empty())
    }

    pub fn resolved_elytra_url(&self) -> Option<&str> {
        self.elytra_url.as_deref().filter(|s| !s.trim().is_empty())
    }

    pub fn resolved_nametag(&self) -> Option<&str> {
        self.nametag.as_deref().filter(|s| !s.trim().is_empty())
    }

    pub fn is_slim(&self, skin_bytes: &[u8]) -> bool {
        if let Some(s) = self.slim {
            return s;
        }
        // Auto-detect from skin pixels if not specified
        if let Ok(img) = image::load_from_memory(skin_bytes) {
            is_slim_skin(&img)
        } else {
            false
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        let w = self.width.unwrap_or(512).clamp(16, 4096);
        let h = self.height.unwrap_or(512).clamp(16, 4096);
        (w, h)
    }

    pub fn merge_with(mut self, other: RenderQuery) -> Self {
        if other.skin_url.is_some() {
            self.skin_url = other.skin_url;
        }
        if other.cape_url.is_some() {
            self.cape_url = other.cape_url;
        }
        if other.elytra_url.is_some() {
            self.elytra_url = other.elytra_url;
        }
        if other.slim.is_some() {
            self.slim = other.slim;
        }
        if other.nametag.is_some() {
            self.nametag = other.nametag;
        }
        if other.disable_nametag_see_through.is_some() {
            self.disable_nametag_see_through = other.disable_nametag_see_through;
        }
        if other.posture.is_some() {
            self.posture = other.posture;
        }
        if other.cape_pitch.is_some() {
            self.cape_pitch = other.cape_pitch;
        }
        if other.elytra_pitch.is_some() {
            self.elytra_pitch = other.elytra_pitch;
        }
        if other.elytra_spread.is_some() {
            self.elytra_spread = other.elytra_spread;
        }
        if other.head_yaw.is_some() {
            self.head_yaw = other.head_yaw;
        }
        if other.head_pitch.is_some() {
            self.head_pitch = other.head_pitch;
        }
        if other.left_arm_roll.is_some() {
            self.left_arm_roll = other.left_arm_roll;
        }
        if other.left_arm_pitch.is_some() {
            self.left_arm_pitch = other.left_arm_pitch;
        }
        if other.right_arm_roll.is_some() {
            self.right_arm_roll = other.right_arm_roll;
        }
        if other.right_arm_pitch.is_some() {
            self.right_arm_pitch = other.right_arm_pitch;
        }
        if other.left_leg_pitch.is_some() {
            self.left_leg_pitch = other.left_leg_pitch;
        }
        if other.right_leg_pitch.is_some() {
            self.right_leg_pitch = other.right_leg_pitch;
        }
        if other.cam_yaw.is_some() {
            self.cam_yaw = other.cam_yaw;
        }
        if other.cam_pitch.is_some() {
            self.cam_pitch = other.cam_pitch;
        }
        if other.cam_zoom.is_some() {
            self.cam_zoom = other.cam_zoom;
        }
        if other.width.is_some() {
            self.width = other.width;
        }
        if other.height.is_some() {
            self.height = other.height;
        }
        if other.format.is_some() {
            self.format = other.format;
        }
        if other.definition.is_some() {
            self.definition = other.definition;
        }
        if other.transparent.is_some() {
            self.transparent = other.transparent;
        }
        if other.pos_x.is_some() {
            self.pos_x = other.pos_x;
        }
        if other.pos_y.is_some() {
            self.pos_y = other.pos_y;
        }
        if other.pos_z.is_some() {
            self.pos_z = other.pos_z;
        }
        if other.rot_x.is_some() {
            self.rot_x = other.rot_x;
        }
        if other.rot_y.is_some() {
            self.rot_y = other.rot_y;
        }
        if other.rot_z.is_some() {
            self.rot_z = other.rot_z;
        }
        self
    }

    pub fn is_webp(&self) -> bool {
        self.format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("webp"))
    }

    /// Resolution scaling factor from definition (default: 1.5, clamped 0.8..=3.5)
    pub fn resolved_definition(&self) -> f32 {
        self.definition.unwrap_or(1.5).clamp(0.8, 3.5)
    }

    /// Single view dimension derived strictly from definition (base 350x670 * definition)
    pub fn skinrendermc_single_dimensions(&self) -> (u32, u32) {
        let scale = self.resolved_definition();
        let w = ((350.0 * scale).round() as u32).clamp(32, 4096);
        let h = ((670.0 * scale).round() as u32).clamp(32, 4096);
        (w, h)
    }

    /// Both views dimensions derived strictly from definition (base 700x670 * definition)
    pub fn skinrendermc_both_dimensions(&self) -> (u32, u32) {
        let scale = self.resolved_definition();
        let single_w = ((350.0 * scale).round() as u32).clamp(32, 4096);
        let h = ((670.0 * scale).round() as u32).clamp(32, 4096);
        (single_w * 2, h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skinrendermc_dimensions_scaling() {
        let mut query = RenderQuery::default();
        // default definition = 1.5, base 350x670 -> (525, 1005)
        assert_eq!(query.skinrendermc_single_dimensions(), (525, 1005));
        assert_eq!(query.skinrendermc_both_dimensions(), (1050, 1005));

        query.definition = Some(1.0);
        assert_eq!(query.skinrendermc_single_dimensions(), (350, 670));
        assert_eq!(query.skinrendermc_both_dimensions(), (700, 670));

        query.definition = Some(2.0);
        assert_eq!(query.skinrendermc_single_dimensions(), (700, 1340));
        assert_eq!(query.skinrendermc_both_dimensions(), (1400, 1340));

        // Clamping to bounds 0.8..=3.5
        query.definition = Some(0.1);
        assert_eq!(query.skinrendermc_single_dimensions(), (280, 536));

        query.definition = Some(10.0);
        assert_eq!(query.skinrendermc_single_dimensions(), (1225, 2345));
    }

    #[test]
    fn test_deserialize_query_flexible_options() {
        let json_data = r#"{
            "skinUrl": "https://example.com/skin.png",
            "nameTag": "TestUser",
            "definition": 1.2,
            "transparent": false,
            "slim": "true"
        }"#;

        let parsed: RenderQuery = serde_json::from_str(json_data).unwrap();
        assert_eq!(
            parsed.skin_url.as_deref(),
            Some("https://example.com/skin.png")
        );
        assert_eq!(parsed.resolved_nametag(), Some("TestUser"));
        assert_eq!(parsed.transparent, Some(false));
        assert_eq!(parsed.slim, Some(true));
        assert_eq!(parsed.definition, Some(1.2));
    }

    #[test]
    fn test_merge_with() {
        let q1 = RenderQuery {
            definition: Some(2.0),
            transparent: Some(true),
            ..Default::default()
        };
        let q2 = RenderQuery {
            skin_url: Some("https://example.com/skin.png".to_string()),
            transparent: Some(false),
            ..Default::default()
        };
        let merged = q1.merge_with(q2);
        assert_eq!(merged.definition, Some(2.0));
        assert_eq!(merged.transparent, Some(false));
        assert_eq!(
            merged.skin_url.as_deref(),
            Some("https://example.com/skin.png")
        );
    }
}
