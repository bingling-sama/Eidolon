use std::thread;
use std::time::Duration;

use base64::Engine;
use crossbeam_channel::{bounded, Sender};
use eidolon_core::{
    animation::AnimationPreset,
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, Posture, SkinType},
    output::{encode_static_image, StaticFormat},
    renderer::Renderer,
};
use serde_json::json;
use tokio::sync::oneshot;
use tracing::{error, info};

use crate::error::AppError;
use crate::model::RenderQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Single,
    Front,
    Back,
    Both,
    JsonAll,
}

pub struct RenderJob {
    pub skin_bytes: Vec<u8>,
    pub cape_bytes: Option<Vec<u8>>,
    pub elytra_bytes: Option<Vec<u8>>,
    pub query: RenderQuery,
    pub mode: RenderMode,
    pub responder: oneshot::Sender<Result<(Vec<u8>, &'static str), AppError>>,
}

#[derive(Clone)]
pub struct RenderWorkerPool {
    sender: Sender<RenderJob>,
}

impl RenderWorkerPool {
    pub fn new(num_workers: usize, queue_capacity: usize) -> Self {
        let (sender, receiver) = bounded::<RenderJob>(queue_capacity);

        for worker_id in 0..num_workers {
            let rx = receiver.clone();
            thread::Builder::new()
                .name(format!("render-worker-{worker_id}"))
                .spawn(move || {
                    info!("Render worker {} started", worker_id);
                    let renderer = match Renderer::new() {
                        Ok(r) => r,
                        Err(e) => {
                            error!("Worker {worker_id} failed to initialize Renderer: {e}");
                            return;
                        }
                    };
                    info!("Worker {} Renderer initialized successfully", worker_id);

                    while let Ok(job) = rx.recv() {
                        let result = match job.mode {
                            RenderMode::Single => execute_render(
                                &renderer,
                                job.skin_bytes,
                                job.cape_bytes,
                                job.elytra_bytes,
                                &job.query,
                            ),
                            RenderMode::Front => execute_render_single_view(
                                &renderer,
                                job.skin_bytes,
                                job.cape_bytes,
                                job.elytra_bytes,
                                &job.query,
                                false,
                            ),
                            RenderMode::Back => execute_render_single_view(
                                &renderer,
                                job.skin_bytes,
                                job.cape_bytes,
                                job.elytra_bytes,
                                &job.query,
                                true,
                            ),
                            RenderMode::Both => execute_render_both(
                                &renderer,
                                job.skin_bytes,
                                job.cape_bytes,
                                job.elytra_bytes,
                                &job.query,
                            ),
                            RenderMode::JsonAll => execute_render_json_all(
                                &renderer,
                                job.skin_bytes,
                                job.cape_bytes,
                                job.elytra_bytes,
                                &job.query,
                            ),
                        };
                        let _ = job.responder.send(result);
                    }
                    info!("Render worker {} shutting down", worker_id);
                    // Leak renderer on process exit to avoid Mesa/OSMesa driver destruction race with main thread exit
                    std::mem::forget(renderer);
                })
                .expect("Failed to spawn render worker thread");
        }

        Self { sender }
    }

    pub fn submit(&self, job: RenderJob) -> Result<(), AppError> {
        self.sender.try_send(job).map_err(|_| AppError::QueueFull)
    }
}

fn resolve_preset_and_time(preset_str: Option<&str>) -> (AnimationPreset, Duration) {
    match preset_str.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("walk") => (
            AnimationPreset::Walk,
            AnimationPreset::Walk.cycle_duration() / 4,
        ),
        Some("run") => (
            AnimationPreset::Run,
            AnimationPreset::Run.cycle_duration() / 4,
        ),
        Some("wave") => (AnimationPreset::Wave, Duration::ZERO),
        Some("fly") => (AnimationPreset::Fly, Duration::ZERO),
        _ => (AnimationPreset::Idle, Duration::ZERO),
    }
}

fn apply_overrides(mut posture: Posture, query: &RenderQuery) -> Posture {
    if let Some(v) = query.cape_pitch {
        posture.cape_pitch = v;
    }
    if let Some(v) = query.elytra_pitch {
        posture.elytra_pitch = v;
    }
    if let Some(v) = query.elytra_spread {
        posture.elytra_roll = v;
    }
    if let Some(v) = query.head_yaw {
        posture.head_yaw = v;
    }
    if let Some(v) = query.head_pitch {
        posture.head_pitch = v;
    }
    if let Some(v) = query.left_arm_roll {
        posture.left_arm_roll = v;
    }
    if let Some(v) = query.left_arm_pitch {
        posture.left_arm_pitch = v;
    }
    if let Some(v) = query.right_arm_roll {
        posture.right_arm_roll = v;
    }
    if let Some(v) = query.right_arm_pitch {
        posture.right_arm_pitch = v;
    }
    if let Some(v) = query.left_leg_pitch {
        posture.left_leg_pitch = v;
    }
    if let Some(v) = query.right_leg_pitch {
        posture.right_leg_pitch = v;
    }
    posture
}

