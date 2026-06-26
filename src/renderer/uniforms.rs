//! Per-body-part [`Uniforms`] for the six-part draw loop.
//!
//! `Camera::scale` is folded into the base model matrix (uniform scale).
//! The body entry uses a small positive `offset` so the outer layer mesh
//! clears the inner mesh along normals.
//!
//! [`PART_CONFIGS`] defines the canonical body part order; both
//! [`compute_body_part_uniforms`] and [`body_part_ref`] consume it,
//! guaranteeing the draw loop and uniform upload stay in sync.

use cgmath::{Matrix4, Rad, Vector3};

use crate::camera::Camera;
use crate::character::Character;
use crate::model::{BodyPart, Model};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniforms {
    pub perspective: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub offset: f32,
    pub _padding: [f32; 3],
}

/// Canonical body part order. Both uniforms computation and render pass
/// draw loop consume this array, guaranteeing they stay in sync.
///
/// Each entry: (pivot_point, layer_offset).
/// Pivot points are joint positions from the Blockbench OBJ model.
/// Coordinate system: Y-up, character centered at X=0 Z=0, feet at Y≈0,
/// head top at Y≈2.
/// - Head pivot: neck joint (bottom-center of head mesh)
/// - Arm/leg pivots: shoulder/hip joint (top-center of limb mesh)
/// - Body pivot: (0,0,0) — root, no rotation applied
///
/// The -90° offsets in uniform computation convert from CLI angle space
/// (90° = "facing forward") to model bind-pose space (0° = facing +Z).
pub(crate) const PART_CONFIGS: &[(Vector3<f32>, f32); 6] = &[
    // Head: pivot at neck joint
    (Vector3::new(0.0, 1.5, 0.0), 0.0),
    // Right Arm: pivot at right shoulder joint
    (Vector3::new(0.3125, 1.375, 0.0), 0.0),
    // Left Arm: pivot at left shoulder joint (X mirror)
    (Vector3::new(-0.3125, 1.375, 0.0), 0.0),
    // Right Leg: pivot at right hip joint
    (Vector3::new(0.125, 0.75, 0.0), 0.0),
    // Left Leg: pivot at left hip joint (X mirror)
    (Vector3::new(-0.125, 0.75, 0.0), 0.0),
    // Body: root — no pivot. Offset 0.0001 prevents Z-fighting with jacket layer.
    (Vector3::new(0.0, 0.0, 0.0), 0.0001),
];

/// Map a PART_CONFIGS index to the corresponding body part in the model.
///
/// Co-located with [`PART_CONFIGS`] so the order definition and its
/// consumers stay in one file.
pub(crate) fn body_part_ref<'a>(i: usize, model: &'a Model) -> &'a BodyPart {
    match i {
        0 => &model.head,
        1 => &model.right_arm,
        2 => &model.left_arm,
        3 => &model.right_leg,
        4 => &model.left_leg,
        5 => &model.body,
        _ => unreachable!(),
    }
}

pub(crate) fn compute_body_part_uniforms(
    character: &Character,
    camera: &Camera,
    width: u32,
    height: u32,
) -> [Uniforms; 6] {
    let perspective: [[f32; 4]; 4] = camera.get_projection_matrix(width, height);
    let view: [[f32; 4]; 4] = camera.get_view_matrix();

    let translation = Matrix4::from_translation(character.position);
    let rotation_matrix = Matrix4::from_angle_x(Rad(character.rotation.x.to_radians()))
        * Matrix4::from_angle_y(Rad(character.rotation.y.to_radians()))
        * Matrix4::from_angle_z(Rad(character.rotation.z.to_radians()));
    let scale = Matrix4::from_scale(camera.scale);
    let base_model_matrix = translation * rotation_matrix * scale;

    let posture = &character.posture;

    let make_uniforms = |model_matrix: Matrix4<f32>, offset: f32| -> Uniforms {
        Uniforms {
            perspective,
            view,
            model: model_matrix.into(),
            offset,
            _padding: [0.0; 3],
        }
    };

    std::array::from_fn(|i| {
        let (pivot, offset) = PART_CONFIGS[i];
        let rotation = match i {
            0 => // Head: yaw(Y) then pitch(X)
                Matrix4::from_angle_y(Rad((posture.head_yaw - 90.0).to_radians()))
                    * Matrix4::from_angle_x(Rad((posture.head_pitch - 90.0).to_radians())),
            1 => // Right Arm: roll(Z) then pitch(X)
                Matrix4::from_angle_z(Rad(posture.right_arm_roll.to_radians()))
                    * Matrix4::from_angle_x(Rad(posture.right_arm_pitch.to_radians())),
            2 => // Left Arm: -roll(Z) then pitch(X)
                Matrix4::from_angle_z(Rad(-posture.left_arm_roll.to_radians()))
                    * Matrix4::from_angle_x(Rad(posture.left_arm_pitch.to_radians())),
            3 => // Right Leg: pitch(X) only
                Matrix4::from_angle_x(Rad((posture.right_leg_pitch - 90.0).to_radians())),
            4 => // Left Leg: pitch(X) only
                Matrix4::from_angle_x(Rad((posture.left_leg_pitch - 90.0).to_radians())),
            5 => // Body: no rotation
                Matrix4::from_scale(1.0),
            _ => unreachable!(),
        };

        // Body (index 5) has no pivot — applies base transform directly
        let transform = if i == 5 {
            base_model_matrix
        } else {
            base_model_matrix
                * Matrix4::from_translation(pivot)
                * rotation
                * Matrix4::from_translation(-pivot)
        };

        make_uniforms(transform, offset)
    })
}
