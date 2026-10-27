use crate::back_equipment::BackEquipment;
use crate::body::Body;
pub use crate::body::SkinType;
use cgmath::Vector3;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterError {
    WouldBecomeEmpty,
}

impl fmt::Display for CharacterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WouldBecomeEmpty => write!(f, "a character must have a body or back equipment"),
        }
    }
}

impl std::error::Error for CharacterError {}

/// Joint angles in degrees. 0° = neutral (no rotation from bind pose).
///
/// Positive yaw turns right, positive pitch looks up.
/// The renderer applies per-joint bind-pose offsets internally;
/// library consumers work with intuitive 0°-is-neutral angles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Posture {
    /// Recommended cape pitch for this posture, in degrees.
    pub cape_pitch: f32,
    /// Whole-character pitch around the model center (degrees).
    pub body_pitch: f32,
    /// Left elytra wing pitch around X (degrees); shared by the mirrored right wing.
    pub elytra_pitch: f32,
    /// Left elytra wing yaw around Y (degrees); mirrored for the right wing.
    pub elytra_yaw: f32,
    /// Left elytra wing roll around Z (degrees); mirrored for the right wing.
    pub elytra_roll: f32,
    /// Head yaw around Y (degrees). 0° = facing forward, positive = turn right.
    pub head_yaw: f32,
    /// Head pitch around X (degrees). 0° = level, positive = look up.
    pub head_pitch: f32,
    /// Left arm roll around Z (degrees). The renderer mirrors the left arm, so a symmetric
    /// pose stores the same value as [`Posture::right_arm_roll`].
    pub left_arm_roll: f32,
    /// Left arm pitch around X (degrees).
    pub left_arm_pitch: f32,
    /// Right arm roll around Z (degrees).
    pub right_arm_roll: f32,
    /// Right arm pitch around X (degrees).
    pub right_arm_pitch: f32,
    /// Left leg pitch around X (degrees). 0° = straight down.
    pub left_leg_pitch: f32,
    /// Right leg pitch around X (degrees). 0° = straight down.
    pub right_leg_pitch: f32,
}

/// Default posture presets.
pub struct DefaultPostures;

impl DefaultPostures {
    /// Neutral standing pose.
    pub const STAND: Posture = Posture {
        cape_pitch: 10.8,
        body_pitch: 0.0,
        elytra_pitch: 15.0,
        elytra_yaw: 0.5729578,
        elytra_roll: 15.0,
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 0.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 0.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 0.0,
        right_leg_pitch: 0.0,
    };
    /// Wave pose.
    pub const WAVE: Posture = Posture {
        cape_pitch: 10.8,
        body_pitch: 0.0,
        elytra_pitch: 15.0,
        elytra_yaw: 0.5729578,
        elytra_roll: 15.0,
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: -28.65,
        left_arm_pitch: 180.0,
        right_arm_roll: 0.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 0.0,
        right_leg_pitch: 0.0,
    };
    /// Walking pose.
    pub const WALKING: Posture = Posture {
        cape_pitch: 10.8,
        body_pitch: 0.0,
        elytra_pitch: 15.0,
        elytra_yaw: 0.5729578,
        elytra_roll: 15.0,
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 4.0,
        left_arm_pitch: -28.65,
        right_arm_roll: 4.0,
        right_arm_pitch: 28.65,
        left_leg_pitch: 28.65,
        right_leg_pitch: -28.65,
    };
    /// Running pose.
    pub const RUNNING: Posture = Posture {
        cape_pitch: 54.0,
        body_pitch: 0.0,
        elytra_pitch: 15.0,
        elytra_yaw: 0.5729578,
        elytra_roll: 15.0,
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 0.0,
        left_arm_pitch: -85.94,
        right_arm_roll: 0.0,
        right_arm_pitch: 85.94,
        left_leg_pitch: 74.48,
        right_leg_pitch: -74.48,
    };
    /// Static horizontal fall-flying pose with spread elytra wings.
    ///
    /// Vanilla dynamically adjusts the wings according to flight direction.
    /// This static preset uses 20 degrees of pitch and 60 degrees of spread.
    pub const FLYING: Posture = Posture {
        cape_pitch: 10.8,
        body_pitch: -90.0,
        elytra_pitch: 20.0,
        elytra_yaw: 0.0,
        elytra_roll: 60.0,
        head_yaw: 0.0,
        head_pitch: 45.0,
        left_arm_roll: 0.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 0.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 0.0,
        right_leg_pitch: 0.0,
    };
}

