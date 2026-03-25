use crate::texture::Texture;
use cgmath::Vector3;

/// Arm width variant: classic (4×4 arms) vs slim (3×4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinType {
    /// Steve-style wide arms (`classic.obj`).
    Classic,
    /// Alex-style slim arms (`slim.obj`).
    Slim,
}

/// Joint angles in degrees; consumed by the renderer when building per-part model matrices.
#[derive(Debug, Clone, Copy)]
pub struct Posture {
    /// Head yaw around Y (degrees). Used as `(head_yaw - 90°)` in the head pivot transform.
    pub head_yaw: f32,
    /// Head pitch around X (degrees). Used as `(head_pitch - 90°)` in the head pivot transform.
    pub head_pitch: f32,
    /// Left arm roll around Z (degrees); negated when building the left-arm matrix.
    pub left_arm_roll: f32,
    /// Left arm pitch around X (degrees).
    pub left_arm_pitch: f32,
    /// Right arm roll around Z (degrees).
    pub right_arm_roll: f32,
    /// Right arm pitch around X (degrees).
    pub right_arm_pitch: f32,
    /// Left leg pitch around X (degrees). Used as `(left_leg_pitch - 90°)` at the hip pivot.
    pub left_leg_pitch: f32,
    /// Right leg pitch around X (degrees). Used as `(right_leg_pitch - 90°)` at the hip pivot.
    pub right_leg_pitch: f32,
}

/// Default posture presets.
pub struct DefaultPostures;

impl DefaultPostures {
    /// Neutral standing pose.
    pub const STAND: Posture = Posture {
        head_yaw: 90.0,
        head_pitch: 90.0,
        left_arm_roll: 0.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 0.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 90.0,
        right_leg_pitch: 90.0,
    };
}

/// Scene object: skin mesh, pose, and transform applied in uniform computation.
pub struct Character {
    /// Loaded skin texture; must be set before calling [`crate::renderer::Renderer::render`].
    pub skin: Option<Texture>,
    /// Chooses `classic.obj` vs `slim.obj` arm geometry.
    pub skin_type: SkinType,
    /// Reserved; not used by the current renderer.
    pub cape: Option<Vec<u8>>,
    /// Reserved; not used by the current renderer.
    pub nametag: Option<String>,
    pub posture: Posture,
    /// World-space translation applied before per-part joint matrices.
    pub position: Vector3<f32>,
    /// World-space rotation in degrees (Euler X, then Y, then Z) applied before joint matrices.
    pub rotation: Vector3<f32>,
}

impl Default for Character {
    fn default() -> Self {
        Self {
            skin: None,
            skin_type: SkinType::Classic,
            cape: None,
            nametag: None,
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
