use clap::Parser;
use skinviewer::{camera::Camera, character::Character, renderer::Renderer};

/// Minecraft皮肤渲染器
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 输出图片文件名
    #[arg(default_value = "output.png")]
    filename: String,

    /// 图片宽度
    #[arg(long, default_value_t = 800)]
    width: u32,

    /// 图片高度
    #[arg(long, default_value_t = 600)]
    height: u32,

    /// PNG材质文件路径
    #[arg(long, default_value = "resources/player.png")]
    texture: String,

    /// 摄像机Yaw
    #[arg(long, default_value_t = 20.0)]
    yaw: f32,

    /// 摄像机Pitch
    #[arg(long, default_value_t = 20.0)]
    pitch: f32,

    /// 摄像机Scale
    #[arg(long, default_value_t = 1.0)]
    scale: f32,
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
