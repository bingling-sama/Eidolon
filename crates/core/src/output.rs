//! Rendered-output policy and file encoders.

use crate::animation::{AnimationPreset, AnimationSchedule};
use crate::error::EidolonError;
use crate::output_avif::validate_frame;
#[cfg(not(target_arch = "wasm32"))]
use crate::output_avif::{frame_timestamp_ms, rounded_units, PreparedAvifAnimation};
use image::{ImageFormat, RgbaImage};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Static image formats supported by rendered output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticFormat {
    Png,
    WebP,
}

impl StaticFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::WebP => "webp",
        }
    }

    fn image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::WebP => ImageFormat::WebP,
        }
    }

    /// Infer a static format at a CLI or other input edge. Unknown extensions use PNG.
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("webp") => Self::WebP,
            _ => Self::Png,
        }
    }
}

/// Animated image formats supported by rendered output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationFormat {
    Gif,
    WebP,
    Apng,
    Avif,
}

impl AnimationFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Apng => "apng",
            Self::Avif => "avif",
        }
    }

    pub fn from_path(path: &Path) -> Result<Self, EidolonError> {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("gif") => Ok(Self::Gif),
            Some("webp") => Ok(Self::WebP),
            Some("apng") => Ok(Self::Apng),
            Some("avif") => Ok(Self::Avif),
            _ => Err(EidolonError::output(
                "animation output must end in .gif, .webp, .apng, or .avif",
            )),
        }
    }
}

/// The complete output policy for one static image or animated cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputIntent {
    Static {
        format: StaticFormat,
        size: (u32, u32),
    },
    Animation {
        format: AnimationFormat,
        preset: AnimationPreset,
        fps: u32,
        speed: f32,
        loop_count: u16,
        size: (u32, u32),
    },
}

impl OutputIntent {
    pub fn static_from_path(path: &Path, size: (u32, u32)) -> Self {
        Self::Static {
            format: StaticFormat::from_path(path),
            size,
        }
    }

    pub fn animation_from_path(
        path: &Path,
        preset: AnimationPreset,
        fps: u32,
        speed: f32,
        loop_count: u16,
        size: (u32, u32),
    ) -> Result<Self, EidolonError> {
        Ok(Self::Animation {
            format: AnimationFormat::from_path(path)?,
            preset,
            fps,
            speed,
            loop_count,
            size,
        })
    }
}

/// Resolve, encode, and write one output. The frame producer owns Character and Posture state.
pub fn write_output(
    path: impl AsRef<Path>,
    intent: OutputIntent,
    mut frame_producer: impl FnMut(Duration) -> Result<RgbaImage, EidolonError>,
) -> Result<PathBuf, EidolonError> {
    let path = path.as_ref();
    let (extension, size) = match intent {
        OutputIntent::Static { format, size } => (format.extension(), size),
        OutputIntent::Animation { format, size, .. } => (format.extension(), size),
    };
    validate_path(path)?;
    validate_size(size)?;
    let resolved = resolve_path(path, extension)?;

    match intent {
        OutputIntent::Static { format, .. } => {
            let image = frame_producer(Duration::ZERO)?;
            validate_frame(&image, size)?;
            image
                .save_with_format(&resolved, format.image_format())
                .map_err(|error| EidolonError::output(format!("failed to save image: {error}")))?;
        }
        OutputIntent::Animation {
            format,
            preset,
            fps,
            speed,
            loop_count,
            size,
        } => {
            validate_animation(size, fps, speed)?;
            let schedule = AnimationSchedule::new(preset, fps, speed)
                .ok_or_else(|| EidolonError::output("invalid animation schedule"))?;
            #[cfg(not(target_arch = "wasm32"))]
            write_animation(
                &resolved,
                format,
                size,
                &schedule,
                loop_count,
                &mut frame_producer,
            )?;
            #[cfg(target_arch = "wasm32")]
            {
                let _ = (format, schedule, loop_count, &mut frame_producer);
                return Err(EidolonError::output(
                    "animated output is unavailable on wasm32",
                ));
            }
        }
    }

    Ok(resolved)
}

/// Encode a static RGBA image into an in-memory byte buffer (PNG or WebP).
pub fn encode_static_image(
    image: &RgbaImage,
    format: StaticFormat,
) -> Result<Vec<u8>, EidolonError> {
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    image
        .write_to(&mut cursor, format.image_format())
        .map_err(|error| EidolonError::output(format!("failed to encode image: {error}")))?;
    Ok(bytes)
}

