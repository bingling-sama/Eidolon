use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Sender};
use eidolon_core::{
    animation::AnimationPreset,
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, Posture, SkinType},
    output::{encode_static_image, StaticFormat},
    renderer::Renderer,
};
use tokio::sync::oneshot;
use tracing::{error, info};

use crate::error::AppError;
use crate::model::RenderQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Single,
    Both,
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
                            RenderMode::Both => execute_render_both(
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
        self.sender
            .try_send(job)
            .map_err(|_| AppError::QueueFull)
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

fn execute_render(
    renderer: &Renderer,
    skin_bytes: Vec<u8>,
    cape_bytes: Option<Vec<u8>>,
    elytra_bytes: Option<Vec<u8>>,
    query: &RenderQuery,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let skin_type = if query.is_slim(&skin_bytes) {
        SkinType::Slim
    } else {
        SkinType::Classic
    };

    let body = renderer
        .load_body_from_memory(&skin_bytes, skin_type)
        .map_err(|e| AppError::RenderError(format!("Failed to load skin image: {e}")))?;

    let mut character = Character::from_body(body);

    if let Some(bytes) = cape_bytes {
        let atlas = renderer
            .load_back_equipment_atlas_from_memory(&bytes)
            .map_err(|e| AppError::RenderError(format!("Failed to load cape: {e}")))?;
        character.equip(BackEquipment::new(atlas, BackEquipmentKind::Cape));
    } else if let Some(bytes) = elytra_bytes {
        let atlas = renderer
            .load_back_equipment_atlas_from_memory(&bytes)
            .map_err(|e| AppError::RenderError(format!("Failed to load elytra: {e}")))?;
        character.equip(BackEquipment::new(atlas, BackEquipmentKind::Elytra));
    }

    let (preset, sample_time) = resolve_preset_and_time(query.posture.as_deref());
    let base_posture = preset.sample(sample_time);
    character.posture = apply_overrides(base_posture, query);

    character.nametag = query.nametag.clone();
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

    let camera = Camera {
        yaw: query.cam_yaw.unwrap_or(180.0),
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
    };

    let (width, height) = query.dimensions();
    let frame = renderer
        .render(&character, &camera, width, height)
        .map_err(|e| AppError::RenderError(format!("Rendering failed: {e}")))?;

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
    let skin_type = if query.is_slim(&skin_bytes) {
        SkinType::Slim
    } else {
        SkinType::Classic
    };

    let total_width = query.width.unwrap_or(600).clamp(32, 4096);
    let total_height = query.height.unwrap_or(600).clamp(32, 4096);
    let half_width = total_width / 2;

    let body = renderer
        .load_body_from_memory(&skin_bytes, skin_type)
        .map_err(|e| AppError::RenderError(format!("Failed to load skin image: {e}")))?;
    let mut character = Character::from_body(body);

    if let Some(bytes) = cape_bytes {
        let atlas = renderer
            .load_back_equipment_atlas_from_memory(&bytes)
            .map_err(|e| AppError::RenderError(format!("Failed to load cape: {e}")))?;
        character.equip(BackEquipment::new(atlas, BackEquipmentKind::Cape));
    } else if let Some(bytes) = elytra_bytes {
        let atlas = renderer
            .load_back_equipment_atlas_from_memory(&bytes)
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

    // Front camera (left side): slight 15-degree turn or custom cam_yaw
    let front_yaw = query.cam_yaw.unwrap_or(195.0);
    let front_camera = Camera {
        yaw: front_yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
    };

    let front_frame = renderer
        .render(&character, &front_camera, half_width, total_height)
        .map_err(|e| AppError::RenderError(format!("Rendering front view failed: {e}")))?;

    // Back camera (right side): back view (yaw = front_yaw - 180.0, e.g. 15.0)
    let back_yaw = (front_yaw - 180.0).rem_euclid(360.0);
    let back_camera = Camera {
        yaw: back_yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
    };

    let back_frame = renderer
        .render(&character, &back_camera, total_width - half_width, total_height)
        .map_err(|e| AppError::RenderError(format!("Rendering back view failed: {e}")))?;

    // Stitch horizontally into a single image buffer
    let mut combined = image::RgbaImage::new(total_width, total_height);
    image::imageops::overlay(&mut combined, &front_frame, 0, 0);
    image::imageops::overlay(&mut combined, &back_frame, half_width as i64, 0);

    let (format, content_type) = if query.is_webp() {
        (StaticFormat::WebP, "image/webp")
    } else {
        (StaticFormat::Png, "image/png")
    };

    let encoded = encode_static_image(&combined, format)
        .map_err(|e| AppError::Internal(format!("Failed to encode image: {e}")))?;

    Ok((encoded, content_type))
}