fn build_character(
    renderer: &Renderer,
    skin_bytes: &[u8],
    cape_bytes: Option<&[u8]>,
    elytra_bytes: Option<&[u8]>,
    query: &RenderQuery,
) -> Result<Character, AppError> {
    let skin_type = if query.is_slim(skin_bytes) {
        SkinType::Slim
    } else {
        SkinType::Classic
    };

    let body = renderer
        .load_body_from_memory(skin_bytes, skin_type)
        .map_err(|e| AppError::RenderError(format!("Failed to load skin image: {e}")))?;
    let mut character = Character::from_body(body);

    if let Some(bytes) = cape_bytes {
        let atlas = renderer
            .load_back_equipment_atlas_from_memory(bytes)
            .map_err(|e| AppError::RenderError(format!("Failed to load cape: {e}")))?;
        character.equip(BackEquipment::new(atlas, BackEquipmentKind::Cape));
    } else if let Some(bytes) = elytra_bytes {
        let atlas = renderer
            .load_back_equipment_atlas_from_memory(bytes)
            .map_err(|e| AppError::RenderError(format!("Failed to load elytra: {e}")))?;
        character.equip(BackEquipment::new(atlas, BackEquipmentKind::Elytra));
    }

    let (preset, sample_time) = resolve_preset_and_time(query.posture.as_deref());
    let base_posture = preset.sample(sample_time);
    character.posture = apply_overrides(base_posture, query);

    character.nametag = query.resolved_nametag().map(|s| s.to_string());
    character.nametag_see_through = !query.disable_nametag_see_through.unwrap_or(false);

    character.position = cgmath::Vector3::new(
        query.pos_x.unwrap_or(0.0),
        query.pos_y.unwrap_or(0.0),
        query.pos_z.unwrap_or(0.0),
    );
    character.rotation = cgmath::Vector3::new(
        query.rot_x.unwrap_or(0.0),
        query.rot_y.unwrap_or(0.0),
        query.rot_z.unwrap_or(0.0),
    );

    Ok(character)
}

fn execute_render(
    renderer: &Renderer,
    skin_bytes: Vec<u8>,
    cape_bytes: Option<Vec<u8>>,
    elytra_bytes: Option<Vec<u8>>,
    query: &RenderQuery,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let character = build_character(
        renderer,
        &skin_bytes,
        cape_bytes.as_deref(),
        elytra_bytes.as_deref(),
        query,
    )?;

    let camera = Camera {
        yaw: query.cam_yaw.unwrap_or(180.0),
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
        target: [0.0, 1.0, 0.0],
    };

    let (width, height) = query.dimensions();
    let mut frame = renderer
        .render(&character, &camera, width, height)
        .map_err(|e| AppError::RenderError(format!("Rendering failed: {e}")))?;

    if query.transparent == Some(false) {
        apply_opaque_background(&mut frame);
    }

    let (format, content_type) = if query.is_webp() {
        (StaticFormat::WebP, "image/webp")
    } else {
        (StaticFormat::Png, "image/png")
    };

    let encoded = encode_static_image(&frame, format)
        .map_err(|e| AppError::Internal(format!("Failed to encode image: {e}")))?;

    Ok((encoded, content_type))
}

fn execute_render_single_view(
    renderer: &Renderer,
    skin_bytes: Vec<u8>,
    cape_bytes: Option<Vec<u8>>,
    elytra_bytes: Option<Vec<u8>>,
    query: &RenderQuery,
    is_back: bool,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let character = build_character(
        renderer,
        &skin_bytes,
        cape_bytes.as_deref(),
        elytra_bytes.as_deref(),
        query,
    )?;

    let (width, height) = query.skinrendermc_single_dimensions();
    let base_yaw = query.cam_yaw.unwrap_or(195.0);
    let yaw = if is_back {
        (base_yaw - 180.0).rem_euclid(360.0)
    } else {
        base_yaw
    };

    let camera = Camera {
        yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
        target: [0.0, 1.0, 0.0],
    };

    let mut frame = renderer
        .render(&character, &camera, width, height)
        .map_err(|e| AppError::RenderError(format!("Rendering view failed: {e}")))?;

    if query.transparent == Some(false) {
        apply_opaque_background(&mut frame);
    }

    let (format, content_type) = if query.is_webp() {
        (StaticFormat::WebP, "image/webp")
    } else {
        (StaticFormat::Png, "image/png")
    };

    let encoded = encode_static_image(&frame, format)
        .map_err(|e| AppError::Internal(format!("Failed to encode image: {e}")))?;

    Ok((encoded, content_type))
}

