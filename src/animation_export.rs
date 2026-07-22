use eidolon::animation::AnimationPreset;
use eidolon::camera::Camera;
use eidolon::character::{Character, Posture};
use eidolon::renderer::Renderer;
use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;
use webp_animation::{AnimParams, Encoder as WebpEncoder, EncoderOptions};

type DynError = Box<dyn std::error::Error>;

/// Frame sampling and cumulative timestamp rounding for one exported cycle.
pub(crate) struct AnimationTiming {
    preset: AnimationPreset,
    fps: u32,
    frame_count: u32,
}

impl AnimationTiming {
    pub(crate) fn new(preset: AnimationPreset, fps: u32, speed: f32) -> Self {
        let cycle_seconds = preset.cycle_duration().as_secs_f64();
        let frame_count = (cycle_seconds / f64::from(speed) * f64::from(fps))
            .round()
            .max(1.0) as u32;
        Self {
            preset,
            fps,
            frame_count,
        }
    }

    pub(crate) const fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub(crate) fn sample_time(&self, frame_index: u32) -> Duration {
        debug_assert!(frame_index < self.frame_count);
        let nanos = self.preset.cycle_duration().as_nanos() * u128::from(frame_index)
            / u128::from(self.frame_count);
        Duration::from_nanos(nanos as u64)
    }

    pub(crate) fn webp_timestamp_ms(&self, frame_boundary: u32) -> u32 {
        rounded_units(frame_boundary, 1_000, self.fps)
    }

    pub(crate) fn gif_delay_cs(&self, frame_index: u32) -> u32 {
        rounded_units(frame_index + 1, 100, self.fps) - rounded_units(frame_index, 100, self.fps)
    }
}

fn rounded_units(frame_boundary: u32, units_per_second: u32, fps: u32) -> u32 {
    (frame_boundary * units_per_second + fps / 2) / fps
}

pub(crate) struct AnimationExportOptions {
    pub(crate) preset: AnimationPreset,
    pub(crate) fps: u32,
    pub(crate) speed: f32,
    pub(crate) loop_count: u16,
    pub(crate) size: (u32, u32),
}

pub(crate) fn export_animation(
    renderer: &Renderer,
    character: &mut Character,
    camera: &Camera,
    output: &Path,
    options: AnimationExportOptions,
    apply_overrides: impl Fn(Posture) -> Posture,
) -> Result<(), DynError> {
    let timing = AnimationTiming::new(options.preset, options.fps, options.speed);
    let mut render_frame = |frame_index| {
        character.posture = apply_overrides(options.preset.sample(timing.sample_time(frame_index)));
        Ok(renderer
            .render(character, camera, options.size.0, options.size.1)?
            .into_raw())
    };

    match output
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("gif") => {
            let file = BufWriter::new(File::create(output)?);
            encode_gif(
                file,
                options.size,
                &timing,
                options.loop_count,
                &mut render_frame,
            )
        }
        Some("webp") => {
            let data = encode_webp(options.size, &timing, options.loop_count, &mut render_frame)?;
            std::fs::write(output, data)?;
            Ok(())
        }
        _ => Err("animation output must end in .gif or .webp".into()),
    }
}

fn encode_gif(
    writer: impl Write,
    size: (u32, u32),
    timing: &AnimationTiming,
    loop_count: u16,
    mut render_frame: impl FnMut(u32) -> Result<Vec<u8>, DynError>,
) -> Result<(), DynError> {
    let mut encoder = GifEncoder::new(writer, size.0 as u16, size.1 as u16, &[])?;
    encoder.set_repeat(if loop_count == 0 {
        Repeat::Infinite
    } else {
        Repeat::Finite(loop_count)
    })?;

    for frame_index in 0..timing.frame_count() {
        let mut rgba = render_frame(frame_index)?;
        let mut frame = GifFrame::from_rgba_speed(size.0 as u16, size.1 as u16, &mut rgba, 10);
        frame.delay = timing.gif_delay_cs(frame_index) as u16;
        encoder.write_frame(&frame)?;
    }
    Ok(())
}

fn encode_webp(
    size: (u32, u32),
    timing: &AnimationTiming,
    loop_count: u16,
    mut render_frame: impl FnMut(u32) -> Result<Vec<u8>, DynError>,
) -> Result<Vec<u8>, DynError> {
    let options = EncoderOptions {
        anim_params: AnimParams {
            loop_count: i32::from(loop_count),
        },
        ..Default::default()
    };
    let mut encoder = WebpEncoder::new_with_options(size, options)?;

    for frame_index in 0..timing.frame_count() {
        let rgba = render_frame(frame_index)?;
        encoder.add_frame(&rgba, timing.webp_timestamp_ms(frame_index).try_into()?)?;
    }
    let data = encoder.finalize(timing.webp_timestamp_ms(timing.frame_count()).try_into()?)?;
    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn rgba_frame(frame_index: u32) -> Result<Vec<u8>, DynError> {
        let value = 40 + frame_index as u8 * 40;
        Ok([value, 20, 220, 255].repeat(4))
    }

    #[test]
    fn gif_encoder_writes_frames_timing_dimensions_and_loop_metadata() {
        let timing = AnimationTiming::new(AnimationPreset::Walk, 4, 1.0);
        let mut bytes = Vec::new();
        encode_gif(&mut bytes, (2, 2), &timing, 3, rgba_frame).unwrap();

        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(Cursor::new(bytes)).unwrap();
        assert_eq!(decoder.width(), 2);
        assert_eq!(decoder.height(), 2);
        assert_eq!(decoder.repeat(), Repeat::Finite(3));

        let mut frame_count = 0;
        let mut total_delay = 0;
        let mut has_nonzero_pixel = false;
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            frame_count += 1;
            total_delay += u32::from(frame.delay);
            has_nonzero_pixel |= frame.buffer.iter().any(|channel| *channel != 0);
        }
        assert_eq!(frame_count, 4);
        assert_eq!(total_delay, 100);
        assert!(has_nonzero_pixel);
    }

    #[test]
    fn webp_encoder_writes_lossless_rgba_frames_dimensions_and_loop_metadata() {
        let timing = AnimationTiming::new(AnimationPreset::Walk, 4, 1.0);
        let bytes = encode_webp((2, 2), &timing, 2, rgba_frame).unwrap();

        let decoder = webp_animation::Decoder::new(&bytes).unwrap();
        assert_eq!(decoder.dimensions(), (2, 2));
        assert!(format!("{decoder:?}").contains("loop_cnt: 2"));
        let frames: Vec<_> = decoder.into_iter().collect();
        assert_eq!(frames.len(), 4);
        assert!(frames
            .iter()
            .flat_map(|frame| frame.data())
            .any(|channel| *channel != 0));
    }
}
