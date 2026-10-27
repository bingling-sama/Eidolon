use criterion::{criterion_group, criterion_main, Criterion};
use eidolon_core::{
    animation::AnimationPreset,
    camera::Camera,
    character::{Character, SkinType},
    output::{write_output, AnimationFormat, OutputIntent, StaticFormat},
    renderer::Renderer,
};
use std::fs;
use std::time::Duration;

fn performance_benchmark(c: &mut Criterion) {
    let output_dir = ".bench";
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    let renderer = Renderer::new().expect("Failed to create renderer");
    let body = renderer
        .load_body(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("resources")
                .join("bingling_sama.png")
                .to_string_lossy(),
            SkinType::Classic,
        )
        .expect("Failed to load body");
    let mut character = Character::from_body(body);

    let mut camera = Camera {
        yaw: 180.0,
        pitch: 80.0,
        scale: 1.0,
    };

    let num_images = 20;

    c.bench_function("render_20_images", |b| {
        b.iter(|| {
            for i in 1..=num_images {
                camera.yaw = 180.0 + (i as f32) * 5.0;
                camera.pitch = 80.0 - (i as f32) * 2.0;

                let filename = format!("{}/output_{}.png", output_dir, i);

                write_output(
                    &filename,
                    OutputIntent::Static {
                        format: StaticFormat::Png,
                        size: (800, 600),
                    },
                    |_: Duration| renderer.render(&character, &camera, 800, 600),
                )
                .unwrap();
            }
        })
    });

    c.bench_function("export_walk_gif_800x600_30fps", |b| {
        b.iter(|| {
            write_output(
                format!("{output_dir}/walk.gif"),
                OutputIntent::Animation {
                    format: AnimationFormat::Gif,
                    preset: AnimationPreset::Walk,
                    fps: 30,
                    speed: 1.0,
                    loop_count: 0,
                    size: (800, 600),
                },
                |time| {
                    character.posture = AnimationPreset::Walk.sample(time);
                    renderer.render(&character, &camera, 800, 600)
                },
            )
            .unwrap();
        })
    });
}

criterion_group!(benches, performance_benchmark);
criterion_main!(benches);
