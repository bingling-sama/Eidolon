//! Loads the rigged Minecraft player mesh from OBJ assets.

use crate::error::EidolonError;
use log::info;
use std::collections::HashMap;
use tobj::{load_obj, GPU_LOAD_OPTIONS};
use wgpu::util::DeviceExt;

/// Vertex layout: position, normal, UV (matches the skin shader `VertexInput`).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexturedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture: [f32; 2],
}

impl TexturedVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TexturedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Indexed triangle mesh uploaded as a single vertex buffer.
pub struct ModelPart {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}

/// One body region: opaque `main` mesh plus `layer` overlay (hat/body/armor layer).
pub struct BodyPart {
    pub main: ModelPart,
    pub layer: ModelPart,
}

/// Full player model: six body parts, each with main + layer geometry.
pub struct Model {
    pub head: BodyPart,
    pub body: BodyPart,
    pub right_arm: BodyPart,
    pub left_arm: BodyPart,
    pub right_leg: BodyPart,
    pub left_leg: BodyPart,
}

impl Model {
    /// Load an OBJ from a file path where each object name maps to a fixed body part.
    ///
    /// Required object names: `Head`, `Hat Layer`, `Body`, `Body Layer`, `Right Arm`,
    /// `Right Arm Layer`, `Left Arm`, `Left Arm Layer`, `Right Leg`, `Right Leg Layer`,
    /// `Left Leg`, `Left Leg Layer`.
    pub fn load_from_obj(
        device: &wgpu::Device,
        path: &str,
    ) -> Result<Self, EidolonError> {
        info!("Loading OBJ file: {}", path);
        let (models, _materials) = load_obj(path, &GPU_LOAD_OPTIONS)
            .map_err(|e| EidolonError::model(format!("failed to load OBJ '{}': {}", path, e)))?;
        info!("OBJ file loaded with {} objects", models.len());
        Self::build_from_tobj(device, models)
    }

    /// Load an OBJ from in-memory bytes. Same object-name requirements as [`load_from_obj`].
    pub fn load_from_obj_bytes(
        device: &wgpu::Device,
        data: &[u8],
        name_hint: &str,
    ) -> Result<Self, EidolonError> {
        info!("Loading OBJ from bytes ({})", name_hint);
        let (models, _materials) = tobj::load_obj_buf(
            &mut std::io::Cursor::new(data),
            &GPU_LOAD_OPTIONS,
            |_| unreachable!("no material files when loading from bytes"),
        )
        .map_err(|e| EidolonError::model(format!("failed to parse OBJ bytes ({}): {}", name_hint, e)))?;
        info!("OBJ bytes loaded with {} objects", models.len());
        Self::build_from_tobj(device, models)
    }

