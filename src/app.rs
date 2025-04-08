use glium::backend::glutin::SimpleWindowBuilder;
use glium::glutin::surface::WindowSurface;
use glium::winit::event::{Event, WindowEvent};
use glium::winit::event_loop::{ControlFlow, EventLoop};
use glium::Display;

use crate::constants::{FRAGMENT_SHADER, VERTEX_SHADER};
use crate::model::Model;
use crate::renderer::Renderer;
use crate::texture::Texture;

pub struct App {
    window: glium::winit::window::Window,
    display: Display<WindowSurface>,
    model: Model,
    texture: Texture,
    renderer: Renderer,
    event_loop: EventLoop<()>,
}

impl App {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // 创建事件循环
        let event_loop = EventLoop::new()?;

        // 创建窗口和OpenGL上下文
        let (window, display) = SimpleWindowBuilder::new()
            .with_title("Minecraft Skin Viewer")
            .with_inner_size(800, 600)
            .build(&event_loop);

        // 加载纹理
        let texture = Texture::load_from_file(&display, "resources/player.png")?;

        // 加载模型
        let model = Model::load_from_obj(&display, "resources/player.obj")?;

        // 创建渲染器
        let renderer = Renderer::new(&display, VERTEX_SHADER, FRAGMENT_SHADER)?;

        Ok(App {
            window,
            display,
            model,
            texture,
            renderer,
            event_loop,
        })
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let App {
            window,
            display,
            model,
            texture,
            renderer,
            event_loop,
        } = self;

        // 主事件循环
        #[allow(deprecated, reason = "TODO: Migrate this into `.run_app()` later")]
        event_loop.run(move |event, active_event_loop| {
            active_event_loop.set_control_flow(ControlFlow::Wait);

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    active_event_loop.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    // 渲染模型
                    if let Err(e) = renderer.render(&display, &model, &texture) {
                        eprintln!("渲染错误: {:?}", e);
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(ws),
                    ..
                } => {
                    display.resize(ws.into());
                }
                Event::AboutToWait { .. } => {
                    window.request_redraw();
                }
                _ => {}
            }
        })?;

        Ok(())
    }
}
