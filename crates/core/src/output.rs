//! Rendered-output policy and file encoders.

use crate::animation::{AnimationPreset, AnimationSchedule};
use crate::error::EidolonError;
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

fn validate_frame(image: &RgbaImage, (width, height): (u32, u32)) -> Result<(), EidolonError> {
    if image.dimensions() != (width, height) {
        return Err(EidolonError::output(format!(
            "frame dimensions {}x{} do not match requested output {}x{}",
            image.width(),
            image.height(),
            width,
            height
        )));
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
fn rounded_units(frame_boundary: u32, units_per_second: u32, fps: u32) -> u32 {
    (frame_boundary * units_per_second + fps / 2) / fps
}

#[cfg(not(target_arch = "wasm32"))]
fn webp_timestamp_ms(frame_boundary: u32, fps: u32) -> u32 {
    rounded_units(frame_boundary, 1_000, fps)
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
                let timestamp: i32 = webp_timestamp_ms(frame_index, schedule.fps())
                    .try_into()
                    .map_err(|error| {
                        EidolonError::output(format!("invalid WebP timestamp: {error}"))
                    })?;
                encoder.add_frame(&rgba, timestamp).map_err(|error| {
                    EidolonError::output(format!("failed to write WebP: {error}"))
                })?;
            }
            let duration: i32 = webp_timestamp_ms(schedule.frame_count(), schedule.fps())
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
                .map_err(|error| EidolonError::output(format!("failed to set APNG delay: {error}")))?;

            let mut writer = encoder
                .write_header()
                .map_err(|error| EidolonError::output(format!("failed to write APNG header: {error}")))?;

            for frame_index in 0..schedule.frame_count() {
                let image = frame_producer(schedule.sample_time(frame_index))?;
                validate_frame(&image, size)?;
                writer
                    .write_image_data(&image)
                    .map_err(|error| EidolonError::output(format!("failed to write APNG frame: {error}")))?;
            }
            writer
                .finish()
                .map_err(|error| EidolonError::output(format!("failed to finalize APNG: {error}")))?;
            Ok(())
        }
        AnimationFormat::Avif => {
            use zenravif::{AnimFrameRgba, Encoder as AvifEncoder, Img, RGBA8};

            let mut frames_data: Vec<Vec<RGBA8>> = Vec::with_capacity(schedule.frame_count() as usize);
            let mut frame_durations: Vec<u32> = Vec::with_capacity(schedule.frame_count() as usize);

            for frame_index in 0..schedule.frame_count() {
                let image = frame_producer(schedule.sample_time(frame_index))?;
                validate_frame(&image, size)?;
                let duration_ms = webp_timestamp_ms(frame_index + 1, schedule.fps())
                    - webp_timestamp_ms(frame_index, schedule.fps());
                frame_durations.push(duration_ms);

                let raw = image.into_raw();
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
                    rgba: Img::new(data.as_slice(), size.0 as usize, size.1 as usize),
                    duration_ms,
                })
                .collect();

            let encoder = AvifEncoder::new()
                .with_speed(6);
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

    #[test]
    fn encoder_timing_rounds_without_drift() {
        let schedule = AnimationSchedule::new(AnimationPreset::Walk, 30, 1.0).unwrap();
        assert_eq!(schedule.frame_count(), 30);
        assert_eq!(webp_timestamp_ms(30, schedule.fps()), 1_000);
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
        let decoder = png::Decoder::new(file);
        let mut reader = decoder.read_info().unwrap();
        assert_eq!(reader.info().size(), (2, 2));
        assert!(reader.info().is_animated());
        let actl = reader.info().animation_control().unwrap();
        assert_eq!(actl.num_frames, 4);
        assert_eq!(actl.num_plays, 2);

        let mut frame_count = 0;
        let mut buf = vec![0u8; reader.output_buffer_size()];
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
}
