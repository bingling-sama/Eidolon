//! Hardcoded cape geometry from bs-community/skinview3d.
//!
//! Cape is a BoxGeometry(10, 16, 1) in pixel units — 10px wide, 16px tall,
//! 1px deep. UVs map to a 64×32 Back equipment atlas using the same layout as
//! skinview3d's `setCapeUVs`.
//!
//! Voxel box → 6 faces × 2 triangles = 12 tris = 36 non-indexed vertices.

use crate::error::EidolonError;
use crate::model::{CapePart, ModelPart, TexturedVertex};
use wgpu::util::DeviceExt;

const MODEL_PIXEL: f32 = 1.0 / 16.0;
const CAPE_HALF_DEPTH: f32 = MODEL_PIXEL / 2.0;
const BODY_BACK_Z: f32 = 2.0 * MODEL_PIXEL;
const BODY_TOP: f32 = 1.5;
// Match skinview3d: the cape starts at the body's top-back edge and extends out.
const CAPE_Z: f32 = BODY_BACK_Z + CAPE_HALF_DEPTH;
const CAPE_Y: f32 = BODY_TOP - 0.5;

/// Build the hardcoded cape mesh and upload to GPU.
///
/// The geometry is centered at origin. The caller positions it on the
/// player's upper back via the pivot transform in the shader uniform.
pub(crate) fn build_cape(device: &wgpu::Device) -> Result<CapePart, EidolonError> {
    // ── 24 unique vertices (position + UV) from skinview3d BoxGeometry(10, 16, 1) ──
    // Pixel coords converted to model space: ÷ 16.
    // The top edge aligns with the body shoulder line.
    // Positive Z places it behind the model in this coordinate system.
    #[rustfmt::skip]
    let positions: [[f32; 3]; 24] = [
        // Right face (+X): vertices 0–3
        [ 0.3125, -0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 0
        [ 0.3125, -0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 1
        [ 0.3125,  0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 2
        [ 0.3125,  0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 3
        // Left face (-X): vertices 4–7
        [-0.3125,  0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 4
        [-0.3125,  0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 5
        [-0.3125, -0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 6
        [-0.3125, -0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 7
        // Top face (+Y): vertices 8–11
        [-0.3125,  0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 8
        [ 0.3125,  0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 9
        [ 0.3125,  0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 10
        [-0.3125,  0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 11
        // Bottom face (-Y): vertices 12–15
        [-0.3125, -0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 12
        [ 0.3125, -0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 13
        [ 0.3125, -0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 14
        [-0.3125, -0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 15
        // Front face (+Z): vertices 16–19
        [-0.3125, -0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 16
        [ 0.3125, -0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 17
        [ 0.3125,  0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 18
        [-0.3125,  0.5 + CAPE_Y,  CAPE_HALF_DEPTH + CAPE_Z], // 19
        // Back face (-Z): vertices 20–23
        [ 0.3125, -0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 20
        [-0.3125, -0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 21
        [-0.3125,  0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 22
        [ 0.3125,  0.5 + CAPE_Y, -CAPE_HALF_DEPTH + CAPE_Z], // 23
    ];

    #[rustfmt::skip]
    let uvs: [[f32; 2]; 24] = [
        // Right face
        [0.171875, 0.53125],  [0.1875, 0.53125],   [0.1875, 0.03125],    [0.171875, 0.03125],
        // Left face
        [0.0, 0.03125],       [0.015625, 0.03125], [0.015625, 0.53125],  [0.0, 0.53125],
        // Top face
        [0.015625, 0.03125],  [0.171875, 0.03125], [0.171875, 0.0],      [0.015625, 0.0],
        // Bottom face
        [0.171875, 0.5625],   [0.328125, 0.5625],  [0.328125, 0.53125],  [0.171875, 0.53125],
        // Front face (+Z — outward from player back)
        [0.015625, 0.53125],  [0.171875, 0.53125], [0.171875, 0.03125],  [0.015625, 0.03125],
        // Back face (-Z — against player)
        [0.1875, 0.53125],    [0.34375, 0.53125],  [0.34375, 0.03125],   [0.1875, 0.03125],
    ];

    // ── 36 indices (6 faces × 2 triangles × 3 verts) ──
    #[rustfmt::skip]
    let indices: [u32; 36] = [
        0, 1, 2,  0, 2, 3,       // Right  (+X)
        4, 5, 6,  4, 6, 7,       // Left   (-X)
        8, 9, 10, 8, 10, 11,     // Top    (+Y)
        12, 13, 14, 12, 14, 15,  // Bottom (-Y)
        16, 17, 18, 16, 18, 19,  // Front  (+Z)
        20, 21, 22, 20, 22, 23,  // Back   (-Z)
    ];

    // Face normals: same order as index groups above
    #[rustfmt::skip]
    let face_normals: [[f32; 3]; 6] = [
        [ 1.0,  0.0,  0.0], // Right
        [-1.0,  0.0,  0.0], // Left
        [ 0.0,  1.0,  0.0], // Top
        [ 0.0, -1.0,  0.0], // Bottom
        [ 0.0,  0.0,  1.0], // Front (+Z)
        [ 0.0,  0.0, -1.0], // Back (-Z)
    ];

    // Build non-indexed vertex buffer: 36 TexturedVertex entries
    let mut vertices_data: Vec<TexturedVertex> = Vec::with_capacity(36);
    for face in 0..6 {
        let normal = face_normals[face];
        for k in 0..6 {
            let vi = indices[face * 6 + k] as usize;
            vertices_data.push(TexturedVertex {
                position: positions[vi],
                normal,
                texture: uvs[vi],
            });
        }
    }

    let vertex_count = vertices_data.len() as u32;
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cape Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    Ok(CapePart {
        main: ModelPart {
            vertex_buffer,
            vertex_count,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cape_inner_face_touches_the_body() {
        let clearance = (CAPE_Z - CAPE_HALF_DEPTH) - BODY_BACK_Z;
        assert_eq!(clearance, 0.0);
    }

    #[test]
    fn cape_top_aligns_with_the_shoulder() {
        let cape_top = CAPE_Y + 0.5;
        assert_eq!(cape_top, BODY_TOP);
    }
}
