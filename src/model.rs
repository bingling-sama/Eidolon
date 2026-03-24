//! 3D 模型模块
//!
//! 这个模块负责加载和处理 Minecraft 角色的 3D 模型。
//! 它将 OBJ 文件中的命名对象解析为独立的、可控制的身体部位。

use log::info;
use std::collections::HashMap;
use tobj::{load_obj, GPU_LOAD_OPTIONS};
use wgpu::util::DeviceExt;

/// 带纹理的顶点结构体
///
/// 定义了每个顶点包含的数据：位置、法线和纹理坐标。
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

/// 代表模型的一个可渲染部分
pub struct ModelPart {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}

/// 代表一个逻辑身体部位，通常包含一个主模型和一个附加层
pub struct BodyPart {
    pub main: ModelPart,
    pub layer: ModelPart,
}

/// 包含所有命名身体部件的角色模型
pub struct Model {
    pub head: BodyPart,
    pub body: BodyPart,
    pub right_arm: BodyPart,
    pub left_arm: BodyPart,
    pub right_leg: BodyPart,
    pub left_leg: BodyPart,
}

impl Model {
    /// 从 OBJ 文件加载 3D 模型
    ///
    /// 加载指定路径的 OBJ 文件，并将其中的命名对象解析到
    /// `Model` 结构对应的身体部位中。
    ///
    /// # 参数
    ///
    /// * `device` - wgpu 设备
    /// * `path` - OBJ 文件路径
    ///
    /// # 返回
    ///
    /// 成功时返回 `Model` 实例，如果 OBJ 文件缺少必要的部件则返回错误。
    pub fn load_from_obj(
        device: &wgpu::Device,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Loading OBJ file: {}", path);
        let (models, _materials) = load_obj(path, &GPU_LOAD_OPTIONS)?;
        info!("OBJ file loaded with {} objects", models.len());

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
        ) -> Result<ModelPart, String> {
            parts
                .remove(name)
                .ok_or_else(|| format!("Missing model part: {}", name))
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
