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
