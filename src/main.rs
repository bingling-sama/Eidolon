use clap::{Parser, Subcommand};
use eidolon::{
    camera::Camera,
    character::{Character, SkinType},
    renderer::Renderer,
};
use std::path::PathBuf;

mod converter;

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

        /// 皮肤类型
        #[arg(long, value_enum, default_value_t = SkinType::Slim)]
        skin_type: SkinType,

        /// 摄像机Yaw
        #[arg(long, default_value_t = 180.0)]
        yaw: f32,

        /// 摄像机Pitch
        #[arg(long, default_value_t = 90.0)]
        pitch: f32,

        /// 摄像机Scale
        #[arg(long, default_value_t = 1.0)]
        scale: f32,

        /// 角色头部摇头角度
        #[arg(long, default_value_t = 90.0)]
        head_yaw: f32,
        /// 角色头部俯仰角度
        #[arg(long, default_value_t = 90.0)]
        head_pitch: f32,
        /// 左手侧举角度
        #[arg(long, default_value_t = 90.0)]
        left_arm_roll: f32,
        /// 左手摆臂角度
        #[arg(long, default_value_t = 0.0)]
        left_arm_pitch: f32,
        /// 右手侧举角度
        #[arg(long, default_value_t = 90.0)]
        right_arm_roll: f32,
        /// 右手摆臂角度
        #[arg(long, default_value_t = 0.0)]
        right_arm_pitch: f32,
        /// 左腿抬腿角度
        #[arg(long, default_value_t = 90.0)]
        left_leg_pitch: f32,
        /// 右腿抬腿角度
        #[arg(long, default_value_t = 90.0)]
        right_leg_pitch: f32,
    },
    /// 将单层皮肤转换为双层皮肤
    Convert {
        /// 输入的单层皮肤图片文件路径
        input: PathBuf,
        /// 转换后的双层皮肤图片输出路径
        output: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Command::Render {
            filename,
            width,
            height,
            texture,
            skin_type,
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
        } => {
            println!("Minecraft皮肤渲染器");
            println!("文件名: {}", filename);
            println!("尺寸: {}x{}", width, height);
            println!("材质文件: {}", texture);

            // 创建渲染器
            println!("正在创建渲染器...");
            let renderer = Renderer::new();
            println!("渲染器创建成功");

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

            // 设置皮肤文件
            println!("正在加载皮肤文件: {}", texture);
            character.load_skin_from_file(&texture, renderer.get_display())?;
            println!("皮肤文件加载成功");

            // 渲染并保存图片
            println!("正在渲染图片...");
            renderer.render_to_image(&character, &camera, &filename, (width, height))?;
            println!("渲染完成！图片已保存到: {}", filename);

            Ok(())
        }
        Command::Convert { input, output } => {
            match converter::convert_to_double_layer(&input, &output) {
                Ok(_) => {
                    println!("转换成功！双层皮肤已保存到: {:?}", output);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("转换失败: {}", e);
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))
                }
            }
        }
    }
}
