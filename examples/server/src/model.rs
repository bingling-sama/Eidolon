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

    pub fn is_webp(&self) -> bool {
        self.format
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case("webp"))
    }
}
