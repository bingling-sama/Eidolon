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

    let mut front_frame = renderer
        .render(&character, &front_camera, half_width, total_height)
        .map_err(|e| AppError::RenderError(format!("Rendering front view failed: {e}")))?;

    let back_yaw = (front_yaw - 180.0).rem_euclid(360.0);
    let back_camera = Camera {
        yaw: back_yaw,
        pitch: query.cam_pitch.unwrap_or(90.0),
        scale: query.cam_zoom.unwrap_or(1.0).max(0.01),
        target: [0.0, 1.0, 0.0],
    };

    let mut back_frame = renderer
        .render(
            &character,
            &back_camera,
            total_width - half_width,
            total_height,
        )
        .map_err(|e| AppError::RenderError(format!("Rendering back view failed: {e}")))?;

    let mut combined = image::RgbaImage::new(total_width, total_height);
    if query.transparent == Some(false) {
        apply_opaque_background(&mut front_frame);
        apply_opaque_background(&mut back_frame);
        apply_opaque_background(&mut combined);
    }

    image::imageops::overlay(&mut combined, &front_frame, 0, 0);
    image::imageops::overlay(&mut combined, &back_frame, half_width as i64, 0);

    let front_bytes = encode_static_image(&front_frame, StaticFormat::Png)
        .map_err(|e| AppError::Internal(format!("Failed to encode front image: {e}")))?;
    let back_bytes = encode_static_image(&back_frame, StaticFormat::Png)
        .map_err(|e| AppError::Internal(format!("Failed to encode back image: {e}")))?;
    let both_bytes = encode_static_image(&combined, StaticFormat::Png)
        .map_err(|e| AppError::Internal(format!("Failed to encode both image: {e}")))?;

    let b64 = &base64::engine::general_purpose::STANDARD;
    let now = js_date_string();

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

fn js_date_string() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = dur.as_secs();
    let days = (total_secs / 86400) as i64;
    let rem_secs = total_secs % 86400;
    let hour = rem_secs / 3600;
    let min = (rem_secs % 3600) / 60;
    let sec = rem_secs % 60;

    let day_of_week = match (4 + days).rem_euclid(7) {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        _ => "Sat",
    };

    let (year, month, day) = civil_from_days(days);
    let month_name = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        _ => "Dec",
    };

    format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        day_of_week, month_name, day, year, hour, min, sec
    )
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1020 + doe / 1461 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_civil_from_days_and_date_string() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 2026-11-02 is day 20759 since 1970-01-01
        let (y, m, d) = civil_from_days(20759);
        assert_eq!((y, m, d), (2026, 11, 2));

        let formatted = js_date_string();
        assert!(formatted.contains("GMT+0000 (Coordinated Universal Time)"));
    }

    #[test]
    fn test_apply_opaque_background() {
        let mut img = image::RgbaImage::new(2, 2);
        // Fully transparent pixel
        img.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
        // Fully opaque red pixel
        img.put_pixel(1, 0, image::Rgba([255, 0, 0, 255]));
        // Semi-transparent pixel
        img.put_pixel(0, 1, image::Rgba([0, 255, 0, 128]));

        apply_opaque_background(&mut img);

        // Fully transparent becomes #F0F0F0 (240, 240, 240, 255)
        assert_eq!(img.get_pixel(0, 0), &image::Rgba([240, 240, 240, 255]));
        // Fully opaque unchanged
        assert_eq!(img.get_pixel(1, 0), &image::Rgba([255, 0, 0, 255]));
        // Semi-transparent blended onto 240
        let p = img.get_pixel(0, 1);
        assert_eq!(p[3], 255);
        assert!(p[1] > 100);
    }
}