fn validate_path(path: &Path) -> Result<(), EidolonError> {
    if path.as_os_str().to_string_lossy().contains('\0') {
        return Err(EidolonError::invalid_path(
            "output filename contains null bytes",
        ));
    }
    if path.file_name().is_none() {
        return Err(EidolonError::invalid_path("output path must name a file"));
    }
    Ok(())
}

fn validate_size((width, height): (u32, u32)) -> Result<(), EidolonError> {
    if width == 0 || height == 0 {
        return Err(EidolonError::output(
            "output dimensions must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_animation(size: (u32, u32), fps: u32, speed: f32) -> Result<(), EidolonError> {
    if size.0 > 2_048 || size.1 > 2_048 {
        return Err(EidolonError::output(
            "animated output dimensions must not exceed 2048 pixels",
        ));
    }
    if !(1..=60).contains(&fps) {
        return Err(EidolonError::output(
            "animation fps must be between 1 and 60",
        ));
    }
    if !speed.is_finite() || !(0.25..=4.0).contains(&speed) {
        return Err(EidolonError::output(
            "animation speed must be between 0.25 and 4.0",
        ));
    }
    Ok(())
}

fn resolve_path(path: &Path, extension: &str) -> Result<PathBuf, EidolonError> {
    let mut resolved = path.to_path_buf();
    if !resolved.set_extension(extension) {
        return Err(EidolonError::invalid_path("output path must name a file"));
    }
    Ok(resolved)
}

#[cfg(not(target_arch = "wasm32"))]
fn gif_delay_cs(frame_index: u32, fps: u32) -> u32 {
    rounded_units(frame_index + 1, 100, fps) - rounded_units(frame_index, 100, fps)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_animation(
    path: &Path,
    format: AnimationFormat,
    size: (u32, u32),
    schedule: &AnimationSchedule,
    loop_count: u16,
    frame_producer: &mut impl FnMut(Duration) -> Result<RgbaImage, EidolonError>,
) -> Result<(), EidolonError> {
    match format {
        AnimationFormat::Gif => {
            use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat};
            use std::fs::File;
            use std::io::BufWriter;

            let file = BufWriter::new(File::create(path).map_err(|error| {
                EidolonError::output(format!("failed to create output: {error}"))
            })?);
            let mut encoder = GifEncoder::new(file, size.0 as u16, size.1 as u16, &[])
                .map_err(|error| EidolonError::output(format!("failed to create GIF: {error}")))?;
            encoder
                .set_repeat(if loop_count == 0 {
                    Repeat::Infinite
                } else {
                    Repeat::Finite(loop_count)
                })
                .map_err(|error| {
                    EidolonError::output(format!("failed to set GIF loop: {error}"))
                })?;

            for frame_index in 0..schedule.frame_count() {
                let image = frame_producer(schedule.sample_time(frame_index))?;
                validate_frame(&image, size)?;
                let mut rgba = image.into_raw();
                let mut frame =
                    GifFrame::from_rgba_speed(size.0 as u16, size.1 as u16, &mut rgba, 10);
                frame.delay = gif_delay_cs(frame_index, schedule.fps()) as u16;
                encoder.write_frame(&frame).map_err(|error| {
                    EidolonError::output(format!("failed to write GIF: {error}"))
                })?;
            }
            let file = encoder.into_inner().map_err(|error| {
                EidolonError::output(format!("failed to finalize GIF: {error}"))
            })?;
            file.into_inner()
                .map_err(|error| EidolonError::output(format!("failed to flush GIF: {error}")))?;
            Ok(())
        }
        AnimationFormat::WebP => {
            use webp_animation::{AnimParams, Encoder as WebpEncoder, EncoderOptions};

            let options = EncoderOptions {
                anim_params: AnimParams {
                    loop_count: i32::from(loop_count),
                },
                ..Default::default()
            };
            let mut encoder = WebpEncoder::new_with_options(size, options)
                .map_err(|error| EidolonError::output(format!("failed to create WebP: {error}")))?;
            for frame_index in 0..schedule.frame_count() {
                let image = frame_producer(schedule.sample_time(frame_index))?;
                validate_frame(&image, size)?;
                let rgba = image.into_raw();
                let timestamp: i32 = frame_timestamp_ms(frame_index, schedule.fps())
                    .try_into()
                    .map_err(|error| {
                        EidolonError::output(format!("invalid WebP timestamp: {error}"))
                    })?;
                encoder.add_frame(&rgba, timestamp).map_err(|error| {
                    EidolonError::output(format!("failed to write WebP: {error}"))
                })?;
            }
            let duration: i32 = frame_timestamp_ms(schedule.frame_count(), schedule.fps())
                .try_into()
                .map_err(|error| EidolonError::output(format!("invalid WebP duration: {error}")))?;
            encoder
                .finalize(duration)
                .map_err(|error| EidolonError::output(format!("failed to finalize WebP: {error}")))
                .and_then(|data| {
                    std::fs::write(path, data.as_ref()).map_err(|error| {
                        EidolonError::output(format!("failed to write output: {error}"))
                    })
                })?;
            Ok(())
        }
        AnimationFormat::Apng => {
            use png::{BitDepth, ColorType, Encoder as PngEncoder};
            use std::fs::File;
            use std::io::BufWriter;

            let file = BufWriter::new(File::create(path).map_err(|error| {
                EidolonError::output(format!("failed to create output: {error}"))
            })?);
            let mut encoder = PngEncoder::new(file, size.0, size.1);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            encoder
                .set_animated(schedule.frame_count(), u32::from(loop_count))
                .map_err(|error| EidolonError::output(format!("failed to set APNG: {error}")))?;
            encoder
                .set_frame_delay(1, schedule.fps() as u16)
                .map_err(|error| {
                    EidolonError::output(format!("failed to set APNG delay: {error}"))
                })?;

            let mut writer = encoder.write_header().map_err(|error| {
                EidolonError::output(format!("failed to write APNG header: {error}"))
            })?;

            for frame_index in 0..schedule.frame_count() {
                let image = frame_producer(schedule.sample_time(frame_index))?;
                validate_frame(&image, size)?;
                writer.write_image_data(&image).map_err(|error| {
                    EidolonError::output(format!("failed to write APNG frame: {error}"))
                })?;
            }
            writer.finish().map_err(|error| {
                EidolonError::output(format!("failed to finalize APNG: {error}"))
            })?;
            Ok(())
        }
        AnimationFormat::Avif => {
            use zenravif::Encoder as AvifEncoder;

            let prepared =
                PreparedAvifAnimation::collect_frames(size, schedule, &mut *frame_producer)?;
            let anim_frames = prepared.to_anim_frames();

            let encoder = AvifEncoder::new().with_speed(6);
            let encoded = encoder
                .encode_animation_rgba(&anim_frames)
                .map_err(|error| EidolonError::output(format!("failed to encode AVIF: {error}")))?;

            std::fs::write(path, &encoded.avif_file).map_err(|error| {
                EidolonError::output(format!("failed to write output: {error}"))
            })?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn rgba_frame(time: Duration) -> Result<RgbaImage, EidolonError> {
        let value = (time.as_millis() as u8).saturating_add(40);
        Ok(RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([value, 20, 220, 255]),
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn indexed_rgba_frame(time: Duration) -> Result<RgbaImage, EidolonError> {
        let frame_index = (time.as_millis() / 250) as u8;
        Ok(RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([40 + frame_index * 40, 20, 220, 55 + frame_index * 50]),
        ))
    }

    #[test]
    fn format_inference_is_case_insensitive() {
        assert_eq!(
            StaticFormat::from_path(Path::new("out.WEBP")),
            StaticFormat::WebP
        );
        assert_eq!(
            AnimationFormat::from_path(Path::new("out.GIF")).unwrap(),
            AnimationFormat::Gif
        );
        assert_eq!(
            AnimationFormat::from_path(Path::new("out.APNG")).unwrap(),
            AnimationFormat::Apng
        );
        assert_eq!(
            AnimationFormat::from_path(Path::new("out.AVIF")).unwrap(),
            AnimationFormat::Avif
        );
    }

    #[test]
    fn animation_format_rejects_static_extension() {
        assert!(AnimationFormat::from_path(Path::new("out.png")).is_err());
    }

    #[test]
    fn static_output_preserves_parent_and_returns_resolved_path() {
        let root = std::env::temp_dir().join(format!("eidolon-output-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let requested = root.join("render.PNG");
        let resolved = write_output(
            &requested,
            OutputIntent::Static {
                format: StaticFormat::WebP,
                size: (2, 2),
            },
            rgba_frame,
        )
        .unwrap();
        assert_eq!(resolved, root.join("render.webp"));
        assert!(resolved.exists());
        fs::remove_file(resolved).ok();
    }

    #[test]
    fn static_output_overwrites_existing_file() {
        let path = std::env::temp_dir().join(format!(
            "eidolon-output-overwrite-{}.png",
            std::process::id()
        ));
        fs::write(&path, b"stale output").unwrap();

        write_output(
            &path,
            OutputIntent::Static {
                format: StaticFormat::Png,
                size: (2, 2),
            },
            rgba_frame,
        )
        .unwrap();

        assert!(image::open(&path).is_ok());
        fs::remove_file(path).ok();
    }

    #[test]
    fn static_output_rejects_zero_dimensions_before_rendering() {
        let result = write_output(
            "output.png",
            OutputIntent::Static {
                format: StaticFormat::Png,
                size: (0, 2),
            },
            |_| panic!("invalid output should not request a frame"),
        );

        assert!(matches!(result, Err(EidolonError::Output(_))));
    }

    #[test]
    fn static_output_rejects_frame_dimension_mismatch() {
        let result = write_output(
            "output.png",
            OutputIntent::Static {
                format: StaticFormat::Png,
                size: (2, 2),
            },
            |_| Ok(RgbaImage::new(1, 2)),
        );

        assert!(matches!(result, Err(EidolonError::Output(_))));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn encoder_timing_rounds_without_drift() {
        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 30, 1.0).unwrap();
        assert_eq!(schedule.frame_count(), 30);
        assert_eq!(frame_timestamp_ms(30, schedule.fps()), 1_000);
        assert_eq!(
            (0..30)
                .map(|frame| gif_delay_cs(frame, schedule.fps()))
                .sum::<u32>(),
            100
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn gif_encoder_writes_frames_timing_dimensions_and_loop_metadata() {
        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 4, 1.0).unwrap();
        let path =
            std::env::temp_dir().join(format!("eidolon-output-gif-{}.gif", std::process::id()));
        write_animation(
            &path,
            AnimationFormat::Gif,
            (2, 2),
            &schedule,
            3,
            &mut indexed_rgba_frame,
        )
        .unwrap();

        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options
            .read_info(std::io::Cursor::new(fs::read(&path).unwrap()))
            .unwrap();
        assert_eq!(decoder.width(), 2);
        assert_eq!(decoder.height(), 2);
        assert_eq!(decoder.repeat(), gif::Repeat::Finite(3));

        let mut frame_count = 0;
        let mut delays = Vec::new();
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            frame_count += 1;
            delays.push(frame.delay);
        }
        assert_eq!(frame_count, 4);
        assert_eq!(delays, [25, 25, 25, 25]);
        fs::remove_file(path).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn webp_encoder_writes_lossless_rgba_frames_dimensions_and_loop_metadata() {
        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 4, 1.0).unwrap();
        let path =
            std::env::temp_dir().join(format!("eidolon-output-webp-{}.webp", std::process::id()));
        write_animation(
            &path,
            AnimationFormat::WebP,
            (2, 2),
            &schedule,
            2,
            &mut indexed_rgba_frame,
        )
        .unwrap();

        let encoded = fs::read(&path).unwrap();
        let decoder = webp_animation::Decoder::new(&encoded).unwrap();
        assert_eq!(decoder.dimensions(), (2, 2));
        assert!(format!("{decoder:?}").contains("loop_cnt: 2"));
        let frames: Vec<_> = decoder.into_iter().collect();
        assert_eq!(frames.len(), 4);
        assert_eq!(
            frames
                .iter()
                .map(|frame| frame.timestamp())
                .collect::<Vec<_>>(),
            [250, 500, 750, 1_000]
        );
        for (frame_index, frame) in frames.iter().enumerate() {
            assert_eq!(frame.dimensions(), (2, 2));
            assert_eq!(
                frame.data(),
                indexed_rgba_frame(Duration::from_millis(frame_index as u64 * 250,))
                    .unwrap()
                    .as_raw()
            );
        }
        fs::remove_file(path).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn apng_encoder_writes_frames_dimensions_and_animation_metadata() {
        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 4, 1.0).unwrap();
        let path =
            std::env::temp_dir().join(format!("eidolon-output-apng-{}.apng", std::process::id()));
        write_animation(
            &path,
            AnimationFormat::Apng,
            (2, 2),
            &schedule,
            2,
            &mut indexed_rgba_frame,
        )
        .unwrap();

        let file = fs::File::open(&path).unwrap();
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let mut reader = decoder.read_info().unwrap();
        assert_eq!(reader.info().size(), (2, 2));
        assert!(reader.info().is_animated());
        let actl = reader.info().animation_control().unwrap();
        assert_eq!(actl.num_frames, 4);
        assert_eq!(actl.num_plays, 2);

        let mut frame_count = 0;
        let mut buf = vec![0u8; reader.output_buffer_size().expect("output buffer size")];
        while let Ok(info) = reader.next_frame(&mut buf) {
            assert_eq!(info.width, 2);
            assert_eq!(info.height, 2);
            assert_eq!(
                &buf[..info.buffer_size()],
                indexed_rgba_frame(Duration::from_millis(frame_count as u64 * 250))
                    .unwrap()
                    .as_raw()
            );
            frame_count += 1;
            if frame_count == 4 {
                break;
            }
        }
        assert_eq!(frame_count, 4);
        fs::remove_file(path).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn avif_encoder_writes_valid_animation() {
        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 4, 1.0).unwrap();
        let path =
            std::env::temp_dir().join(format!("eidolon-output-avif-{}.avif", std::process::id()));
        write_animation(
            &path,
            AnimationFormat::Avif,
            (64, 64),
            &schedule,
            0,
            &mut |_| Ok(RgbaImage::new(64, 64)),
        )
        .unwrap();

        let encoded = fs::read(&path).unwrap();
        assert!(!encoded.is_empty());
        // Verify AVIF container signature (ftyp box with avif/avis brand)
        assert!(encoded.len() > 12);
        assert_eq!(&encoded[4..8], b"ftyp");
        fs::remove_file(path).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    struct IsoBox<'a> {
        kind: [u8; 4],
        payload: &'a [u8],
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[derive(Debug)]
    struct ParsedAvifTrack {
        track_id: u32,
        handler: [u8; 4],
        tkhd_size: (u32, u32),
        av01_size: (u32, u32),
        mdhd_timescale: u32,
        mdhd_duration: u64,
        stsz_sample_sizes: Vec<u32>,
        frame_durations: Vec<u32>,
        auxl_refs: Vec<u32>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[derive(Debug)]
    struct ParsedAvifContainer {
        major_brand: [u8; 4],
        compatible_brands: Vec<[u8; 4]>,
        mvhd_timescale: u32,
        mvhd_duration: u64,
        tracks: Vec<ParsedAvifTrack>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read_u16_be(bytes: &[u8], offset: usize) -> Result<u16, String> {
        let slice = bytes
            .get(offset..offset + 2)
            .ok_or_else(|| format!("unexpected EOF reading u16 at offset {offset}"))?;
        Ok(u16::from_be_bytes([slice[0], slice[1]]))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, String> {
        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| format!("unexpected EOF reading u32 at offset {offset}"))?;
        Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read_u64_be(bytes: &[u8], offset: usize) -> Result<u64, String> {
        let slice = bytes
            .get(offset..offset + 8)
            .ok_or_else(|| format!("unexpected EOF reading u64 at offset {offset}"))?;
        Ok(u64::from_be_bytes([
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ]))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read_fourcc(bytes: &[u8], offset: usize) -> Result<[u8; 4], String> {
        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| format!("unexpected EOF reading 4CC at offset {offset}"))?;
        Ok([slice[0], slice[1], slice[2], slice[3]])
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn iter_iso_boxes(mut data: &[u8]) -> Result<Vec<IsoBox<'_>>, String> {
        let mut boxes = Vec::new();
        while !data.is_empty() {
            if data.len() < 8 {
                return Err(format!(
                    "truncated ISOBMFF box header ({} bytes left)",
                    data.len()
                ));
            }
            let size32 = read_u32_be(data, 0)? as usize;
            let kind = read_fourcc(data, 4)?;
            let (header_len, total_len) = match size32 {
                0 => (8, data.len()),
                1 => {
                    let ext = usize::try_from(read_u64_be(data, 8)?)
                        .map_err(|_| "ISOBMFF box size overflow".to_string())?;
                    (16, ext)
                }
                n => (8, n),
            };
            if total_len < header_len || total_len > data.len() {
                return Err(format!(
                    "invalid ISOBMFF box {:?} size {total_len} (available {})",
                    String::from_utf8_lossy(&kind),
                    data.len()
                ));
            }
            boxes.push(IsoBox {
                kind,
                payload: &data[header_len..total_len],
            });
            data = &data[total_len..];
        }
        Ok(boxes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn find_iso_box<'a>(boxes: &'a [IsoBox<'a>], kind: &[u8; 4]) -> Result<&'a [u8], String> {
        boxes
            .iter()
            .find(|b| &b.kind == kind)
            .map(|b| b.payload)
            .ok_or_else(|| {
                format!(
                    "missing required ISOBMFF box {:?}",
                    String::from_utf8_lossy(kind)
                )
            })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_timescale_and_duration(payload: &[u8]) -> Result<(u32, u64), String> {
        let version = *payload
            .first()
            .ok_or_else(|| "empty FullBox timing header".to_string())?;
        match version {
            0 => Ok((
                read_u32_be(payload, 12)?,
                u64::from(read_u32_be(payload, 16)?),
            )),
            1 => Ok((read_u32_be(payload, 20)?, read_u64_be(payload, 24)?)),
            v => Err(format!("unsupported FullBox timing version {v}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_avif_track(trak_payload: &[u8]) -> Result<ParsedAvifTrack, String> {
        let trak_boxes = iter_iso_boxes(trak_payload)?;

        let tkhd = find_iso_box(&trak_boxes, b"tkhd")?;
        let tkhd_version = *tkhd.first().ok_or_else(|| "empty tkhd box".to_string())?;
        let (track_id, dim_offset) = match tkhd_version {
            0 => (read_u32_be(tkhd, 12)?, 76),
            1 => (read_u32_be(tkhd, 20)?, 88),
            v => return Err(format!("unsupported tkhd version {v}")),
        };
        let tkhd_width = read_u32_be(tkhd, dim_offset)? >> 16;
        let tkhd_height = read_u32_be(tkhd, dim_offset + 4)? >> 16;

        let mdia_boxes = iter_iso_boxes(find_iso_box(&trak_boxes, b"mdia")?)?;
        let (mdhd_timescale, mdhd_duration) =
            parse_timescale_and_duration(find_iso_box(&mdia_boxes, b"mdhd")?)?;
        let handler = read_fourcc(find_iso_box(&mdia_boxes, b"hdlr")?, 8)?;

        let minf_boxes = iter_iso_boxes(find_iso_box(&mdia_boxes, b"minf")?)?;
        let stbl_boxes = iter_iso_boxes(find_iso_box(&minf_boxes, b"stbl")?)?;

        let stsd = find_iso_box(&stbl_boxes, b"stsd")?;
        let entry_count = read_u32_be(stsd, 4)?;
        if entry_count < 1 {
            return Err("stsd must contain at least 1 sample entry".to_string());
        }
        let sample_entries = iter_iso_boxes(&stsd[8..])?;
        let av01 = find_iso_box(&sample_entries, b"av01")?;
        let av01_size = (
            u32::from(read_u16_be(av01, 24)?),
            u32::from(read_u16_be(av01, 26)?),
        );

        let stts = find_iso_box(&stbl_boxes, b"stts")?;
        let stts_count = read_u32_be(stts, 4)? as usize;
        let mut frame_durations = Vec::new();
        for idx in 0..stts_count {
            let entry_offset = 8 + idx * 8;
            let sample_count = read_u32_be(stts, entry_offset)?;
            let sample_delta = read_u32_be(stts, entry_offset + 4)?;
            frame_durations.extend(std::iter::repeat_n(sample_delta, sample_count as usize));
        }

        let stsz = find_iso_box(&stbl_boxes, b"stsz")?;
        let uniform_size = read_u32_be(stsz, 4)?;
        let sample_count = read_u32_be(stsz, 8)? as usize;
        let mut stsz_sample_sizes = Vec::with_capacity(sample_count);
        if uniform_size != 0 {
            stsz_sample_sizes.resize(sample_count, uniform_size);
        } else {
            for idx in 0..sample_count {
                stsz_sample_sizes.push(read_u32_be(stsz, 12 + idx * 4)?);
            }
        }

        let mut auxl_refs = Vec::new();
        if let Ok(tref_payload) = find_iso_box(&trak_boxes, b"tref") {
            let tref_boxes = iter_iso_boxes(tref_payload)?;
            if let Ok(auxl_payload) = find_iso_box(&tref_boxes, b"auxl") {
                for offset in (0..auxl_payload.len()).step_by(4) {
                    auxl_refs.push(read_u32_be(auxl_payload, offset)?);
                }
            }
        }

        Ok(ParsedAvifTrack {
            track_id,
            handler,
            tkhd_size: (tkhd_width, tkhd_height),
            av01_size,
            mdhd_timescale,
            mdhd_duration,
            stsz_sample_sizes,
            frame_durations,
            auxl_refs,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_avif_animation(bytes: &[u8]) -> Result<ParsedAvifContainer, String> {
        let top_boxes = iter_iso_boxes(bytes)?;

        let ftyp = find_iso_box(&top_boxes, b"ftyp")?;
        let major_brand = read_fourcc(ftyp, 0)?;
        let mut compatible_brands = Vec::new();
        for offset in (8..ftyp.len()).step_by(4) {
            compatible_brands.push(read_fourcc(ftyp, offset)?);
        }

        let _meta = find_iso_box(&top_boxes, b"meta")?;
        let moov_boxes = iter_iso_boxes(find_iso_box(&top_boxes, b"moov")?)?;
        let (mvhd_timescale, mvhd_duration) =
            parse_timescale_and_duration(find_iso_box(&moov_boxes, b"mvhd")?)?;

        let mut tracks = Vec::new();
        for b in &moov_boxes {
            if &b.kind == b"trak" {
                tracks.push(parse_avif_track(b.payload)?);
            }
        }

        Ok(ParsedAvifContainer {
            major_brand,
            compatible_brands,
            mvhd_timescale,
            mvhd_duration,
            tracks,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn avif_export_correctness_and_validation() {
        let path = std::env::temp_dir().join(format!(
            "eidolon-avif-correctness-{}.avif",
            std::process::id()
        ));
        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 15, 1.0).unwrap();
        assert_eq!(schedule.fps(), 15);
        assert_eq!(schedule.frame_count(), 15);

        let expected_durations: Vec<u32> = (0..15)
            .map(|i| frame_timestamp_ms(i + 1, 15) - frame_timestamp_ms(i, 15))
            .collect();
        let expected_total_duration: u32 = expected_durations.iter().sum();
        assert_eq!(
            expected_total_duration, 1_000,
            "15 frames at 15fps must total exactly 1000ms"
        );

        // Producer returning frames with varied RGB and alpha (transparent, semi-transparent, opaque)
        let mut frame_call_count = 0u32;
        let result = write_output(
            &path,
            OutputIntent::Animation {
                format: AnimationFormat::Avif,
                preset: AnimationPreset::Walk,
                fps: 15,
                speed: 1.0,
                loop_count: 0,
                size: (64, 64),
            },
            |_| {
                let idx = frame_call_count;
                frame_call_count += 1;
                let alpha = match idx % 3 {
                    0 => 0,
                    1 => 128,
                    _ => 255,
                };
                Ok(RgbaImage::from_pixel(
                    64,
                    64,
                    image::Rgba([
                        ((idx * 17) % 256) as u8,
                        ((idx * 29) % 256) as u8,
                        ((idx * 43) % 256) as u8,
                        alpha,
                    ]),
                ))
            },
        );
        assert!(
            result.is_ok(),
            "AVIF animation export must succeed: {:?}",
            result.err()
        );
        assert_eq!(frame_call_count, 15, "Must sample all 15 frames");

        let encoded = fs::read(&path).unwrap();
        fs::remove_file(&path).ok();

        let parsed = parse_avif_animation(&encoded)
            .expect("encoded AVIF animation must contain valid ISOBMFF container boxes");
        assert_eq!(parsed.major_brand, *b"avis", "Major brand must be avis");
        assert!(
            parsed.compatible_brands.contains(b"avif"),
            "Compatible brands must include avif"
        );
        assert_eq!(
            parsed.mvhd_timescale, 1_000,
            "mvhd timescale must be 1000ms"
        );
        assert_eq!(
            parsed.mvhd_duration,
            u64::from(expected_total_duration),
            "mvhd duration must match serialized 1000ms total"
        );
        assert_eq!(
            parsed.tracks.len(),
            2,
            "Translucent RGBA animation must emit both color (pict) and alpha (auxv) tracks"
        );

        let color_track = parsed
            .tracks
            .iter()
            .find(|t| &t.handler == b"pict")
            .expect("Must contain pict color track");
        assert_eq!(color_track.tkhd_size, (64, 64));
        assert_eq!(color_track.av01_size, (64, 64));
        assert_eq!(color_track.mdhd_timescale, 1_000);
        assert_eq!(
            color_track.mdhd_duration,
            u64::from(expected_total_duration)
        );
        assert_eq!(color_track.stsz_sample_sizes.len(), 15);
        assert!(color_track.stsz_sample_sizes.iter().all(|&sz| sz > 0));
        assert_eq!(
            color_track.frame_durations, expected_durations,
            "Encoded ISOBMFF stts durations must match cumulative-rounded 15fps schedule"
        );

        let alpha_track = parsed
            .tracks
            .iter()
            .find(|t| &t.handler == b"auxv")
            .expect("Must contain auxv alpha auxiliary track");
        assert_eq!(alpha_track.tkhd_size, (64, 64));
        assert_eq!(alpha_track.av01_size, (64, 64));
        assert_eq!(alpha_track.mdhd_timescale, 1_000);
        assert_eq!(
            alpha_track.mdhd_duration,
            u64::from(expected_total_duration)
        );
        assert_eq!(alpha_track.stsz_sample_sizes.len(), 15);
        assert!(alpha_track.stsz_sample_sizes.iter().all(|&sz| sz > 0));
        assert_eq!(alpha_track.frame_durations, expected_durations);
        assert_eq!(
            alpha_track.auxl_refs,
            vec![color_track.track_id],
            "Alpha track must reference the color track via tref/auxl"
        );

        // Opaque input must omit the auxiliary alpha track (proving auxv responds to alpha presence)
        let opaque_path =
            std::env::temp_dir().join(format!("eidolon-avif-opaque-{}.avif", std::process::id()));
        let mut opaque_idx = 0u8;
        write_output(
            &opaque_path,
            OutputIntent::Animation {
                format: AnimationFormat::Avif,
                preset: AnimationPreset::Walk,
                fps: 4,
                speed: 1.0,
                loop_count: 0,
                size: (64, 64),
            },
            |_| {
                let c = opaque_idx.wrapping_mul(50);
                opaque_idx = opaque_idx.wrapping_add(1);
                Ok(RgbaImage::from_pixel(
                    64,
                    64,
                    image::Rgba([c, 120, 200, 255]),
                ))
            },
        )
        .expect("Opaque AVIF export must succeed");
        let opaque_encoded = fs::read(&opaque_path).unwrap();
        fs::remove_file(&opaque_path).ok();

        let opaque_parsed = parse_avif_animation(&opaque_encoded)
            .expect("Opaque AVIF animation must parse cleanly");
        assert_eq!(
            opaque_parsed.tracks.len(),
            1,
            "Opaque animation must emit only the color track without auxv"
        );
        assert_eq!(opaque_parsed.tracks[0].handler, *b"pict");
        assert_eq!(opaque_parsed.tracks[0].stsz_sample_sizes.len(), 4);
        assert_eq!(opaque_parsed.tracks[0].frame_durations, vec![250; 4]);

        // Reject frame dimension mismatch
        let mismatch_result = write_output(
            "mismatch.avif",
            OutputIntent::Animation {
                format: AnimationFormat::Avif,
                preset: AnimationPreset::Walk,
                fps: 15,
                speed: 1.0,
                loop_count: 0,
                size: (64, 64),
            },
            |_| Ok(RgbaImage::new(32, 32)),
        );
        assert!(
            matches!(mismatch_result, Err(EidolonError::Output(_))),
            "Must reject mismatched frame dimensions"
        );
    }
}
