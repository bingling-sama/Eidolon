use criterion::{criterion_group, criterion_main, Criterion};
use eidolon::{
    animation::AnimationPreset,
    camera::Camera,
    character::{Character, SkinType},
    renderer::Renderer,
    OutputFormat,
};
use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat};
use std::fs;
use std::time::Duration;

fn performance_benchmark(c: &mut Criterion) {
    let output_dir = ".bench";
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    let renderer = Renderer::new().expect("Failed to create renderer");
    let body = renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
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

                renderer
                    .render_to_image(
                        &character,
                        &camera,
                        &filename,
                        (800, 600),
                        OutputFormat::Png,
                    )
                    .unwrap();
            }
        })
    });

    c.bench_function("export_walk_gif_800x600_30fps", |b| {
        b.iter(|| {
            let mut encoder = GifEncoder::new(std::io::sink(), 800, 600, &[]).unwrap();
            encoder.set_repeat(Repeat::Infinite).unwrap();
            for frame_index in 0..30 {
                character.posture = AnimationPreset::Walk
                    .sample(Duration::from_secs_f64(f64::from(frame_index) / 30.0));
                let mut rgba = renderer
                    .render(&character, &camera, 800, 600)
                    .unwrap()
                    .into_raw();
                let mut frame = GifFrame::from_rgba_speed(800, 600, &mut rgba, 10);
                frame.delay = ((frame_index + 1) * 100 / 30 - frame_index * 100 / 30) as u16;
                encoder.write_frame(&frame).unwrap();
            }
        })
    });
}

criterion_group!(benches, performance_benchmark);
criterion_main!(benches);
