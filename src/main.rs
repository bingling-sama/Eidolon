use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use eidolon::{
    camera::Camera,
    character::{Character, SkinType},
    renderer::{OutputFormat, Renderer},
    utils::converter,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Minecraft皮肤工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
enum SkinTypeCli {
    Classic,
    Slim,
}

impl From<SkinTypeCli> for SkinType {
    fn from(value: SkinTypeCli) -> Self {
        match value {
            SkinTypeCli::Classic => SkinType::Classic,
            SkinTypeCli::Slim => SkinType::Slim,
        }
    }
}

#[derive(Parser, Debug)]
struct ViewportArgs {
    /// 图片或窗口宽度
    #[arg(long, default_value_t = 800, value_parser = clap::value_parser!(u32).range(1..))]
    width: u32,
    /// 图片或窗口高度
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u32).range(1..))]
    height: u32,
}

#[derive(Parser, Debug)]
struct SceneArgs {
    /// PNG材质文件路径
    #[arg(long, default_value = "resources/bingling_sama.png")]
    texture: String,

    /// 皮肤类型，`classic` 或 `slim`
    #[arg(long, value_enum)]
    skin_type: SkinTypeCli,

    /// 摄像机视角绕角色旋转角度（XZ 平面绕 Y 轴旋转），0~360，0 是正前，90 是正右，180 是正后，270 是正左
    #[arg(long, default_value_t = 180.0)]
    yaw: f32,

    /// 摄像机视角绕角色俯仰角度（YZ 平面绕 X 轴旋转），0~180，90 是正前，0 是脚下，180 是头顶
    #[arg(long, default_value_t = 90.0)]
    pitch: f32,

    /// 缩放比例，>=0
    #[arg(long, default_value_t = 1.0)]
    scale: f32,

    /// 角色头部摇头角度（XZ 平面绕 Y 轴旋转），0~180，90 是正前，0 是正左，180 是正右
    #[arg(long, default_value_t = 90.0)]
    head_yaw: f32,
    /// 角色头部俯仰角度（YZ 平面绕 X 轴旋转），0~180，90 是正前，0 是垂直向下看，180 是垂直向上看
    #[arg(long, default_value_t = 90.0)]
    head_pitch: f32,
    /// 左手侧举角度（XY 平面绕 Z 轴旋转），0~180，90 是向右侧平举，0 是垂直向下，180 是垂直向上抬起
    #[arg(long, default_value_t = 90.0)]
    left_arm_roll: f32,
    /// 左手摆臂角度（YZ 平面绕 X 轴旋转），0~360，0 是垂直向下，90 是水平前摆，180 是垂直向上，270 是水平向后
    #[arg(long, default_value_t = 0.0)]
    left_arm_pitch: f32,
    /// 右手侧举角度（XY 平面绕 Z 轴旋转），0~180，90 是向右侧平举，0 是垂直向下，180 是垂直向上抬起
    #[arg(long, default_value_t = 90.0)]
    right_arm_roll: f32,
    /// 右手摆臂角度（YZ 平面绕 X 轴旋转），0~360，0 是垂直向下，90 是水平前摆，180 是垂直向上，270 是水平向后
    #[arg(long, default_value_t = 0.0)]
    right_arm_pitch: f32,
    /// 左腿抬腿角度（YZ 平面绕 X 轴旋转），0~180，90 是垂直于地面，0 是水平前摆，180 是水平后摆
    #[arg(long, default_value_t = 90.0)]
    left_leg_pitch: f32,
    /// 右腿抬腿角度（YZ 平面绕 X 轴旋转），0~180，90 是垂直于地面，0 是水平前摆，180 是水平后摆
    #[arg(long, default_value_t = 90.0)]
    right_leg_pitch: f32,

    /// 角色位置 X 坐标
    #[arg(long, default_value_t = 0.0)]
    position_x: f32,
    /// 角色位置 Y 坐标
    #[arg(long, default_value_t = 0.0)]
    position_y: f32,
    /// 角色位置 Z 坐标
    #[arg(long, default_value_t = 0.0)]
    position_z: f32,

    /// 角色旋轉 X（度）
    #[arg(long, default_value_t = 0.0)]
    rotation_x: f32,
    /// 角色旋轉 Y（度）
    #[arg(long, default_value_t = 0.0)]
    rotation_y: f32,
    /// 角色旋轉 Z（度）
    #[arg(long, default_value_t = 0.0)]
    rotation_z: f32,
}

