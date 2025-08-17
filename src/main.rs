use clap::Parser;
use eidolon::{camera::Camera, character::Character, renderer::Renderer};

/// Minecraft皮肤渲染器
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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
    #[arg(long, default_value = "resources/slim.png")]
    texture: String,

    /// 摄像机Yaw
    #[arg(long, default_value_t = 180.0)]
    yaw: f32,

    /// 摄像机Pitch
    #[arg(long, default_value_t = 80.0)]
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Minecraft皮肤渲染器");
    println!("文件名: {}", args.filename);
    println!("尺寸: {}x{}", args.width, args.height);
    println!("材质文件: {}", args.texture);

    // 创建渲染器
    println!("正在创建渲染器...");
    let renderer = Renderer::new();
    println!("渲染器创建成功");

    // 创建角色和相机
    let mut character = Character::new();
    let camera = Camera {
        yaw: args.yaw,
        pitch: args.pitch,
        scale: args.scale,
    };

    // 设置角色姿势
    character.posture.head_yaw = args.head_yaw;
    character.posture.head_pitch = args.head_pitch;
    character.posture.left_arm_roll = args.left_arm_roll;
    character.posture.left_arm_pitch = args.left_arm_pitch;
    character.posture.right_arm_roll = args.right_arm_roll;
    character.posture.right_arm_pitch = args.right_arm_pitch;
    character.posture.left_leg_pitch = args.left_leg_pitch;
    character.posture.right_leg_pitch = args.right_leg_pitch;

    // 设置皮肤文件
    println!("正在加载皮肤文件: {}", args.texture);
    character.load_skin_from_file(&args.texture, renderer.get_display())?;
    println!("皮肤文件加载成功");

    // 渲染并保存图片
    println!("正在渲染图片...");
    renderer.render_to_image(
        &character,
        &camera,
        &args.filename,
        (args.width, args.height),
    )?;
    println!("渲染完成！图片已保存到: {}", args.filename);

    Ok(())
}
