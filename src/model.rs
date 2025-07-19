//! 3D 模型模块
//!
//! 这个模块负责加载和处理 Minecraft 角色的 3D 模型。
//! 支持从 OBJ 文件格式加载模型，并转换为 OpenGL 可用的顶点缓冲区。

use glium::backend::glutin::headless::Headless;
use glium::{implement_vertex, VertexBuffer};
use tobj::{load_obj, GPU_LOAD_OPTIONS};

/// 带纹理的顶点结构体
///
/// 定义了每个顶点包含的数据：位置、法线和纹理坐标。
/// 这个结构体用于在 GPU 和 CPU 之间传递顶点数据。
#[derive(Copy, Clone)]
pub struct TexturedVertex {
    /// 顶点在 3D 空间中的位置坐标 (x, y, z)
    pub position: [f32; 3],
    /// 顶点法线向量，用于光照计算 (nx, ny, nz)
    pub normal: [f32; 3],
    /// 纹理坐标，用于纹理映射 (u, v)
    pub texture: [f32; 2],
}

// 为 TexturedVertex 实现 glium 的顶点特性
implement_vertex!(TexturedVertex, position, normal, texture);

/// 3D 模型结构体
///
/// 封装了模型的顶点数据和相关的渲染信息。
/// 提供了从 OBJ 文件加载模型的功能。
pub struct Model {
    /// 顶点缓冲区，包含所有顶点数据
    pub vertices: VertexBuffer<TexturedVertex>,
}

impl Model {
    /// 从 OBJ 文件加载 3D 模型
    ///
    /// 加载指定路径的 OBJ 文件，解析顶点、法线和纹理坐标，
    /// 并创建 OpenGL 顶点缓冲区。
    ///
    /// # 参数
    ///
    /// * `display` - OpenGL 显示上下文
    /// * `path` - OBJ 文件路径
    ///
    /// # 返回
    ///
    /// 成功时返回 `Model` 实例，失败时返回错误信息
    ///
    /// # 错误
    ///
    /// - 文件不存在或无法读取
    /// - OBJ 文件格式错误
    /// - 模型没有有效的顶点数据
    /// - OpenGL 缓冲区创建失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use skinviewer::model::Model;
    ///
    /// let model = Model::load_from_obj(&display, "resources/player.obj")?;
    /// ```
    pub fn load_from_obj(
        display: &Headless,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
