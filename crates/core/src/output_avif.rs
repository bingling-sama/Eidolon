//! Internal AVIF animation frame preparation and zero-copy slice borrowing helpers.
//!
//! Kept crate-private (never re-exported as public API) and shared directly with
//! `tests/memory_allocation.rs` so allocation and pointer assertions exercise the
//! production code path without exposing internal encoder types.

#[cfg(not(target_arch = "wasm32"))]
use crate::animation::AnimationSchedule;
use crate::error::EidolonError;
use image::RgbaImage;
#[cfg(not(target_arch = "wasm32"))]
use rgb::FromSlice;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

pub(crate) fn validate_frame(
    image: &RgbaImage,
    (width, height): (u32, u32),
) -> Result<(), EidolonError> {
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

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn rounded_units(frame_boundary: u32, units_per_second: u32, fps: u32) -> u32 {
    (frame_boundary * units_per_second + fps / 2) / fps
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn frame_timestamp_ms(frame_boundary: u32, fps: u32) -> u32 {
    rounded_units(frame_boundary, 1_000, fps)
}

/// Prepared AVIF animation frame buffers paired with their target frame dimensions and durations.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct PreparedAvifAnimation {
    size: (u32, u32),
    raw_frames: Vec<Vec<u8>>,
    frame_durations: Vec<u32>,
}

#[cfg(not(target_arch = "wasm32"))]
impl PreparedAvifAnimation {
    /// Collects and validates raw frames, computing accumulated rounded frame durations.
    pub(crate) fn collect_frames<F>(
        size: (u32, u32),
        schedule: &AnimationSchedule,
        mut frame_producer: F,
    ) -> Result<Self, EidolonError>
    where
        F: FnMut(Duration) -> Result<RgbaImage, EidolonError>,
    {
        let frame_count = schedule.frame_count() as usize;
        let mut raw_frames = Vec::with_capacity(frame_count);
        let mut frame_durations = Vec::with_capacity(frame_count);

        for frame_index in 0..schedule.frame_count() {
            let image = frame_producer(schedule.sample_time(frame_index))?;
            validate_frame(&image, size)?;
            let duration_ms = frame_timestamp_ms(frame_index + 1, schedule.fps())
                - frame_timestamp_ms(frame_index, schedule.fps());
            frame_durations.push(duration_ms);

            raw_frames.push(image.into_raw());
        }

        Ok(Self {
            size,
            raw_frames,
            frame_durations,
        })
    }

    /// Constructs zero-copy encoder input views borrowing directly from the backing raw pixel buffers.
    ///
    /// Performs 0 pixel allocations: borrows `&[zenravif::RGBA8]` slices directly via `as_rgba()`.
    #[inline]
    pub(crate) fn to_anim_frames(&self) -> Vec<zenravif::AnimFrameRgba<'_>> {
        let mut anim_frames = Vec::with_capacity(self.raw_frames.len());
        let width = self.size.0 as usize;
        let height = self.size.1 as usize;
        for (raw, &duration_ms) in self.raw_frames.iter().zip(self.frame_durations.iter()) {
            let rgba_slice: &[zenravif::RGBA8] = raw.as_slice().as_rgba();
            anim_frames.push(zenravif::AnimFrameRgba {
                rgba: zenravif::Img::new(rgba_slice, width, height),
                duration_ms,
            });
        }
        anim_frames
    }
}