/// A non-empty player-aligned appearance and its scene state.
#[derive(Clone)]
pub struct Character {
    body: Option<Body>,
    back_equipment: Option<BackEquipment>,
    /// Optional player name rendered above the model. `None` hides the nametag.
    pub nametag: Option<String>,
    /// Whether an occluded nametag remains visible at reduced opacity.
    pub nametag_see_through: bool,
    pub posture: Posture,
    /// World-space translation applied before per-part joint matrices.
    pub position: Vector3<f32>,
    /// World-space rotation in degrees (Euler X, then Y, then Z) applied before joint matrices.
    pub rotation: Vector3<f32>,
}

impl Character {
    fn with_visuals(body: Option<Body>, back_equipment: Option<BackEquipment>) -> Self {
        debug_assert!(body.is_some() || back_equipment.is_some());
        Self {
            body,
            back_equipment,
            nametag: None,
            nametag_see_through: true,
            posture: DefaultPostures::STAND,
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn from_body(body: Body) -> Self {
        Self::with_visuals(Some(body), None)
    }

    pub fn from_back_equipment(back_equipment: BackEquipment) -> Self {
        Self::with_visuals(None, Some(back_equipment))
    }

    pub fn body(&self) -> Option<&Body> {
        self.body.as_ref()
    }

    pub fn body_mut(&mut self) -> Option<&mut Body> {
        self.body.as_mut()
    }

    pub fn attach_body(&mut self, body: Body) -> Option<Body> {
        self.body.replace(body)
    }

    pub fn remove_body(&mut self) -> Result<Option<Body>, CharacterError> {
        if self.body.is_none() {
            return Ok(None);
        }
        if self.back_equipment.is_none() {
            return Err(CharacterError::WouldBecomeEmpty);
        }
        Ok(self.body.take())
    }

    pub fn back_equipment(&self) -> Option<&BackEquipment> {
        self.back_equipment.as_ref()
    }

    pub fn back_equipment_mut(&mut self) -> Option<&mut BackEquipment> {
        self.back_equipment.as_mut()
    }

    pub fn equip(&mut self, back_equipment: BackEquipment) -> Option<BackEquipment> {
        self.back_equipment.replace(back_equipment)
    }

    pub fn unequip(&mut self) -> Result<Option<BackEquipment>, CharacterError> {
        if self.back_equipment.is_none() {
            return Ok(None);
        }
        if self.body.is_none() {
            return Err(CharacterError::WouldBecomeEmpty);
        }
        Ok(self.back_equipment.take())
    }

    #[cfg(test)]
    pub(crate) fn empty_for_test() -> Self {
        Self {
            body: None,
            back_equipment: None,
            nametag: None,
            nametag_see_through: true,
            posture: DefaultPostures::STAND,
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

impl fmt::Debug for Character {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Character")
            .field("body", &self.body)
            .field("back_equipment", &self.back_equipment)
            .field("nametag", &self.nametag)
            .field("nametag_see_through", &self.nametag_see_through)
            .field("posture", &self.posture)
            .field("position", &self.position)
            .field("rotation", &self.rotation)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skin_type_eq() {
        assert_eq!(SkinType::Classic, SkinType::Classic);
        assert_eq!(SkinType::Slim, SkinType::Slim);
        assert_ne!(SkinType::Classic, SkinType::Slim);
    }

    #[test]
    fn nametag_is_hidden_by_default() {
        assert_eq!(Character::empty_for_test().nametag, None);
    }

    #[test]
    fn nametag_uses_translucent_occlusion_by_default() {
        assert!(Character::empty_for_test().nametag_see_through);
    }

    #[test]
    fn skin_type_clone() {
        let s = SkinType::Slim;
        assert_eq!(s, s.clone());
    }

    #[test]
    fn skin_type_debug() {
        let d = format!("{:?}", SkinType::Classic);
        assert!(d.contains("Classic"));
        let d = format!("{:?}", SkinType::Slim);
        assert!(d.contains("Slim"));
    }

    #[test]
    fn posture_debug_and_clone() {
        let p = Posture {
            cape_pitch: 9.0,
            body_pitch: 0.0,
            elytra_pitch: 10.0,
            elytra_yaw: 11.0,
            elytra_roll: 12.0,
            head_yaw: 1.0,
            head_pitch: 2.0,
            left_arm_roll: 3.0,
            left_arm_pitch: 4.0,
            right_arm_roll: 5.0,
            right_arm_pitch: 6.0,
            left_leg_pitch: 7.0,
            right_leg_pitch: 8.0,
        };
        let p2 = p;
        let debug = format!("{:?}", p2);
        assert!(debug.contains("head_yaw"));
        assert!(debug.contains("1.0"));
    }

    #[test]
    fn default_postures_stand_all_zero() {
        let p = DefaultPostures::STAND;
        assert_eq!(p.cape_pitch, 10.8);
        assert_eq!(p.head_yaw, 0.0);
        assert_eq!(p.head_pitch, 0.0);
        assert_eq!(p.left_arm_roll, 0.0);
        assert_eq!(p.left_arm_pitch, 0.0);
        assert_eq!(p.right_arm_roll, 0.0);
        assert_eq!(p.right_arm_pitch, 0.0);
        assert_eq!(p.left_leg_pitch, 0.0);
        assert_eq!(p.right_leg_pitch, 0.0);
    }

    #[test]
    fn default_postures_wave_has_left_arm_up() {
        let p = DefaultPostures::WAVE;
        assert!(p.left_arm_pitch > 0.0, "wave: left arm should be raised");
        assert!(p.left_arm_roll != 0.0, "wave: left arm should roll");
        assert_eq!(p.right_arm_pitch, 0.0, "wave: right arm stays down");
        assert_eq!(p.left_leg_pitch, 0.0, "wave: legs stay still");
    }

    #[test]
    fn default_postures_walking_alternating_limbs() {
        let p = DefaultPostures::WALKING;
        // Arms and legs alternate: left arm forward, right arm back
        assert!(p.left_arm_pitch < 0.0, "walking: left arm swings forward");
        assert!(p.right_arm_pitch > 0.0, "walking: right arm swings back");
        // Legs alternate opposite: left leg forward, right leg back
        assert!(p.left_leg_pitch > 0.0, "walking: left leg swings forward");
        assert!(p.right_leg_pitch < 0.0, "walking: right leg swings back");
    }

    #[test]
    fn default_postures_running_larger_angles_than_walking() {
        let w = DefaultPostures::WALKING;
        let r = DefaultPostures::RUNNING;
        assert!(
            r.left_arm_pitch.abs() > w.left_arm_pitch.abs(),
            "running: arms swing wider"
        );
        assert!(
            r.left_leg_pitch.abs() > w.left_leg_pitch.abs(),
            "running: legs swing wider"
        );
        assert_eq!(w.cape_pitch, 10.8);
        assert_eq!(r.cape_pitch, 54.0);
    }

    #[test]
    fn default_postures_flying_sets_minecraft_glide_pose() {
        let stand = DefaultPostures::STAND;
        let flying = DefaultPostures::FLYING;

        assert_eq!(stand.elytra_pitch, 15.0);
        assert_eq!(stand.elytra_roll, 15.0);
        assert_eq!(flying.elytra_pitch, 20.0);
        assert_eq!(flying.elytra_yaw, 0.0);
        assert_eq!(flying.elytra_roll, 60.0);
        assert_eq!(flying.body_pitch, -90.0);
        assert_eq!(flying.head_pitch, 45.0);
        assert_eq!(flying.left_arm_pitch, 0.0);
        assert_eq!(flying.right_arm_pitch, 0.0);
        assert_eq!(flying.left_leg_pitch, 0.0);
        assert_eq!(flying.right_leg_pitch, 0.0);
    }

    #[test]
    fn character_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Character>();
    }

    #[test]
    fn character_scene_defaults() {
        let c = Character::empty_for_test();
        assert_eq!(c.posture.head_yaw, 0.0);
        assert_eq!(c.position.x, 0.0);
        assert_eq!(c.position.y, 0.0);
        assert_eq!(c.position.z, 0.0);
        assert_eq!(c.rotation.x, 0.0);
        assert_eq!(c.rotation.y, 0.0);
        assert_eq!(c.rotation.z, 0.0);
        assert_eq!(c.posture.cape_pitch, 10.8);
    }

    #[test]
    fn character_mutation() {
        let mut c = Character::empty_for_test();
        c.posture = DefaultPostures::RUNNING;
        c.position = Vector3::new(1.0, 2.0, 3.0);
        c.rotation = Vector3::new(45.0, 90.0, 0.0);
        assert_eq!(c.posture.right_leg_pitch, -74.48);
        assert_eq!(c.position.y, 2.0);
        assert_eq!(c.rotation.x, 45.0);
    }
}
