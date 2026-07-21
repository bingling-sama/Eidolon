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
pub(crate) const LEFT_ELYTRA_UNIFORM_INDEX: u32 = CAPE_UNIFORM_INDEX + 1;
pub(crate) const RIGHT_ELYTRA_UNIFORM_INDEX: u32 = LEFT_ELYTRA_UNIFORM_INDEX + 1;
pub(crate) const UNIFORM_COUNT: u32 = RIGHT_ELYTRA_UNIFORM_INDEX + 1;
pub(crate) const CAPE_PIVOT: Vector3<f32> = Vector3::new(0.0, 1.5, 0.125);
pub(crate) const ELYTRA_PIVOT: Vector3<f32> = Vector3::new(0.0, 1.5, 0.125);
const BODY_PIVOT: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

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

fn compute_base_model_matrix(character: &Character, camera: &Camera) -> Matrix4<f32> {
    let translation = Matrix4::from_translation(character.position);
    let character_rotation = Matrix4::from_angle_x(Rad(character.rotation.x.to_radians()))
        * Matrix4::from_angle_y(Rad(character.rotation.y.to_radians()))
        * Matrix4::from_angle_z(Rad(character.rotation.z.to_radians()));
    let body_rotation = Matrix4::from_translation(BODY_PIVOT)
        * Matrix4::from_angle_x(Rad(character.posture.body_pitch.to_radians()))
        * Matrix4::from_translation(-BODY_PIVOT);

    translation * character_rotation * Matrix4::from_scale(camera.scale) * body_rotation
}

