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

/// Number of body parts in the model and draw loop.
pub(crate) const BODY_PART_COUNT: usize = 6;
pub(crate) const CAPE_UNIFORM_INDEX: u32 = BODY_PART_COUNT as u32;
pub(crate) const UNIFORM_COUNT: u32 = CAPE_UNIFORM_INDEX + 1;
pub(crate) const CAPE_PIVOT: Vector3<f32> = Vector3::new(0.0, 1.5, 0.125);

/// Identifies a body part in [`PART_CONFIGS`] — single source of truth
/// for mapping config entries to [`Model`] fields.
#[derive(Copy, Clone, Debug)]
pub(crate) enum PartId {
    Head,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
    Body,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniforms {
    pub perspective: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub offset: f32,
    pub _padding: [f32; 3],
}

/// Canonical body part order — defines pivot, layer offset, and which model
/// field each entry targets. Both uniform computation and the render pass
/// draw loop consume this array, guaranteeing they stay in sync.
///
/// Pivot points are joint positions from the Blockbench OBJ model.
/// Coordinate system: Y-up, character centered at X=0 Z=0, feet at Y≈0,
/// head top at Y≈2.
pub(crate) const PART_CONFIGS: &[(Vector3<f32>, f32, PartId); BODY_PART_COUNT] = &[
    // Head: pivot at neck joint
    (Vector3::new(0.0, 1.5, 0.0), 0.0, PartId::Head),
    // Right Arm: pivot at right shoulder joint
    (Vector3::new(0.3125, 1.375, 0.0), 0.0, PartId::RightArm),
    // Left Arm: pivot at left shoulder joint (X mirror)
    (Vector3::new(-0.3125, 1.375, 0.0), 0.0, PartId::LeftArm),
    // Right Leg: pivot at right hip joint
    (Vector3::new(0.125, 0.75, 0.0), 0.0, PartId::RightLeg),
    // Left Leg: pivot at left hip joint (X mirror)
    (Vector3::new(-0.125, 0.75, 0.0), 0.0, PartId::LeftLeg),
    // Body: root — no pivot. Offset 0.0001 prevents Z-fighting with jacket layer.
    (Vector3::new(0.0, 0.0, 0.0), 0.0001, PartId::Body),
];

/// Map a [`PART_CONFIGS`] index to the corresponding [`BodyPart`] in the model.
///
/// The mapping is derived from the `PartId` in PART_CONFIGS — no
/// separate hardcoded order to maintain.
pub(crate) fn body_part_ref<'a>(i: usize, model: &'a Model) -> &'a BodyPart {
    match PART_CONFIGS[i].2 {
        PartId::Head => &model.head,
        PartId::RightArm => &model.right_arm,
        PartId::LeftArm => &model.left_arm,
        PartId::RightLeg => &model.right_leg,
        PartId::LeftLeg => &model.left_leg,
        PartId::Body => &model.body,
    }
}

pub(crate) fn compute_body_part_uniforms(
    character: &Character,
    camera: &Camera,
    width: u32,
    height: u32,
) -> [Uniforms; BODY_PART_COUNT] {
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
        let (pivot, offset, _part_id) = PART_CONFIGS[i];
        let rotation = match i {
            0 =>
            // Head: yaw(Y) then pitch(X)
            {
                Matrix4::from_angle_y(Rad(posture.head_yaw.to_radians()))
                    * Matrix4::from_angle_x(Rad(posture.head_pitch.to_radians()))
            }
            1 =>
            // Right Arm: roll(Z) then pitch(X)
            {
                Matrix4::from_angle_z(Rad(posture.right_arm_roll.to_radians()))
                    * Matrix4::from_angle_x(Rad(posture.right_arm_pitch.to_radians()))
            }
            2 =>
            // Left Arm: -roll(Z) then pitch(X)
            {
                Matrix4::from_angle_z(Rad(-posture.left_arm_roll.to_radians()))
                    * Matrix4::from_angle_x(Rad(posture.left_arm_pitch.to_radians()))
            }
            3 =>
            // Right Leg: pitch(X) only
            {
                Matrix4::from_angle_x(Rad(posture.right_leg_pitch.to_radians()))
            }
            4 =>
            // Left Leg: pitch(X) only
            {
                Matrix4::from_angle_x(Rad(posture.left_leg_pitch.to_radians()))
            }
            5 =>
            // Body: no rotation
            {
                Matrix4::from_scale(1.0)
            }
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

pub(crate) fn compute_cape_uniform(
    character: &Character,
    camera: &Camera,
    width: u32,
    height: u32,
) -> Uniforms {
    let translation = Matrix4::from_translation(character.position);
    let character_rotation = Matrix4::from_angle_x(Rad(character.rotation.x.to_radians()))
        * Matrix4::from_angle_y(Rad(character.rotation.y.to_radians()))
        * Matrix4::from_angle_z(Rad(character.rotation.z.to_radians()));
    let base = translation * character_rotation * Matrix4::from_scale(camera.scale);
    // skinview3d uses -Z for the back; this renderer uses +Z, so invert pitch.
    let cape_rotation = Matrix4::from_angle_x(Rad(-character.cape_pitch.to_radians()));
    let model = base
        * Matrix4::from_translation(CAPE_PIVOT)
        * cape_rotation
        * Matrix4::from_translation(-CAPE_PIVOT);

    Uniforms {
        perspective: camera.get_projection_matrix(width, height),
        view: camera.get_view_matrix(),
        model: model.into(),
        offset: 0.0,
        _padding: [0.0; 3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{Point3, Transform};

    #[test]
    fn cape_rotation_keeps_attachment_axis_fixed() {
        let mut character = Character::new();
        character.cape_pitch = 45.0;
        let camera = Camera::default();
        let uniform = compute_cape_uniform(&character, &camera, 800, 600);
        let model: Matrix4<f32> = uniform.model.into();
        let pivot = Point3::new(CAPE_PIVOT.x, CAPE_PIVOT.y, CAPE_PIVOT.z);

        let transformed = model.transform_point(pivot);
        assert!((transformed.x - CAPE_PIVOT.x).abs() < 1e-6);
        assert!((transformed.y - CAPE_PIVOT.y).abs() < 1e-6);
        assert!((transformed.z - CAPE_PIVOT.z).abs() < 1e-6);
    }

    #[test]
    fn positive_cape_pitch_moves_the_hanging_cape_backwards() {
        let mut character = Character::new();
        character.cape_pitch = 10.8;
        let camera = Camera::default();
        let uniform = compute_cape_uniform(&character, &camera, 800, 600);
        let model: Matrix4<f32> = uniform.model.into();
        let below_pivot = Point3::new(CAPE_PIVOT.x, CAPE_PIVOT.y - 1.0, CAPE_PIVOT.z);

        let transformed = model.transform_point(below_pivot);
        assert!(transformed.z > below_pivot.z);
    }
}
