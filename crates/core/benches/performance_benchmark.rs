use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use eidolon_core::{
    animation::AnimationPreset,
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, SkinType},
    classifier::{inspect_image, inspect_texture},
    converter::single2double,
    output::{encode_static_image, write_output, AnimationFormat, OutputIntent, StaticFormat},
    renderer::Renderer,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

// Duplicates `crates/core/tests/common/mod.rs` because benchmarks cannot import test-only helpers.
fn resource(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("resources")
        .join(name)
}

fn resource_str(name: &str) -> String {
    resource(name).to_string_lossy().into_owned()
}

static SHARED_RENDERER: OnceLock<Renderer> = OnceLock::new();

fn shared_renderer() -> &'static Renderer {
    SHARED_RENDERER.get_or_init(|| Renderer::new().expect("Failed to create renderer"))
}

/// RAII guard ensuring scratch directories are deleted on normal completion or on panic.
struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{prefix}_{}", std::process::id()));
        fs::create_dir_all(&path).expect("Failed to create temporary benchmark directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct TrackingAllocator {
    tracking: AtomicBool,
    allocated_bytes: AtomicUsize,
    alloc_count: AtomicUsize,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            tracking: AtomicBool::new(false),
            allocated_bytes: AtomicUsize::new(0),
            alloc_count: AtomicUsize::new(0),
        }
    }

    fn start_tracking(&self) {
        self.allocated_bytes.store(0, Ordering::SeqCst);
        self.alloc_count.store(0, Ordering::SeqCst);
        self.tracking.store(true, Ordering::SeqCst);
    }

    fn stop_tracking(&self) -> (usize, usize) {
        self.tracking.store(false, Ordering::SeqCst);
        (
            self.alloc_count.load(Ordering::SeqCst),
            self.allocated_bytes.load(Ordering::SeqCst),
        )
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() && self.tracking.load(Ordering::Relaxed) {
            self.allocated_bytes
                .fetch_add(layout.size(), Ordering::Relaxed);
            self.alloc_count.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() && self.tracking.load(Ordering::Relaxed) {
            self.allocated_bytes
                .fetch_add(layout.size(), Ordering::Relaxed);
            self.alloc_count.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() && self.tracking.load(Ordering::Relaxed) {
            if new_size > layout.size() {
                self.allocated_bytes
                    .fetch_add(new_size - layout.size(), Ordering::Relaxed);
            }
            self.alloc_count.fetch_add(1, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

/// Measures end-to-end `renderer.render()` latency across standard resolutions.
/// Note: includes GPU execution, synchronous readback, and output buffer allocation.
fn bench_render_resolutions(c: &mut Criterion) {
    let renderer = shared_renderer();
    let body = renderer
        .load_body(&resource_str("bingling_sama.png"), SkinType::Classic)
        .expect("Failed to load body");
    let character = Character::from_body(body);
    let camera = Camera {
        yaw: 180.0,
        pitch: 80.0,
        scale: 1.0,
        target: [0.0, 1.0, 0.0],
    };

    let resolutions = [(128, 128), (400, 300), (800, 600), (1920, 1080)];

    let mut group = c.benchmark_group("render_resolution");
    // Reduce sample size to 20 so 1080p finishes reliably on software rasterizers (e.g. llvmpipe).
    group.sample_size(20);
    for &(w, h) in &resolutions {
        group.bench_with_input(
            BenchmarkId::new("render", format!("{w}x{h}")),
            &(w, h),
            |b, &(width, height)| {
                b.iter(|| {
                    renderer
                        .render(&character, &camera, width, height)
                        .expect("Render failed")
                });
            },
        );
    }
    group.finish();
}

fn bench_render_scenes(c: &mut Criterion) {
    let renderer = shared_renderer();
    let classic_body = renderer
        .load_body(&resource_str("bingling_sama.png"), SkinType::Classic)
        .expect("Failed to load classic body");
    let slim_body = renderer
        .load_body(&resource_str("undefinedR2.png"), SkinType::Slim)
        .expect("Failed to load slim body");
    let atlas = renderer
        .load_back_equipment_atlas(&resource_str("Turtle_Cape.png"))
        .expect("Failed to load cape atlas");

    let camera = Camera {
        yaw: 180.0,
        pitch: 80.0,
        scale: 1.0,
        target: [0.0, 1.0, 0.0],
    };

    let char_classic = Character::from_body(classic_body.clone());
    let char_slim = Character::from_body(slim_body);

    let mut char_cape = Character::from_body(classic_body.clone());
    char_cape.equip(BackEquipment::new(atlas.clone(), BackEquipmentKind::Cape));

    let mut char_elytra = Character::from_body(classic_body.clone());
    char_elytra.equip(BackEquipment::new(atlas.clone(), BackEquipmentKind::Elytra));

    let mut char_nametag = Character::from_body(classic_body);
    char_nametag.nametag = Some("Steve".to_string());

    let char_standalone_cape =
        Character::from_back_equipment(BackEquipment::new(atlas.clone(), BackEquipmentKind::Cape));
    let char_standalone_elytra =
        Character::from_back_equipment(BackEquipment::new(atlas, BackEquipmentKind::Elytra));

    let scenes = [
        ("classic_body", &char_classic),
        ("slim_body", &char_slim),
        ("with_cape", &char_cape),
        ("with_elytra", &char_elytra),
        ("with_nametag", &char_nametag),
        ("standalone_cape", &char_standalone_cape),
        ("standalone_elytra", &char_standalone_elytra),
    ];

    let mut group = c.benchmark_group("render_scenes_800x600");
    for (name, ch) in scenes {
        group.bench_function(name, |b| {
            b.iter(|| {
                renderer
                    .render(ch, &camera, 800, 600)
                    .expect("Render failed")
            });
        });
    }
    group.finish();
}

fn bench_asset_loading(c: &mut Criterion) {
    let renderer = Renderer::new().expect("Failed to create renderer");
    let skin_bytes = fs::read(resource("bingling_sama.png")).expect("Failed to read skin");
    let slim_bytes = fs::read(resource("undefinedR2.png")).expect("Failed to read slim skin");
    let cape_bytes = fs::read(resource("Turtle_Cape.png")).expect("Failed to read cape");

    let mut group = c.benchmark_group("asset_loading");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(1));
    group.bench_function("load_body_classic", |b| {
        b.iter(|| {
            renderer
                .load_body_from_memory(&skin_bytes, SkinType::Classic)
                .expect("Failed to load classic body")
        });
    });
    group.bench_function("load_body_slim", |b| {
        b.iter(|| {
            renderer
                .load_body_from_memory(&slim_bytes, SkinType::Slim)
                .expect("Failed to load slim body")
        });
    });
    group.bench_function("load_cape_atlas", |b| {
        b.iter(|| {
            renderer
                .load_back_equipment_atlas_from_memory(&cape_bytes)
                .expect("Failed to load cape atlas")
        });
    });
    group.finish();
}

fn bench_image_processing(c: &mut Criterion) {
    let skin_bytes = fs::read(resource("bingling_sama.png")).expect("Failed to read skin");
    let skin_dyn_image =
        image::load_from_memory(&skin_bytes).expect("Failed to decode skin dynamic image");

    let legacy_bytes = fs::read(resource("SSSSSteven.png")).expect("Failed to read legacy skin");
    let legacy_dyn_image =
        image::load_from_memory(&legacy_bytes).expect("Failed to decode legacy dynamic image");

    let mut group = c.benchmark_group("image_processing");
    group.bench_function("inspect_texture_bytes", |b| {
        b.iter(|| inspect_texture(&skin_bytes).expect("Failed to inspect texture"));
    });
    group.bench_function("inspect_image_dynamic", |b| {
        b.iter(|| inspect_image(&skin_dyn_image));
    });
    group.bench_function("single2double_conversion", |b| {
        b.iter(|| single2double(&legacy_dyn_image).expect("Failed to convert legacy skin"));
    });
    group.finish();
}

fn bench_animation_and_pose(c: &mut Criterion) {
    let duration = Duration::from_millis(500);

    let mut group = c.benchmark_group("animation_sampling");
    group.bench_function("walk_preset", |b| {
        b.iter(|| AnimationPreset::Walk.sample(duration));
    });
    group.bench_function("run_preset", |b| {
        b.iter(|| AnimationPreset::Run.sample(duration));
    });
    group.bench_function("wave_preset", |b| {
        b.iter(|| AnimationPreset::Wave.sample(duration));
    });
    group.bench_function("fly_preset", |b| {
        b.iter(|| AnimationPreset::Fly.sample(duration));
    });
    group.finish();
}

fn bench_encoding_static(c: &mut Criterion) {
    let renderer = shared_renderer();
    let body = renderer
        .load_body(&resource_str("bingling_sama.png"), SkinType::Classic)
        .expect("Failed to load body");
    let character = Character::from_body(body);
    let camera = Camera {
        yaw: 180.0,
        pitch: 80.0,
        scale: 1.0,
        target: [0.0, 1.0, 0.0],
    };
    let frame = renderer
        .render(&character, &camera, 800, 600)
        .expect("Render failed");

    let mut group = c.benchmark_group("encoding_static_800x600");
    group.bench_function("png", |b| {
        b.iter(|| encode_static_image(&frame, StaticFormat::Png).expect("Encode PNG failed"));
    });
    group.bench_function("webp", |b| {
        b.iter(|| encode_static_image(&frame, StaticFormat::WebP).expect("Encode WebP failed"));
    });
    group.finish();
}

fn bench_animation_export(c: &mut Criterion) {
    let renderer = shared_renderer();
    let body = renderer
        .load_body(&resource_str("bingling_sama.png"), SkinType::Classic)
        .expect("Failed to load body");
    let character = Character::from_body(body);
    let camera = Camera {
        yaw: 180.0,
        pitch: 80.0,
        scale: 1.0,
        target: [0.0, 1.0, 0.0],
    };

    let temp_dir = TempDirGuard::new("eidolon_bench");

    let formats = [
        (
            "gif",
            AnimationFormat::Gif,
            temp_dir.path().join("walk.gif"),
        ),
        (
            "apng",
            AnimationFormat::Apng,
            temp_dir.path().join("walk.apng"),
        ),
        (
            "webp",
            AnimationFormat::WebP,
            temp_dir.path().join("walk.webp"),
        ),
        (
            "avif",
            AnimationFormat::Avif,
            temp_dir.path().join("walk.avif"),
        ),
    ];

    let mut group = c.benchmark_group("export_animation_walk_400x300_15fps");
    group.sample_size(10);

    for (name, format, path) in formats {
        let mut char_export = character.clone();
        group.bench_function(name, |b| {
            b.iter(|| {
                write_output(
                    &path,
                    OutputIntent::Animation {
                        format,
                        preset: AnimationPreset::Walk,
                        fps: 15,
                        speed: 1.0,
                        loop_count: 0,
                        size: (400, 300),
                    },
                    |time| {
                        char_export.posture = AnimationPreset::Walk.sample(time);
                        renderer.render(&char_export, &camera, 400, 300)
                    },
                )
                .expect("Failed to export walk animation");
            });
        });
    }

    group.finish();
}

/// Micro-benchmark, memory quantification, and end-to-end comparison for AVIF export (Issue #24).
///
/// 1. Memory Quantification & Micro-benchmark (`avif_frame_prep_800x600`):
///    Measures isolated per-frame buffer preparation overhead and heap allocations.
///    Zero-copy slice borrowing (`as_rgba`) eliminates the scalar chunk loop and
///    achieves 0 heap allocations / 0 bytes, eliminating ~1.92 MB per 800x600 frame.
///
/// 2. End-to-end export comparison (`export_walk_avif_comparison_400x300_15fps`):
///    Measures end-to-end animation export pipeline (render + prep + encode).
///    Both pipelines use strictly identical frame durations via cumulative rounding
///    (`((boundary * 1000 + fps / 2) / fps)`) for fair A/B comparison.
///    Demonstrates that while buffer preparation is nearly instant, overall export
///    duration is dominated by the underlying AV1 video encoder (zenrav1e).
fn bench_avif_export(c: &mut Criterion) {
    let renderer = shared_renderer();
    let body = renderer
        .load_body(&resource_str("bingling_sama.png"), SkinType::Classic)
        .expect("Failed to load body");
    let character = Character::from_body(body);
    let camera = Camera {
        yaw: 180.0,
        pitch: 80.0,
        scale: 1.0,
        target: [0.0, 1.0, 0.0],
    };

    // 1. Isolated single-frame buffer preparation micro-benchmark (800x600)
    let sample_img = renderer.render(&character, &camera, 800, 600).unwrap();
    let sample_raw = sample_img.into_raw();

    // Programmatic memory allocation quantification (Issue #24)
    ALLOCATOR.start_tracking();
    {
        use rgb::FromSlice;
        let rgba_slice: &[zenravif::RGBA8] = sample_raw.as_slice().as_rgba();
        std::hint::black_box(rgba_slice);
    }
    let (zc_allocs, zc_bytes) = ALLOCATOR.stop_tracking();

    ALLOCATOR.start_tracking();
    {
        use zenravif::RGBA8;
        let rgba_vec: Vec<RGBA8> = sample_raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
            .collect();
        std::hint::black_box(rgba_vec);
    }
    let (legacy_allocs, legacy_bytes) = ALLOCATOR.stop_tracking();

    eprintln!(
        "\n======================================================================\n\
         [AVIF Memory Benchmark] Buffer Allocation Quantification (Issue #24)\n\
         ----------------------------------------------------------------------\n\
         Single-Frame Prep (800x600):\n\
           - zerocopy_slice_borrow:  {} allocs, {} bytes\n\
           - legacy_chunk_alloc_vec: {} allocs, {} bytes ({:.2} MiB)\n\
           - Net Reduction:          {} bytes (100.0% eliminated)\n\
         ======================================================================\n",
        zc_allocs,
        zc_bytes,
        legacy_allocs,
        legacy_bytes,
        legacy_bytes as f64 / (1024.0 * 1024.0),
        legacy_bytes.saturating_sub(zc_bytes),
    );

    let mut prep_group = c.benchmark_group("avif_frame_prep_800x600");
    prep_group.sample_size(10);

    prep_group.bench_function("zerocopy_slice_borrow", |b| {
        use rgb::FromSlice;
        b.iter(|| {
            let rgba_slice: &[zenravif::RGBA8] = sample_raw.as_slice().as_rgba();
            std::hint::black_box(rgba_slice);
        });
    });

    prep_group.bench_function("legacy_chunk_alloc_vec", |b| {
        use zenravif::RGBA8;
        b.iter(|| {
            let rgba_vec: Vec<RGBA8> = sample_raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|chunk| RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
                .collect();
            std::hint::black_box(rgba_vec);
        });
    });

    prep_group.finish();

    // 2. End-to-end comparison (400x300, 15fps)
    let temp_dir = TempDirGuard::new("eidolon_bench_avif");
    let path_zerocopy = temp_dir.path().join("walk_zerocopy.avif");
    let path_legacy = temp_dir.path().join("walk_legacy.avif");

    let mut e2e_group = c.benchmark_group("export_walk_avif_comparison_400x300_15fps");
    e2e_group.sample_size(10);

    let mut char_zerocopy = character.clone();
    e2e_group.bench_function("zerocopy_pipeline", |b| {
        b.iter(|| {
            write_output(
                &path_zerocopy,
                OutputIntent::Animation {
                    format: AnimationFormat::Avif,
                    preset: AnimationPreset::Walk,
                    fps: 15,
                    speed: 1.0,
                    loop_count: 0,
                    size: (400, 300),
                },
                |time| {
                    char_zerocopy.posture = AnimationPreset::Walk.sample(time);
                    renderer.render(&char_zerocopy, &camera, 400, 300)
                },
            )
            .expect("Failed to export walk AVIF animation (zerocopy)");
        });
    });

    let mut char_legacy = character.clone();
    e2e_group.bench_function("legacy_allocation_pipeline", |b| {
        use eidolon_core::animation::AnimationSchedule;
        use zenravif::{AnimFrameRgba, Encoder as AvifEncoder, Img, RGBA8};

        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 15, 1.0).unwrap();
        b.iter(|| {
            let frame_count = schedule.frame_count() as usize;
            let mut frames_data: Vec<Vec<RGBA8>> = Vec::with_capacity(frame_count);
            let mut frame_durations: Vec<u32> = Vec::with_capacity(frame_count);

            for frame_index in 0..schedule.frame_count() {
                char_legacy.posture =
                    AnimationPreset::Walk.sample(schedule.sample_time(frame_index));
                let img = renderer.render(&char_legacy, &camera, 400, 300).unwrap();
                // Match the cumulative rounding timestamp logic from production write_output
                let fps = schedule.fps();
                let duration_ms = ((frame_index + 1) * 1_000 + fps / 2) / fps
                    - (frame_index * 1_000 + fps / 2) / fps;
                frame_durations.push(duration_ms);

                let raw = img.into_raw();
                let rgba_vec: Vec<RGBA8> = raw
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|chunk| RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
                    .collect();
                frames_data.push(rgba_vec);
            }

            let anim_frames: Vec<AnimFrameRgba> = frames_data
                .iter()
                .zip(frame_durations.iter())
                .map(|(data, &duration_ms)| AnimFrameRgba {
                    rgba: Img::new(data.as_slice(), 400, 300),
                    duration_ms,
                })
                .collect();

            let encoder = AvifEncoder::new().with_speed(6);
            let encoded = encoder.encode_animation_rgba(&anim_frames).unwrap();
            std::fs::write(&path_legacy, &encoded.avif_file).unwrap();
        });
    });

    e2e_group.finish();
}

criterion_group!(
    benches,
    bench_render_resolutions,
    bench_render_scenes,
    bench_asset_loading,
    bench_image_processing,
    bench_animation_and_pose,
    bench_encoding_static,
    bench_animation_export,
    bench_avif_export,
);
criterion_main!(benches);