pub(crate) fn compute_body_part_uniforms(
    character: &Character,
    camera: &Camera,
    width: u32,
    height: u32,
) -> [Uniforms; BODY_PART_COUNT] {
    let perspective: [[f32; 4]; 4] = camera.get_projection_matrix(width, height);
    let view: [[f32; 4]; 4] = camera.get_view_matrix();

    let base_model_matrix = compute_base_model_matrix(character, camera);

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
    let base = compute_base_model_matrix(character, camera);
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

pub(crate) fn compute_elytra_uniforms(
    character: &Character,
    camera: &Camera,
    width: u32,
    height: u32,
) -> [Uniforms; 2] {
    let base = compute_base_model_matrix(character, camera);
    let projection = camera.get_projection_matrix(width, height);
    let view = camera.get_view_matrix();

    let posture = character.posture;
    let wing = |pivot: Vector3<f32>, yaw: f32, roll: f32| {
        // Match upstream ModelPart/Three.js local XYZ Euler rotation order.
        let rotation = Matrix4::from_angle_z(Rad(roll.to_radians()))
            * Matrix4::from_angle_y(Rad(yaw.to_radians()))
            * Matrix4::from_angle_x(Rad(-posture.elytra_pitch.to_radians()));
        let model =
            base * Matrix4::from_translation(pivot) * rotation * Matrix4::from_translation(-pivot);
        Uniforms {
            perspective: projection,
            view,
            model: model.into(),
            offset: 0.0,
            _padding: [0.0; 3],
        }
    };

    // skinview3d uses -Z for the back. Mirroring that axis reverses X/Y
    // rotations while preserving the left/right Z roll.
    [
        wing(ELYTRA_PIVOT, -posture.elytra_yaw, posture.elytra_roll),
        wing(ELYTRA_PIVOT, posture.elytra_yaw, -posture.elytra_roll),
    ]
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

    #[test]
    fn elytra_wings_keep_the_central_back_attachment_fixed() {
        let camera = Camera::default();
        for posture in [
            crate::character::DefaultPostures::STAND,
            crate::character::DefaultPostures::FLYING,
        ] {
            let mut character = Character::new();
            character.posture = posture;
            let uniforms = compute_elytra_uniforms(&character, &camera, 800, 600);
            let base = compute_base_model_matrix(&character, &camera);

            for uniform in &uniforms {
                let model: Matrix4<f32> = uniform.model.into();
                let point = Point3::new(CAPE_PIVOT.x, CAPE_PIVOT.y, CAPE_PIVOT.z);
                let transformed = model.transform_point(point);
                let expected = base.transform_point(point);
                assert!((transformed.x - expected.x).abs() < 1e-6);
                assert!((transformed.y - expected.y).abs() < 1e-6);
                assert!((transformed.z - expected.z).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn flying_posture_spreads_both_elytra_wings() {
        let camera = Camera::default();
        let standing = Character::new();
        let mut flying = Character::new();
        flying.posture = crate::character::DefaultPostures::FLYING;

        let standing_uniforms = compute_elytra_uniforms(&standing, &camera, 800, 600);
        let flying_uniforms = compute_elytra_uniforms(&flying, &camera, 800, 600);
        let pivots = [ELYTRA_PIVOT; 2];
        let mut standing_tips = Vec::new();
        let mut flying_tips = Vec::new();

        for ((standing_uniform, flying_uniform), pivot) in
            standing_uniforms.iter().zip(&flying_uniforms).zip(pivots)
        {
            let tip = Point3::new(pivot.x, pivot.y - 1.0, pivot.z);
            let standing_model: Matrix4<f32> = standing_uniform.model.into();
            let flying_model: Matrix4<f32> = flying_uniform.model.into();
            standing_tips.push(standing_model.transform_point(tip));
            flying_tips.push(flying_model.transform_point(tip));
        }

        assert!(flying_tips[0].x > standing_tips[0].x);
        assert!(flying_tips[1].x < standing_tips[1].x);
        assert!((flying_tips[0].x + flying_tips[1].x).abs() < 1e-6);
        assert!((flying_tips[0].y - flying_tips[1].y).abs() < 1e-6);
        assert!((flying_tips[0].z - flying_tips[1].z).abs() < 1e-6);
    }

    #[test]
    fn elytra_rotation_uses_upstream_local_xyz_order() {
        let mut character = Character::new();
        character.posture.elytra_pitch = 23.0;
        character.posture.elytra_yaw = 17.0;
        character.posture.elytra_roll = 41.0;
        let camera = Camera::default();
        let actual: Matrix4<f32> = compute_elytra_uniforms(&character, &camera, 800, 600)[0]
            .model
            .into();
        let rotation = Matrix4::from_angle_z(Rad(41.0_f32.to_radians()))
            * Matrix4::from_angle_y(Rad(-17.0_f32.to_radians()))
            * Matrix4::from_angle_x(Rad(-23.0_f32.to_radians()));
        let expected = compute_base_model_matrix(&character, &camera)
            * Matrix4::from_translation(ELYTRA_PIVOT)
            * rotation
            * Matrix4::from_translation(-ELYTRA_PIVOT);
        let probe = Point3::new(0.25, 0.5, 0.25);
        let actual_probe = actual.transform_point(probe);
        let expected_probe = expected.transform_point(probe);

        assert!((actual_probe.x - expected_probe.x).abs() < 1e-6);
        assert!((actual_probe.y - expected_probe.y).abs() < 1e-6);
        assert!((actual_probe.z - expected_probe.z).abs() < 1e-6);
    }

    #[test]
    fn flying_posture_rotates_the_player_horizontally_around_its_center() {
        let mut character = Character::new();
        character.posture = crate::character::DefaultPostures::FLYING;
        let uniforms = compute_body_part_uniforms(&character, &Camera::default(), 800, 600);
        let body_model: Matrix4<f32> = uniforms[5].model.into();

        let top = body_model.transform_point(Point3::new(0.0, 2.0, 0.0));
        let bottom = body_model.transform_point(Point3::new(0.0, 0.0, 0.0));

        assert!((top.y - 1.0).abs() < 1e-6);
        assert!((bottom.y - 1.0).abs() < 1e-6);
        assert!(top.z < -0.9);
        assert!(bottom.z > 0.9);
    }
}