fn execute_render_both(
    renderer: &Renderer,
    skin_bytes: Vec<u8>,
    cape_bytes: Option<Vec<u8>>,
    elytra_bytes: Option<Vec<u8>>,
    query: &RenderQuery,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let character = build_character(
        renderer,
        &skin_bytes,
        cape_bytes.as_deref(),
        elytra_bytes.as_deref(),
        query,
    )?;

    let (total_width, total_height) = query.skinrendermc_both_dimensions();
    let half_width = total_width / 2;

    let front_yaw = query.cam_yaw.unwrap_or(195.0);
    let front_camera = Camera {
        yaw: front_yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
        target: [0.0, 1.0, 0.0],
    };

    let front_frame = renderer
        .render(&character, &front_camera, half_width, total_height)
        .map_err(|e| AppError::RenderError(format!("Rendering front view failed: {e}")))?;

    let back_yaw = (front_yaw - 180.0).rem_euclid(360.0);
    let back_camera = Camera {
        yaw: back_yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
        target: [0.0, 1.0, 0.0],
    };

    let back_frame = renderer
        .render(
            &character,
            &back_camera,
            total_width - half_width,
            total_height,
        )
        .map_err(|e| AppError::RenderError(format!("Rendering back view failed: {e}")))?;

    let mut combined = image::RgbaImage::new(total_width, total_height);
    image::imageops::overlay(&mut combined, &front_frame, 0, 0);
    image::imageops::overlay(&mut combined, &back_frame, half_width as i64, 0);

    if query.transparent == Some(false) {
        apply_opaque_background(&mut combined);
    }

    let (format, content_type) = if query.is_webp() {
        (StaticFormat::WebP, "image/webp")
    } else {
        (StaticFormat::Png, "image/png")
    };

    let encoded = encode_static_image(&combined, format)
        .map_err(|e| AppError::Internal(format!("Failed to encode image: {e}")))?;

    Ok((encoded, content_type))
}

fn execute_render_json_all(
    renderer: &Renderer,
    skin_bytes: Vec<u8>,
    cape_bytes: Option<Vec<u8>>,
    elytra_bytes: Option<Vec<u8>>,
    query: &RenderQuery,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let character = build_character(
        renderer,
        &skin_bytes,
        cape_bytes.as_deref(),
        elytra_bytes.as_deref(),
        query,
    )?;

    let is_slim = query.is_slim(&skin_bytes);
    let (total_width, total_height) = query.skinrendermc_both_dimensions();
    let half_width = total_width / 2;

    let front_yaw = query.cam_yaw.unwrap_or(195.0);
    let front_camera = Camera {
        yaw: front_yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
        target: [0.0, 1.0, 0.0],
    };

    let front_frame = renderer
        .render(&character, &front_camera, half_width, total_height)
        .map_err(|e| AppError::RenderError(format!("Rendering front view failed: {e}")))?;

    let back_yaw = (front_yaw - 180.0).rem_euclid(360.0);
    let back_camera = Camera {
        yaw: back_yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
        target: [0.0, 1.0, 0.0],
    };

    let back_frame = renderer
        .render(
            &character,
            &back_camera,
            total_width - half_width,
            total_height,
        )
        .map_err(|e| AppError::RenderError(format!("Rendering back view failed: {e}")))?;

    let mut combined = image::RgbaImage::new(total_width, total_height);
    image::imageops::overlay(&mut combined, &front_frame, 0, 0);
    image::imageops::overlay(&mut combined, &back_frame, half_width as i64, 0);

    let front_bytes = encode_static_image(&front_frame, StaticFormat::Png)
        .map_err(|e| AppError::Internal(format!("Failed to encode front image: {e}")))?;
    let back_bytes = encode_static_image(&back_frame, StaticFormat::Png)
        .map_err(|e| AppError::Internal(format!("Failed to encode back image: {e}")))?;
    let both_bytes = encode_static_image(&combined, StaticFormat::Png)
        .map_err(|e| AppError::Internal(format!("Failed to encode both image: {e}")))?;

    let b64 = &base64::engine::general_purpose::STANDARD;
    let now = chrono_or_fallback_time();

    let json_val = json!({
        "front_b64": b64.encode(&front_bytes),
        "back_b64": b64.encode(&back_bytes),
        "both_b64": b64.encode(&both_bytes),
        "backendInfo": {
            "slim": is_slim,
            "serverTimeJs": now,
        }
    });

    let json_bytes = serde_json::to_vec(&json_val)
        .map_err(|e| AppError::Internal(format!("Failed to serialize JSON: {e}")))?;

    Ok((json_bytes, "application/json"))
}

fn chrono_or_fallback_time() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("UnixTimestamp: {}s", dur.as_secs())
}

fn apply_opaque_background(img: &mut image::RgbaImage) {
    for pixel in img.pixels_mut() {
        if pixel[3] < 255 {
            let alpha = pixel[3] as f32 / 255.0;
            // SkinRenderMC default background: light grayish #F0F0F0
            let bg = 240.0;
            pixel[0] = (pixel[0] as f32 * alpha + bg * (1.0 - alpha)).round() as u8;
            pixel[1] = (pixel[1] as f32 * alpha + bg * (1.0 - alpha)).round() as u8;
            pixel[2] = (pixel[2] as f32 * alpha + bg * (1.0 - alpha)).round() as u8;
            pixel[3] = 255;
        }
    }
}
