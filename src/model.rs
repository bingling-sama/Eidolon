use glium::VertexBuffer;
use glium::Display;
use std::fs::File;
use std::io::BufReader;
use tobj::{load_obj, GPU_LOAD_OPTIONS};

#[derive(Copy, Clone)]
pub struct TexturedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture: [f32; 2],
}

implement_vertex!(TexturedVertex, position, normal, texture);

pub struct Model {
    pub vertices: VertexBuffer<TexturedVertex>,
}

impl Model {
    pub fn load_from_obj(display: &Display, path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Loading OBJ file: {}", path);
        let (models, _materials) = load_obj(path, &GPU_LOAD_OPTIONS)?;

        println!("OBJ file loaded successfully with {} models", models.len());

        // 创建顶点数组
        let mut vertices: Vec<TexturedVertex> = Vec::new();

        // 遍历所有模型
        for model in models {
            let mesh = &model.mesh;

            // 确保模型有顶点位置
            if mesh.positions.is_empty() {
                continue;
            }

            // 处理顶点位置
            let positions = mesh
                .positions
                .chunks(3)
                .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
                .collect::<Vec<_>>();

            // 处理法线
            let normals = if mesh.normals.is_empty() {
                // 如果没有法线，创建默认法线（向上）
                vec![[0.0, 1.0, 0.0]; positions.len()]
            } else {
                mesh.normals
                    .chunks(3)
                    .map(|n| [n[0] as f32, n[1] as f32, n[2] as f32])
                    .collect::<Vec<_>>()
            };

            // 处理纹理坐标
            let textures = if mesh.texcoords.is_empty() {
                // 如果没有纹理坐标，创建默认坐标
                vec![[0.0, 0.0]; positions.len()]
            } else {
                mesh.texcoords
                    .chunks(2)
                    .map(|t| [t[0] as f32, t[1] as f32])
                    .collect::<Vec<_>>()
            };

            // 使用索引创建顶点
            if !mesh.indices.is_empty() {
                for idx in &mesh.indices {
                    let i = *idx as usize;
                    if i < positions.len() {
                        let normal_idx = i.min(normals.len() - 1);
                        let tex_idx = i.min(textures.len() - 1);
                        vertices.push(TexturedVertex {
                            position: positions[i],
                            normal: normals[normal_idx],
                            texture: textures[tex_idx],
                        });
                    }
                }
            } else {
                // 如果没有索引，直接使用顶点
                for i in 0..positions.len() {
                    let normal_idx = i.min(normals.len() - 1);
                    let tex_idx = i.min(textures.len() - 1);
                    vertices.push(TexturedVertex {
                        position: positions[i],
                        normal: normals[normal_idx],
                        texture: textures[tex_idx],
                    });
                }
            }
        }

        // 确保我们有顶点可用
        if vertices.is_empty() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "无法加载模型顶点",
            )));
        }

        let vertex_buffer = VertexBuffer::new(display, &vertices)?;
        
        Ok(Model {
            vertices: vertex_buffer,
        })
    }
}