    fn build_from_tobj(
        device: &wgpu::Device,
        models: Vec<tobj::Model>,
    ) -> Result<Self, EidolonError> {
        let mut parts: HashMap<String, ModelPart> = HashMap::new();

        for model in models {
            let mesh = &model.mesh;
            if mesh.positions.is_empty() {
                continue;
            }

            let mut vertices_data = Vec::new();
            let positions: Vec<_> = mesh.positions.chunks(3).collect();
            let normals: Vec<_> = mesh.normals.chunks(3).collect();
            let texcoords: Vec<_> = mesh.texcoords.chunks(2).collect();

            for i in 0..mesh.indices.len() {
                let pos_idx = mesh.indices[i] as usize;
                let pos = [positions[pos_idx][0], positions[pos_idx][1], positions[pos_idx][2]];

                let nml_idx = if !mesh.normal_indices.is_empty() {
                    mesh.normal_indices[i] as usize
                } else {
                    pos_idx
                };
                let nml = if nml_idx < normals.len() {
                    [normals[nml_idx][0], normals[nml_idx][1], normals[nml_idx][2]]
                } else {
                    [0.0, 1.0, 0.0]
                };

                let tex_idx = if !mesh.texcoord_indices.is_empty() {
                    mesh.texcoord_indices[i] as usize
                } else {
                    pos_idx
                };
                let tex = if tex_idx < texcoords.len() {
                    [texcoords[tex_idx][0], 1.0 - texcoords[tex_idx][1]]
                } else {
                    [0.0, 0.0]
                };

                vertices_data.push(TexturedVertex {
                    position: pos,
                    normal: nml,
                    texture: tex,
                });
            }

            if vertices_data.is_empty() {
                continue;
            }

            let vertex_count = vertices_data.len() as u32;
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Vertex Buffer: {}", model.name)),
                contents: bytemuck::cast_slice(&vertices_data),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let model_part = ModelPart {
                vertex_buffer,
                vertex_count,
            };
            info!("Loaded part: {}", model.name);
            parts.insert(model.name, model_part);
        }

        fn extract_part(
            parts: &mut HashMap<String, ModelPart>,
            name: &str,
        ) -> Result<ModelPart, EidolonError> {
            parts
                .remove(name)
                .ok_or_else(|| EidolonError::model(format!("Missing model part: {}", name)))
        }

        Ok(Model {
            head: BodyPart {
                main: extract_part(&mut parts, "Head")?,
                layer: extract_part(&mut parts, "Hat Layer")?,
            },
            body: BodyPart {
                main: extract_part(&mut parts, "Body")?,
                layer: extract_part(&mut parts, "Body Layer")?,
            },
            right_arm: BodyPart {
                main: extract_part(&mut parts, "Right Arm")?,
                layer: extract_part(&mut parts, "Right Arm Layer")?,
            },
            left_arm: BodyPart {
                main: extract_part(&mut parts, "Left Arm")?,
                layer: extract_part(&mut parts, "Left Arm Layer")?,
            },
            right_leg: BodyPart {
                main: extract_part(&mut parts, "Right Leg")?,
                layer: extract_part(&mut parts, "Right Leg Layer")?,
            },
            left_leg: BodyPart {
                main: extract_part(&mut parts, "Left Leg")?,
                layer: extract_part(&mut parts, "Left Leg Layer")?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("No wgpu adapter");
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("No wgpu device")
    }

    #[test]
    fn load_slim_model_has_all_parts() {
        let (device, _queue) = make_device();
        let model = Model::load_from_obj(&device, "resources/slim.obj")
            .expect("Failed to load slim model");
        let _ = &model.head;
        let _ = &model.body;
        let _ = &model.right_arm;
        let _ = &model.left_arm;
        let _ = &model.right_leg;
        let _ = &model.left_leg;
    }

    #[test]
    fn load_classic_model_has_all_parts() {
        let (device, _queue) = make_device();
        let model = Model::load_from_obj(&device, "resources/classic.obj")
            .expect("Failed to load classic model");
        let _ = &model.head;
        let _ = &model.body;
        let _ = &model.right_arm;
        let _ = &model.left_arm;
        let _ = &model.right_leg;
        let _ = &model.left_leg;
    }

    #[test]
    fn load_nonexistent_model_returns_error() {
        let (device, _queue) = make_device();
        let result = Model::load_from_obj(&device, "nonexistent_file.obj");
        assert!(result.is_err());
    }

    #[test]
    fn load_from_obj_bytes_missing_head_part_errors() {
        let (device, _queue) = make_device();
        // OBJ with "Hat Layer" but missing "Head" — extract_part("Head") fails
        let obj_data = b"o Hat Layer\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let result = Model::load_from_obj_bytes(&device, obj_data, "no_head.obj");
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Missing model part: Head"),
            "Expected 'Missing model part: Head', got: {msg}"
        );
    }

    #[test]
    fn load_from_obj_bytes_missing_layer_part_errors() {
        let (device, _queue) = make_device();
        // OBJ with "Head" but missing "Hat Layer"
        let obj_data = b"o Head\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let result = Model::load_from_obj_bytes(&device, obj_data, "no_hat_layer.obj");
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Missing model part: Hat Layer"),
            "Expected 'Missing model part: Hat Layer', got: {msg}"
        );
    }

    #[test]
    fn load_from_obj_bytes_invalid_obj_syntax_errors() {
        let (device, _queue) = make_device();
        // Garbage bytes — tobj::load_obj_buf parse fails
        let result = Model::load_from_obj_bytes(&device, b"this is not an obj file at all", "junk.obj");
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Model error"),
            "Expected Model error for invalid OBJ, got: {msg}"
        );
    }

    #[test]
    fn load_from_obj_bytes_empty_mesh_skipped() {
        let (device, _queue) = make_device();
        // OBJ with named objects but zero vertices — mesh.positions is empty, skipped
        // Only "Head" and "Hat Layer" defined with empty geometry leads to missing-part error
        let obj_data = b"o Head\n# no vertices\n";
        let result = Model::load_from_obj_bytes(&device, obj_data, "empty.obj");
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Missing model part"),
            "Expected missing-part error for empty-mesh OBJ, got: {msg}"
        );
    }

    #[test]
    fn textured_vertex_desc_layout_is_valid() {
        let desc = TexturedVertex::desc();
        assert_eq!(desc.step_mode, wgpu::VertexStepMode::Vertex);
        assert_eq!(desc.attributes.len(), 3);
        // Position: Float32x3 at offset 0
        assert_eq!(desc.attributes[0].format, wgpu::VertexFormat::Float32x3);
        assert_eq!(desc.attributes[0].shader_location, 0);
        // Normal: Float32x3 at size_of<[f32;3]>()
        assert_eq!(desc.attributes[1].format, wgpu::VertexFormat::Float32x3);
        assert_eq!(desc.attributes[1].shader_location, 1);
        // UV: Float32x2 at size_of<[f32;6]>()
        assert_eq!(desc.attributes[2].format, wgpu::VertexFormat::Float32x2);
        assert_eq!(desc.attributes[2].shader_location, 2);
    }
}
