#[macro_use]
extern crate glium;

use glium::backend::glutin::SimpleWindowBuilder;
use glium::uniforms::SamplerWrapFunction::Repeat;
use glium::winit::event::{Event, WindowEvent};
use glium::winit::event_loop::{ControlFlow, EventLoop};
use glium::{uniform, Surface};
use glium::{Program, VertexBuffer};
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;
use tobj::{load_obj, GPU_LOAD_OPTIONS};

#[derive(Copy, Clone)]
struct TexturedVertex {
    position: [f32; 3],
    normal: [f32; 3],
    texture: [f32; 2],
}
implement_vertex!(TexturedVertex, position, normal, texture);

const VERTEX_SHADER: &str = r#"
    #version 410

    in vec3 position;
    in vec3 normal;
    in vec2 texture;

    out vec3 v_normal;
    out vec2 v_texture;

    uniform mat4 perspective;
    uniform mat4 view;
    uniform mat4 model;

    void main() {
        mat4 modelview = view * model;
        v_texture = texture;
        v_normal = transpose(inverse(mat3(model))) * normal;
        gl_Position = perspective * modelview * vec4(position, 1.0);
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 330 core
    in vec2 v_texture;
    in vec3 v_normal;
    out vec4 FragColor;

    uniform sampler2D texture1;

    void main()
    {
        // Minecraft textures are pixel art, so we want nearest neighbor filtering
        vec4 texColor = texture(texture1, v_texture);

        // 只丢弃完全透明的像素，保留半透明像素
        if(texColor.a < 0.01)
            discard;

        // Enhanced lighting for Minecraft models
        vec3 lightDir1 = normalize(vec3(1.0, 1.0, 1.0));
        vec3 lightDir2 = normalize(vec3(-1.0, 0.5, -0.5)); // Secondary light from opposite direction

        float ambient = 0.5; // Higher ambient for Minecraft-style lighting
        float diff1 = max(dot(normalize(v_normal), lightDir1), 0.0);
        float diff2 = max(dot(normalize(v_normal), lightDir2), 0.0) * 0.3; // Secondary light is dimmer

        vec3 diffuse = (ambient + diff1 * 0.5 + diff2) * vec3(1.0, 1.0, 1.0);

        // Apply lighting but preserve original colors
        FragColor = vec4(texColor.rgb * diffuse, texColor.a);
    }
"#;

// Function to create a view matrix
fn view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
    let f = {
        let f = direction;
        let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
        let len = len.sqrt();
        [f[0] / len, f[1] / len, f[2] / len]
    };

    let s = [up[1] * f[2] - up[2] * f[1],
             up[2] * f[0] - up[0] * f[2],
             up[0] * f[1] - up[1] * f[0]];

    let s_norm = {
        let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
        let len = len.sqrt();
        [s[0] / len, s[1] / len, s[2] / len]
    };

    let u = [f[1] * s_norm[2] - f[2] * s_norm[1],
             f[2] * s_norm[0] - f[0] * s_norm[2],
             f[0] * s_norm[1] - f[1] * s_norm[0]];

    let p = [-position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
             -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
             -position[0] * f[0] - position[1] * f[1] - position[2] * f[2]];

    [
        [s_norm[0], u[0], f[0], 0.0],
        [s_norm[1], u[1], f[1], 0.0],
        [s_norm[2], u[2], f[2], 0.0],
        [p[0], p[1], p[2], 1.0],
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an event loop
    let event_loop = EventLoop::new()?;

    // Create a window and OpenGL context
    let (window, display) = SimpleWindowBuilder::new()
        .with_title("Minecraft Skin Viewer")
        .with_inner_size(800, 600)
        .build(&event_loop);

    // 加载皮肤纹理
    println!("Loading skin texture...");
    let image = image::load(
        BufReader::new(File::open("resources/player.png")?),
        image::ImageFormat::Png,
    )?.to_rgba8();

    let image_dimensions = image.dimensions();
    println!("Texture dimensions: {}x{}", image_dimensions.0, image_dimensions.1);

    // 创建 OpenGL 纹理
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let texture = glium::texture::Texture2d::new(&display, image)?;
    println!("Texture loaded into GPU");

    // Compile shaders
    let program = Program::from_source(&display, VERTEX_SHADER, FRAGMENT_SHADER, None)?;

    // 加载玩家模型
    println!("Loading OBJ file...");
    let (models, _materials) = load_obj("resources/player.obj", &GPU_LOAD_OPTIONS)?;

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
        eprintln!("错误：无法加载模型顶点");
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "无法加载模型顶点")));
    }
    let vertex_buffer = VertexBuffer::new(&display, &vertices)?;

    // Set up depth testing and disable backface culling
    let params = glium::DrawParameters {
        depth: glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            .. Default::default()
        },
        backface_culling: glium::draw_parameters::BackfaceCullingMode::CullingDisabled, // 禁用背面剔除
        blend: glium::Blend::alpha_blending(), // 启用alpha混合
        .. Default::default()
    };

    // For animation/rotation
    let start_time = Instant::now();

    // Main event loop
    #[allow(deprecated, reason = "TODO: Migrate this into `.run_app()` later")]
    event_loop.run(move |event, active_event_loop| {
        active_event_loop.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                active_event_loop.exit();
            },
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                // 计算基于时间的旋转，降低旋转速度
                let elapsed = start_time.elapsed().as_secs_f32();
                let rotation_angle = elapsed * 0.3; // 降低旋转速度

                // Draw the model
                let mut target = display.draw();
                // Use a darker blue background to better showcase the model
                target.clear_color_and_depth((0.2, 0.2, 0.4, 1.0), 1.0);

                let perspective = {
                    let (width, height) = target.get_dimensions();
                    let aspect_ratio = height as f32 / width as f32;

                    let fov: f32 = std::f32::consts::PI / 3.0;
                    let zfar = 1024.0;
                    let znear = 0.1;

                    let f = 1.0 / (fov / 2.0).tan();

                    [
                        [f * aspect_ratio, 0.0, 0.0, 0.0],
                        [0.0, f, 0.0, 0.0],
                        [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                        [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
                    ]
                };

                // 调整相机位置以更好地查看模型
                // 将相机置于稍高一点的位置，并微微向下看
                let view = view_matrix(&[0.0, 0.3, 3.0], &[0.0, -0.2, -1.0], &[0.0, 1.0, 0.0]);

                // 创建带旋转的模型矩阵
                let scale = 1.2; // 缩放比例
                let model = [
                    [rotation_angle.cos() * scale, 0.0, rotation_angle.sin() * scale, 0.0],
                    [0.0, scale, 0.0, 0.0],
                    [-rotation_angle.sin() * scale, 0.0, rotation_angle.cos() * scale, 0.0],
                    [0.0, -1.4, 0.0, 1.0f32]  // 将模型向下移动更多
                ];

                let uniforms = uniform! {
                    perspective: perspective,
                    view: view,
                    model: model,
                    texture1: texture.sampled()
                        .wrap_function(Repeat)
                        .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                        .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
                };

                target.draw(
                    &vertex_buffer,
                    &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                    &program,
                    &uniforms,
                    &params
                ).unwrap();

                target.finish().unwrap();
            },
            Event::WindowEvent { event: WindowEvent::Resized(ws), .. } => {
                display.resize(ws.into());
            },
            Event::AboutToWait { .. } => {
                window.request_redraw();
            },
            _ => {}
        }
    })?;

    Ok(())
}
