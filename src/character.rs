use cgmath::Vector3;

/// Arm width variant: classic (4×4 arms) vs slim (3×4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinType {
    /// Steve-style wide arms (`classic.obj`).
    Classic,
    /// Alex-style slim arms (`slim.obj`).
    Slim,
}

/// Joint angles in degrees. 0° = neutral (no rotation from bind pose).
///
/// Positive yaw turns right, positive pitch looks up.
/// The renderer applies per-joint bind-pose offsets internally;
/// library consumers work with intuitive 0°-is-neutral angles.
#[derive(Debug, Clone, Copy)]
pub struct Posture {
    /// Head yaw around Y (degrees). 0° = facing forward, positive = turn right.
    pub head_yaw: f32,
    /// Head pitch around X (degrees). 0° = level, positive = look up.
    pub head_pitch: f32,
    /// Left arm roll around Z (degrees); negated internally for the left-arm matrix.
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
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 5.32,
        left_arm_pitch: -28.65,
        right_arm_roll: -5.32,
        right_arm_pitch: 28.65,
        left_leg_pitch: 28.65,
        right_leg_pitch: -28.65,
    };
    /// Running pose.
    pub const RUNNING: Posture = Posture {
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 12.27,
        left_arm_pitch: -85.94,
        right_arm_roll: -12.27,
        right_arm_pitch: 85.94,
        left_leg_pitch: 74.48,
        right_leg_pitch: -74.48,
    };
}

/// Scene object: pose, skin type, and world-space transform.
///
/// Skin texture is managed separately — pass it to [`crate::renderer::Renderer::render`]
/// alongside the character.
#[derive(Debug, Clone)]
pub struct Character {
    /// Chooses `classic.obj` vs `slim.obj` arm geometry.
    pub skin_type: SkinType,
    pub posture: Posture,
    /// World-space translation applied before per-part joint matrices.
    pub position: Vector3<f32>,
    /// World-space rotation in degrees (Euler X, then Y, then Z) applied before joint matrices.
    pub rotation: Vector3<f32>,
}

impl Default for Character {
    fn default() -> Self {
        Self {
            skin_type: SkinType::Classic,
            posture: DefaultPostures::STAND,
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Character {
    pub fn new() -> Self {
        Self::default()
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
            head_yaw: 1.0, head_pitch: 2.0,
            left_arm_roll: 3.0, left_arm_pitch: 4.0,
            right_arm_roll: 5.0, right_arm_pitch: 6.0,
            left_leg_pitch: 7.0, right_leg_pitch: 8.0,
        };
        let p2 = p;
        let debug = format!("{:?}", p2);
        assert!(debug.contains("head_yaw"));
        assert!(debug.contains("1.0"));
    }

    #[test]
    fn default_postures_stand_all_zero() {
        let p = DefaultPostures::STAND;
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
        assert!(r.left_arm_pitch.abs() > w.left_arm_pitch.abs(), "running: arms swing wider");
        assert!(r.left_leg_pitch.abs() > w.left_leg_pitch.abs(), "running: legs swing wider");
    }

    #[test]
    fn character_new_defaults() {
        let c = Character::new();
        assert_eq!(c.skin_type, SkinType::Classic);
        assert_eq!(c.posture.head_yaw, 0.0);
        assert_eq!(c.position.x, 0.0);
        assert_eq!(c.position.y, 0.0);
        assert_eq!(c.position.z, 0.0);
        assert_eq!(c.rotation.x, 0.0);
        assert_eq!(c.rotation.y, 0.0);
        assert_eq!(c.rotation.z, 0.0);
    }

    #[test]
    fn character_default_equals_new() {
        let c1 = Character::new();
        let c2 = Character::default();
        assert_eq!(c1.skin_type, c2.skin_type);
        assert_eq!(c1.position.x, c2.position.x);
        assert_eq!(c1.rotation.x, c2.rotation.x);
    }

    #[test]
    fn character_mutation() {
        let mut c = Character::new();
        c.skin_type = SkinType::Slim;
        c.posture = DefaultPostures::RUNNING;
        c.position = Vector3::new(1.0, 2.0, 3.0);
        c.rotation = Vector3::new(45.0, 90.0, 0.0);
        assert_eq!(c.skin_type, SkinType::Slim);
        assert_eq!(c.posture.right_leg_pitch, -74.48);
        assert_eq!(c.position.y, 2.0);
        assert_eq!(c.rotation.x, 45.0);
    }
}