fn character_and_camera_from_scene(scene: &SceneArgs) -> (Character, Camera) {
    let mut character = Character::new();
    character.skin_type = scene.skin_type.into();
    character.posture.head_yaw = scene.head_yaw;
    character.posture.head_pitch = scene.head_pitch;
    character.posture.left_arm_roll = scene.left_arm_roll;
    character.posture.left_arm_pitch = scene.left_arm_pitch;
    character.posture.right_arm_roll = scene.right_arm_roll;
    character.posture.right_arm_pitch = scene.right_arm_pitch;
    character.posture.left_leg_pitch = scene.left_leg_pitch;
    character.posture.right_leg_pitch = scene.right_leg_pitch;
    character.position = cgmath::Vector3::new(scene.position_x, scene.position_y, scene.position_z);
    character.rotation = cgmath::Vector3::new(scene.rotation_x, scene.rotation_y, scene.rotation_z);

    let camera = Camera {
        yaw: scene.yaw,
        pitch: scene.pitch,
        scale: scene.scale,
    };
    (character, camera)
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 渲染皮肤为3D图像
    Render {
        /// 输出图片文件名
        #[arg(long, default_value = "output.png")]
        filename: String,

        /// 输出图片格式，png 或 webp，默认 png
        #[arg(long, default_value = "png")]
        format: String,

        #[command(flatten)]
        viewport: ViewportArgs,

        #[command(flatten)]
        scene: SceneArgs,
    },
    /// 在窗口中预览皮肤
    Preview {
        #[command(flatten)]
        viewport: ViewportArgs,

        #[command(flatten)]
        scene: SceneArgs,
    },
    /// 将单层皮肤转换为双层皮肤
    Convert {
        /// 输入的单层皮肤图片文件路径
        input: PathBuf,
        /// 转换后的双层皮肤图片输出路径
        #[arg(default_value = "output.png")]
        output: PathBuf,
    },
}

struct PreviewApp {
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    character: Character,
    camera: Camera,
    texture_path: String,
    initial_size: PhysicalSize<u32>,
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Eidolon Preview")
            .with_inner_size(self.initial_size);
        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        let mut renderer = Renderer::new_windowed(window.clone());
        self.character.skin = Some(renderer.load_texture(&self.texture_path).unwrap());
        let size = window.inner_size();
        renderer.resize(size.width, size.height);

        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &self.renderer {
                    match renderer.render_frame(&self.character, &self.camera) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                if let Some(r) = &mut self.renderer {
                                    r.resize(size.width, size.height);
                                }
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            event_loop.exit();
                        }
                        Err(e) => log::error!("Render error: {:?}", e),
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    use log::{error, info};
    match args.command {
        Command::Render {
            filename,
            format,
            viewport,
            scene,
        } => {
            info!("Minecraft皮肤渲染器");
            info!("文件名: {}", filename);
            info!("尺寸: {}x{}", viewport.width, viewport.height);
            info!("材质文件: {}", scene.texture);

            info!("正在创建渲染器...");
            let renderer = Renderer::new();
            info!("渲染器创建成功");

            let (mut character, camera) = character_and_camera_from_scene(&scene);

            info!("正在加载皮肤文件: {}", scene.texture);
            character.skin = Some(renderer.load_texture(&scene.texture)?);
            info!("皮肤文件加载成功");

            info!("正在渲染图片...");

            let output_format = match format.to_lowercase().as_str() {
                "png" => OutputFormat::Png,
                "webp" => OutputFormat::WebP,
                other => {
                    error!("不支持的输出格式: {}，仅支持 png 或 webp", other);
                    return Err(Box::from("不支持的输出格式"));
                }
            };

            let mut filename = filename;
            if filename == "output.png" {
                filename = match format.to_lowercase().as_str() {
                    "png" => "output.png".to_string(),
                    "webp" => "output.webp".to_string(),
                    _ => filename,
                };
            }

            renderer.render_to_image(
                &character,
                &camera,
                &filename,
                (viewport.width, viewport.height),
                output_format,
            )?;
            info!("渲染完成！图片已保存到: {}", filename);

            Ok(())
        }
        Command::Preview { viewport, scene } => {
            let (character, camera) = character_and_camera_from_scene(&scene);

            let event_loop = EventLoop::new()?;
            let mut app = PreviewApp {
                renderer: None,
                window: None,
                character,
                camera,
                texture_path: scene.texture,
                initial_size: PhysicalSize::new(viewport.width, viewport.height),
            };
            event_loop.run_app(&mut app)?;

            Ok(())
        }
        Command::Convert { input, output } => {
            let img =
                image::open(input).map_err(|e| format!("Failed to open input image: {}", e))?;

            match converter::single2double(&img) {
                Ok(result) => {
                    info!("转换成功！双层皮肤已保存到: {:?}", output);
                    result
                        .save(output)
                        .map_err(|e| format!("Failed to save output image: {}", e))?;
                    Ok(())
                }
                Err(e) => {
                    error!("转换失败: {}", e);
                    Err(Box::new(std::io::Error::other(e)))
                }
            }
        }
    }
}
