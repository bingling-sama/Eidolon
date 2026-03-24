use cgmath::{Matrix4, Rad, Vector3};

use crate::camera::Camera;
use crate::character::Character;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniforms {
    pub perspective: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub offset: f32,
    pub _padding: [f32; 3],
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

    // Head
    let head_pivot = Vector3::new(0.0, 1.5, 0.0);
    let head_yaw_rad = (posture.head_yaw - 90.0).to_radians();
    let head_pitch_rad = (posture.head_pitch - 90.0).to_radians();
    let head_rotation =
        Matrix4::from_angle_y(Rad(head_yaw_rad)) * Matrix4::from_angle_x(Rad(head_pitch_rad));
    let head_transform = base_model_matrix
        * Matrix4::from_translation(head_pivot)
        * head_rotation
        * Matrix4::from_translation(-head_pivot);

    // Right Arm
    let right_arm_pivot = Vector3::new(0.3125, 1.375, 0.0);
    let right_arm_roll_rad = posture.right_arm_roll.to_radians();
    let right_arm_pitch_rad = posture.right_arm_pitch.to_radians();
    let right_arm_rotation = Matrix4::from_angle_z(Rad(right_arm_roll_rad))
        * Matrix4::from_angle_x(Rad(right_arm_pitch_rad));
    let right_arm_transform = base_model_matrix
        * Matrix4::from_translation(right_arm_pivot)
        * right_arm_rotation
        * Matrix4::from_translation(-right_arm_pivot);

    // Left Arm
    let left_arm_pivot = Vector3::new(-0.3125, 1.375, 0.0);
    let left_arm_roll_rad = -posture.left_arm_roll.to_radians();
    let left_arm_pitch_rad = posture.left_arm_pitch.to_radians();
    let left_arm_rotation = Matrix4::from_angle_z(Rad(left_arm_roll_rad))
        * Matrix4::from_angle_x(Rad(left_arm_pitch_rad));
    let left_arm_transform = base_model_matrix
        * Matrix4::from_translation(left_arm_pivot)
        * left_arm_rotation
        * Matrix4::from_translation(-left_arm_pivot);

    // Right Leg
    let right_leg_pivot = Vector3::new(0.125, 0.75, 0.0);
    let right_leg_pitch_rad = (posture.right_leg_pitch - 90.0).to_radians();
    let right_leg_rotation = Matrix4::from_angle_x(Rad(right_leg_pitch_rad));
    let right_leg_transform = base_model_matrix
        * Matrix4::from_translation(right_leg_pivot)
        * right_leg_rotation
        * Matrix4::from_translation(-right_leg_pivot);

    // Left Leg
    let left_leg_pivot = Vector3::new(-0.125, 0.75, 0.0);
    let left_leg_pitch_rad = (posture.left_leg_pitch - 90.0).to_radians();
    let left_leg_rotation = Matrix4::from_angle_x(Rad(left_leg_pitch_rad));
    let left_leg_transform = base_model_matrix
        * Matrix4::from_translation(left_leg_pivot)
        * left_leg_rotation
        * Matrix4::from_translation(-left_leg_pivot);

    // Body (no additional rotation)
    let body_transform = base_model_matrix;

    [
        make_uniforms(head_transform, 0.0),
        make_uniforms(right_arm_transform, 0.0),
        make_uniforms(left_arm_transform, 0.0),
        make_uniforms(right_leg_transform, 0.0),
        make_uniforms(left_leg_transform, 0.0),
        make_uniforms(body_transform, 0.0001),
    ]
}
