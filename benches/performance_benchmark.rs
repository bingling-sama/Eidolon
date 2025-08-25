use criterion::{criterion_group, criterion_main, Criterion};
use eidolon::{camera::Camera, character::Character, renderer::{Renderer, OutputFormat}};
use std::fs;

fn performance_benchmark(c: &mut Criterion) {
    // 创建输出目录
    let output_dir = "performance_test_output_bench";
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    // 设置渲染器、角色和相机
    let renderer = Renderer::new();
    let mut character = Character::new();
    character.load_skin_from_file("resources/slim.png", renderer.get_display()).unwrap();
    
    let mut camera = Camera {
        yaw: 180.0,
        pitch: 80.0,
        scale: 1.0,
    };

    let num_images = 20;

    c.bench_function("render_20_images", |b| {
        b.iter(|| {
            for i in 1..=num_images {
                // 为每次运行生成略微不同的参数
                camera.yaw = 180.0 + (i as f32) * 5.0;
                camera.pitch = 80.0 - (i as f32) * 2.0;

                let filename = format!("{}/output_{}.png", output_dir, i);
                
                // 渲染图片
                renderer
                    .render_to_image(&character, &camera, &filename, (800, 600), OutputFormat::Png)
                    .unwrap();
            }
        })
    });
}

criterion_group!(benches, performance_benchmark);
criterion_main!(benches);
