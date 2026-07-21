//! Hardcoded elytra geometry using bs-community/skinview3d's atlas mapping.
//!
//! Each wing uses a 12x22x2 pixel box whose UVs map to the 10x20x2 elytra
//! region of a 64x32 cape texture. The major wing faces reverse their V
//! direction within that atlas region while the narrow side strips retain
//! their atlas direction. Width and height retain skinview3d's
//! expanded silhouette, while depth follows the atlas's two-pixel side strip.

use crate::error::EidolonError;
use crate::model::{ElytraPart, ModelPart, TexturedVertex};
use wgpu::util::DeviceExt;

const MODEL_PIXEL: f32 = 1.0 / 16.0;
const BODY_TOP: f32 = 1.5;
const WING_CENTER_Z: f32 = 2.0 * MODEL_PIXEL + MODEL_PIXEL;
const WING_HALF_WIDTH: f32 = 6.0 * MODEL_PIXEL;
const WING_HALF_HEIGHT: f32 = 11.0 * MODEL_PIXEL;
const WING_HALF_DEPTH: f32 = MODEL_PIXEL;
const WING_CENTER_Y: f32 = BODY_TOP - WING_HALF_HEIGHT - MODEL_PIXEL;

fn wing_vertices(mirrored: bool) -> Vec<TexturedVertex> {
    #[rustfmt::skip]
    let mut positions: [[f32; 3]; 24] = [
        [ WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z + WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y - WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [-WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
        [ WING_HALF_WIDTH, WING_CENTER_Y + WING_HALF_HEIGHT, WING_CENTER_Z - WING_HALF_DEPTH],
    ];

    // skinview3d uses -Z for the player's back; this renderer uses +Z. The
    // final two face blocks therefore put the exterior atlas region on +Z.
    // The major faces reverse V within the elytra region. The narrow X side
    // strips retain their atlas V direction so their alpha follows the wing
    // outline instead of forming detached straight lines. The bottom face
    // uses only row 0; row 1 is transparent padding before the exterior.
    #[rustfmt::skip]
    let uvs: [[f32; 2]; 24] = [
        [0.53125, 0.6875], [0.5625, 0.6875], [0.5625, 0.0625], [0.53125, 0.0625],
        [0.375, 0.0625], [0.34375, 0.0625], [0.34375, 0.6875], [0.375, 0.6875],
        [0.375, 0.0625], [0.53125, 0.0625], [0.53125, 0.0], [0.375, 0.0],
        [0.53125, 0.03125], [0.6875, 0.03125], [0.6875, 0.0], [0.53125, 0.0],
        [0.5625, 0.0625], [0.71875, 0.0625], [0.71875, 0.6875], [0.5625, 0.6875],
        [0.375, 0.0625], [0.53125, 0.0625], [0.53125, 0.6875], [0.375, 0.6875],
    ];

    #[rustfmt::skip]
    let indices: [usize; 36] = [
        0, 1, 2, 0, 2, 3,
        4, 5, 6, 4, 6, 7,
        8, 9, 10, 8, 10, 11,
        12, 13, 14, 12, 14, 15,
        16, 17, 18, 16, 18, 19,
        20, 21, 22, 20, 22, 23,
    ];
    #[rustfmt::skip]
    let mut face_normals: [[f32; 3]; 6] = [
        [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0], [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0], [0.0, 0.0, -1.0],
    ];

    if mirrored {
        for position in &mut positions {
            position[0] = -position[0];
        }
        for normal in &mut face_normals {
            normal[0] = -normal[0];
        }
    }

    let mut vertices = Vec::with_capacity(36);
    for face in 0..6 {
        for index in &indices[face * 6..face * 6 + 6] {
            vertices.push(TexturedVertex {
                position: positions[*index],
                normal: face_normals[face],
                texture: uvs[*index],
            });
        }
    }
    vertices
}

fn upload_wing(device: &wgpu::Device, label: &str, vertices: &[TexturedVertex]) -> ModelPart {
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    ModelPart {
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    }
}

pub fn build_elytra(device: &wgpu::Device) -> Result<ElytraPart, EidolonError> {
    let left = wing_vertices(false);
    let right = wing_vertices(true);
    Ok(ElytraPart {
        left_wing: upload_wing(device, "Left Elytra Wing Vertex Buffer", &left),
        right_wing: upload_wing(device, "Right Elytra Wing Vertex Buffer", &right),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_wing_has_six_box_faces() {
        assert_eq!(wing_vertices(false).len(), 36);
        assert_eq!(wing_vertices(true).len(), 36);
    }

    #[test]
    fn right_wing_mirrors_left_wing_geometry() {
        for (left, right) in wing_vertices(false).iter().zip(wing_vertices(true)) {
            assert_eq!(left.position[0], -right.position[0]);
            assert_eq!(left.position[1], right.position[1]);
            assert_eq!(left.position[2], right.position[2]);
            assert_eq!(left.texture, right.texture);
        }
    }

    #[test]
    fn wing_geometry_has_expected_dimensions() {
        let vertices = wing_vertices(false);
        let bounds = |axis: usize| {
            vertices
                .iter()
                .map(|vertex| vertex.position[axis])
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                    (min.min(value), max.max(value))
                })
        };

        assert_eq!(bounds(0), (-0.375, 0.375));
        assert_eq!(bounds(1), (0.0625, 1.4375));
        assert_eq!(bounds(2), (0.125, 0.25));
    }

    #[test]
    fn wing_top_is_inset_below_the_shoulder_before_posture_rotation() {
        let wing_top = wing_vertices(false)
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(wing_top, BODY_TOP - MODEL_PIXEL);
    }

    #[test]
    fn wing_geometry_depth_matches_the_two_pixel_texture_strip() {
        let vertices = wing_vertices(false);
        let (min_z, max_z) = vertices
            .iter()
            .map(|vertex| vertex.position[2])
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });

        assert_eq!(max_z - min_z, 2.0 * MODEL_PIXEL);
    }

    #[test]
    fn wing_uvs_stay_in_the_elytra_atlas_region() {
        for vertex in wing_vertices(false) {
            assert!((22.0 / 64.0..=46.0 / 64.0).contains(&vertex.texture[0]));
            assert!((0.0..=22.0 / 32.0).contains(&vertex.texture[1]));
        }
    }

    #[test]
    fn wing_major_faces_flip_vertical_atlas_direction() {
        for vertex in wing_vertices(false)
            .iter()
            .filter(|vertex| vertex.normal == [0.0, 0.0, 1.0] || vertex.normal == [0.0, 0.0, -1.0])
        {
            let expected_v = if vertex.position[1] > WING_CENTER_Y {
                22.0 / 32.0
            } else {
                2.0 / 32.0
            };
            assert_eq!(vertex.texture[1], expected_v);
        }
    }

    #[test]
    fn wing_side_faces_keep_atlas_vertical_direction() {
        for vertex in wing_vertices(false)
            .iter()
            .filter(|vertex| vertex.normal[0].abs() == 1.0)
        {
            let expected_v = if vertex.position[1] > WING_CENTER_Y {
                2.0 / 32.0
            } else {
                22.0 / 32.0
            };
            assert_eq!(vertex.texture[1], expected_v);
        }
    }

    #[test]
    fn wing_bottom_face_skips_the_transparent_padding_row() {
        let max_v = wing_vertices(false)
            .iter()
            .filter(|vertex| vertex.normal == [0.0, -1.0, 0.0])
            .map(|vertex| vertex.texture[1])
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(max_v, 1.0 / 32.0);
    }

    #[test]
    fn wing_uvs_match_elytra_atlas_face_mapping() {
        #[rustfmt::skip]
        let expected = [
            [0.53125, 0.6875], [0.5625, 0.6875], [0.5625, 0.0625],
            [0.53125, 0.6875], [0.5625, 0.0625], [0.53125, 0.0625],
            [0.375, 0.0625], [0.34375, 0.0625], [0.34375, 0.6875],
            [0.375, 0.0625], [0.34375, 0.6875], [0.375, 0.6875],
            [0.375, 0.0625], [0.53125, 0.0625], [0.53125, 0.0],
            [0.375, 0.0625], [0.53125, 0.0], [0.375, 0.0],
            [0.53125, 0.03125], [0.6875, 0.03125], [0.6875, 0.0],
            [0.53125, 0.03125], [0.6875, 0.0], [0.53125, 0.0],
            [0.5625, 0.0625], [0.71875, 0.0625], [0.71875, 0.6875],
            [0.5625, 0.0625], [0.71875, 0.6875], [0.5625, 0.6875],
            [0.375, 0.0625], [0.53125, 0.0625], [0.53125, 0.6875],
            [0.375, 0.0625], [0.53125, 0.6875], [0.375, 0.6875],
        ];
        let actual: Vec<_> = wing_vertices(false)
            .into_iter()
            .map(|vertex| vertex.texture)
            .collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn outward_face_uses_the_exterior_texture_region() {
        let vertices = wing_vertices(false);
        let u_bounds = |normal: [f32; 3]| {
            vertices
                .iter()
                .filter(|vertex| vertex.normal == normal)
                .map(|vertex| vertex.texture[0])
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                    (min.min(value), max.max(value))
                })
        };

        assert_eq!(u_bounds([0.0, 0.0, 1.0]), (36.0 / 64.0, 46.0 / 64.0));
        assert_eq!(u_bounds([0.0, 0.0, -1.0]), (24.0 / 64.0, 34.0 / 64.0));
    }

    #[test]
    fn side_faces_preserve_uv_direction_after_z_reflection() {
        let vertices = wing_vertices(false);
        for vertex in vertices
            .iter()
            .filter(|vertex| vertex.normal == [-1.0, 0.0, 0.0])
        {
            let expected_u = if vertex.position[2] < WING_CENTER_Z {
                24.0 / 64.0
            } else {
                22.0 / 64.0
            };
            assert_eq!(vertex.texture[0], expected_u);
        }
    }
}
