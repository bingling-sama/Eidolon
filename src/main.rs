use clap::{Parser, Subcommand};
use eidolon::{
    camera::Camera,
    character::{Character, SkinType},
    renderer::Renderer,
};
use std::path::PathBuf;

mod utils;
use utils::converter;

/// Minecraft皮肤工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 渲染皮肤为3D图像
    Render {
        /// 输出图片文件名
        #[arg(long, default_value = "output.png")]
        filename: String,

        /// 图片宽度
        #[arg(long, default_value_t = 800)]
        width: u32,

        /// 图片高度
        #[arg(long, default_value_t = 600)]
        height: u32,

        /// PNG材质文件路径
        #[arg(long, default_value = "resources/bingling_sama.png")]
        texture: String,

        /// 皮肤类型，`classic` 或 `slim`
        #[arg(long, value_enum)]
        skin_type: SkinType,

        /// 输出图片格式，png 或 webp，默认 png
        #[arg(long, default_value = "png")]
        format: String,

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    use log::{info, error};
    match args.command {
        Command::Render {
            filename,
            width,
            height,
            texture,
            skin_type,
            format,
            yaw,
            pitch,
            scale,
            head_yaw,
            head_pitch,
            left_arm_roll,
            left_arm_pitch,
            right_arm_roll,
            right_arm_pitch,
            left_leg_pitch,
            right_leg_pitch,
            position_x,
            position_y,
            position_z,
            rotation_x,
            rotation_y,
            rotation_z,
        } => {
            info!("Minecraft皮肤渲染器");
            info!("文件名: {}", filename);
            info!("尺寸: {}x{}", width, height);
            info!("材质文件: {}", texture);

            // 创建渲染器
            info!("正在创建渲染器...");
            let renderer = Renderer::new();
            info!("渲染器创建成功");

            // 创建角色和相机
            let mut character = Character::new();
            character.skin_type = skin_type;
            let camera = Camera { yaw, pitch, scale };

            // 设置角色姿势
            character.posture.head_yaw = head_yaw;
            character.posture.head_pitch = head_pitch;
            character.posture.left_arm_roll = left_arm_roll;
            character.posture.left_arm_pitch = left_arm_pitch;
            character.posture.right_arm_roll = right_arm_roll;
            character.posture.right_arm_pitch = right_arm_pitch;
            character.posture.left_leg_pitch = left_leg_pitch;
            character.posture.right_leg_pitch = right_leg_pitch;

            // 设置角色位置
            character.position = cgmath::Vector3::new(position_x, position_y, position_z);

            // 设置角色旋轉
            character.rotation = cgmath::Vector3::new(rotation_x, rotation_y, rotation_z);

            // 设置皮肤文件
            info!("正在加载皮肤文件: {}", texture);
            character.load_skin_from_file(&texture, renderer.get_display())?;
            info!("皮肤文件加载成功");

            // 渲染并保存图片
            info!("正在渲染图片...");

            // 解析输出格式
            let output_format = match format.to_lowercase().as_str() {
                "png" => eidolon::renderer::OutputFormat::Png,
                "webp" => eidolon::renderer::OutputFormat::WebP,
                other => {
                    error!("不支持的输出格式: {}，仅支持 png 或 webp", other);
                    return Err(Box::from("不支持的输出格式"));
                }
            };

            // 自动调整默认文件名后缀
            let mut filename = filename;
            if filename == "output.png" {
                filename = match format.to_lowercase().as_str() {
                    "png" => "output.png".to_string(),
                    "webp" => "output.webp".to_string(),
                    _ => filename,
                };
            }

            renderer.render_to_image(&character, &camera, &filename, (width, height), output_format)?;
            info!("渲染完成！图片已保存到: {}", filename);

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
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))
                }
            }
        }
    }
}
