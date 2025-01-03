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
        uniform mat4 matrix;

        void main() {
            mat4 modelview = view * matrix;
            v_texture = texture;
            v_normal = transpose(inverse(mat3(matrix))) * normal;
            gl_Position = perspective * modelview * vec4(position, 1.0);
        }
    "#;

const FRAGMENT_SHADER: &str = r#"
        #version 330 core
        in vec2 v_texture;
        out vec4 FragColor;

        uniform sampler2D texture1;

        void main()
        {
            vec4 texColor = texture(texture1, v_texture);
            if(texColor.a < 0.1)
                discard;
            FragColor = texColor;
        }
    "#;

fn view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
    let f = {
        let f = direction;
        let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
        let len = len.sqrt();
        [f[0] / len, f[1] / len, f[2] / len]
    };

    let s = [
        up[1] * f[2] - up[2] * f[1],
        up[2] * f[0] - up[0] * f[2],
        up[0] * f[1] - up[1] * f[0],
    ];

    let s_norm = {
        let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
        let len = len.sqrt();
        [s[0] / len, s[1] / len, s[2] / len]
    };

    let u = [
        f[1] * s_norm[2] - f[2] * s_norm[1],
        f[2] * s_norm[0] - f[0] * s_norm[2],
        f[0] * s_norm[1] - f[1] * s_norm[0],
    ];

    let p = [
        -position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
        -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
        -position[0] * f[0] - position[1] * f[1] - position[2] * f[2],
    ];

    [
        [s_norm[0], u[0], f[0], 0.0],
        [s_norm[1], u[1], f[1], 0.0],
        [s_norm[2], u[2], f[2], 0.0],
        [p[0], p[1], p[2], 1.0],
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;

    let (window, display) = SimpleWindowBuilder::new()
        .with_title("LollipopRender")
        .build(&event_loop);

    let image = image::load(
        BufReader::new(File::open("resources/skin.png")?),
        image::ImageFormat::Png,
    )?
    .to_rgba8();
    let image_dimensions = image.dimensions();
    let image =
        glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let texture = glium::texture::Texture2d::new(&display, image)?;

    let program = Program::from_source(&display, VERTEX_SHADER, FRAGMENT_SHADER, None)?;

    let (models, _materials) = load_obj("resources/player.obj", &GPU_LOAD_OPTIONS)?;
    let mut vertices: Vec<TexturedVertex> = Vec::new();
    for model in models {
        let mesh = &model.mesh;
        let positions = mesh
            .positions
            .chunks(3)
            .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
            .collect::<Vec<_>>();
        let normals = mesh
            .normals
            .chunks(3)
            .map(|n| [n[0] as f32, n[1] as f32, n[2] as f32])
            .collect::<Vec<_>>();
        let textures = mesh
            .texcoords
            .chunks(2)
            .map(|t| [t[0] as f32, t[1] as f32])
            .collect::<Vec<_>>();

        for idx in &mesh.indices {
            let i = *idx as usize;
            vertices.push(TexturedVertex {
                position: positions[i],
                normal: normals[i],
                texture: textures[i],
            })
        }
    }
    let vb = VertexBuffer::new(&display, &vertices)?;

    let params = glium::DrawParameters {
        depth: glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            .. Default::default()
        },
        .. Default::default()
    };

    #[allow(deprecated, reason = "TODO: Migrate this into `.run_app()` later")]
    event_loop.run(move |event, active_event_loop| {
        active_event_loop.set_control_flow(ControlFlow::Wait);

        match event {
            Event::LoopExiting => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => active_event_loop.exit(),
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let mut target = display.draw();

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

                let view = view_matrix(&[0.0, 0.0, 5.0], &[0.0, 0.0, -1.0], &[0.0, 1.0, 0.0]);

                let uniforms = uniform! {
                    matrix: [
                        [0.6, 0.0, 0.0, 0.0],
                        [0.0, 0.6, 0.0, 0.0],
                        [0.0, 0.0, 0.6, 0.0],
                        [0.0, 0.0, 0.0, 1.0f32]
                    ],
                    perspective: perspective,
                    view: view,
                    texture1: texture.sampled()
                        .wrap_function(Repeat)
                        .minify_filter(glium::uniforms::MinifySamplerFilter::NearestMipmapLinear)
                        .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
                };
                target.clear_color(56.0 / 255.0f32, 164.0 / 255.0f32, 90.0 / 255.0f32, 1.0f32);
                target
                    .draw(
                        &vb,
                        &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                        &program,
                        &uniforms,
                        &params,
                    )
                    .unwrap();
                target.finish().unwrap();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(ws),
                ..
            } => display.resize(ws.into()),
            Event::AboutToWait { .. } => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;

    Ok(())
}
