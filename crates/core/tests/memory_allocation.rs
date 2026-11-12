//! Automated verification and memory allocation quantification for AVIF export (Issue #24).
//!
//! Evaluates the shared internal production implementation (`output_avif::PreparedAvifAnimation`)
//! in an isolated integration-test process for `#[global_allocator]` hygiene:
//! - Verifies encoder input views borrow backing raw buffers directly (pointer identity,
//!   dimensions, channel values, and frame `duration_ms`).
//! - Quantifies allocation volume during production frame preparation under symmetric A/B
//!   container boundaries.
//!
//! Note: End-to-end CPU export & ISOBMFF container verification lives in `crates/core/src/output.rs::tests`.

#![cfg(not(target_arch = "wasm32"))]

// Re-export public submodules so the shared private `output_avif.rs` resolves identical
// `crate::animation` and `crate::error` paths here and inside `eidolon_core`.
use eidolon_core::{animation, error};

#[path = "../src/output_avif.rs"]
mod output_avif;

use animation::{AnimationPreset, AnimationSchedule};
use error::EidolonError;
use image::{Rgba, RgbaImage};
use output_avif::{frame_timestamp_ms, validate_frame, PreparedAvifAnimation};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default)]
struct AllocationStats {
    alloc_calls: usize,
    alloc_bytes: usize,
    alloc_zeroed_calls: usize,
    alloc_zeroed_bytes: usize,
    realloc_calls: usize,
    realloc_expansion_bytes: usize,
}

impl AllocationStats {
    fn total_calls(&self) -> usize {
        self.alloc_calls + self.alloc_zeroed_calls + self.realloc_calls
    }

    fn total_requested_bytes(&self) -> usize {
        self.alloc_bytes + self.alloc_zeroed_bytes + self.realloc_expansion_bytes
    }
}

/// Zero-overhead tracking allocator instrumenting heap allocations during dedicated test windows.
struct TrackingAllocator {
    tracking: AtomicBool,
    alloc_calls: AtomicUsize,
    alloc_bytes: AtomicUsize,
    alloc_zeroed_calls: AtomicUsize,
    alloc_zeroed_bytes: AtomicUsize,
    realloc_calls: AtomicUsize,
    realloc_expansion_bytes: AtomicUsize,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            tracking: AtomicBool::new(false),
            alloc_calls: AtomicUsize::new(0),
            alloc_bytes: AtomicUsize::new(0),
            alloc_zeroed_calls: AtomicUsize::new(0),
            alloc_zeroed_bytes: AtomicUsize::new(0),
            realloc_calls: AtomicUsize::new(0),
            realloc_expansion_bytes: AtomicUsize::new(0),
        }
    }

    fn start_tracking(&self) {
        self.alloc_calls.store(0, Ordering::SeqCst);
        self.alloc_bytes.store(0, Ordering::SeqCst);
        self.alloc_zeroed_calls.store(0, Ordering::SeqCst);
        self.alloc_zeroed_bytes.store(0, Ordering::SeqCst);
        self.realloc_calls.store(0, Ordering::SeqCst);
        self.realloc_expansion_bytes.store(0, Ordering::SeqCst);
        self.tracking.store(true, Ordering::SeqCst);
    }

    fn stop_tracking(&self) -> AllocationStats {
        self.tracking.store(false, Ordering::SeqCst);
        AllocationStats {
            alloc_calls: self.alloc_calls.load(Ordering::SeqCst),
            alloc_bytes: self.alloc_bytes.load(Ordering::SeqCst),
            alloc_zeroed_calls: self.alloc_zeroed_calls.load(Ordering::SeqCst),
            alloc_zeroed_bytes: self.alloc_zeroed_bytes.load(Ordering::SeqCst),
            realloc_calls: self.realloc_calls.load(Ordering::SeqCst),
            realloc_expansion_bytes: self.realloc_expansion_bytes.load(Ordering::SeqCst),
        }
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() && self.tracking.load(Ordering::Relaxed) {
            self.alloc_bytes.fetch_add(layout.size(), Ordering::Relaxed);
            self.alloc_calls.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() && self.tracking.load(Ordering::Relaxed) {
            self.alloc_zeroed_bytes
                .fetch_add(layout.size(), Ordering::Relaxed);
            self.alloc_zeroed_calls.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() && self.tracking.load(Ordering::Relaxed) {
            if new_size > layout.size() {
                self.realloc_expansion_bytes
                    .fetch_add(new_size - layout.size(), Ordering::Relaxed);
            }
            self.realloc_calls.fetch_add(1, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

/// Computes an explicit metadata container allocation budget for `PreparedAvifAnimation`.
fn expected_prep_metadata_budget(frame_count: usize) -> usize {
    let raw_frames_vec = frame_count * std::mem::size_of::<Vec<u8>>();
    let durations_vec = frame_count * std::mem::size_of::<u32>();
    let anim_frames_vec = frame_count * std::mem::size_of::<zenravif::AnimFrameRgba<'_>>();
    raw_frames_vec + durations_vec + anim_frames_vec + 3 * 128
}

/// Legacy baseline mirroring pre-optimization `write_animation` AVIF collection and pixel copy
/// with symmetric container boundaries (frame container + duration list + encoder view list).
struct LegacyAvifAnimation {
    frames_data: Vec<Vec<zenravif::RGBA8>>,
    frame_durations: Vec<u32>,
}

impl LegacyAvifAnimation {
    fn collect_frames<F>(
        size: (u32, u32),
        schedule: &AnimationSchedule,
        mut frame_producer: F,
    ) -> Result<Self, EidolonError>
    where
        F: FnMut(Duration) -> Result<RgbaImage, EidolonError>,
    {
        let frame_count = schedule.frame_count() as usize;
        let mut frames_data = Vec::with_capacity(frame_count);
        let mut frame_durations = Vec::with_capacity(frame_count);

        for frame_index in 0..schedule.frame_count() {
            let image = frame_producer(schedule.sample_time(frame_index))?;
            validate_frame(&image, size)?;
            let duration_ms = frame_timestamp_ms(frame_index + 1, schedule.fps())
                - frame_timestamp_ms(frame_index, schedule.fps());
            frame_durations.push(duration_ms);

            let raw = image.into_raw();
            let rgba_vec: Vec<zenravif::RGBA8> = raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|chunk| zenravif::RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
                .collect();
            frames_data.push(rgba_vec);
        }

        Ok(Self {
            frames_data,
            frame_durations,
        })
    }

    fn to_anim_frames(&self, size: (u32, u32)) -> Vec<zenravif::AnimFrameRgba<'_>> {
        let mut anim_frames = Vec::with_capacity(self.frames_data.len());
        let width = size.0 as usize;
        let height = size.1 as usize;
        for (data, &duration_ms) in self.frames_data.iter().zip(self.frame_durations.iter()) {
            anim_frames.push(zenravif::AnimFrameRgba {
                rgba: zenravif::Img::new(data.as_slice(), width, height),
                duration_ms,
            });
        }
        anim_frames
    }
}

/// Verifies encoder input views directly borrow backing buffers with exact geometry, timing, and pixels.
fn assert_pointer_and_view_invariance(
    width: u32,
    height: u32,
    prepared: &PreparedAvifAnimation,
    expected_ptrs: &[*const u8],
    expected_durations: &[u32],
    expected_images: &[RgbaImage],
) {
    let anim_frames = prepared.to_anim_frames();
    assert_eq!(
        anim_frames.len(),
        expected_ptrs.len(),
        "Frame count must match expected"
    );
    assert_eq!(
        expected_durations.len(),
        expected_ptrs.len(),
        "Duration count must match frame count"
    );

    for (i, frame) in anim_frames.iter().enumerate() {
        assert_eq!(
            frame.rgba.width(),
            width as usize,
            "Frame {i} width must match target width"
        );
        assert_eq!(
            frame.rgba.height(),
            height as usize,
            "Frame {i} height must match target height"
        );
        assert_eq!(
            frame.duration_ms, expected_durations[i],
            "Frame {i} duration_ms must match cumulative-rounded schedule duration"
        );

        let slice: &[zenravif::RGBA8] = frame.rgba.buf();
        assert_eq!(
            slice.len(),
            (width * height) as usize,
            "Frame {i} slice length must equal width * height"
        );

        // Core pointer identity assertion: slice pointer MUST equal original backing buffer pointer
        let slice_ptr = slice.as_ptr() as *const u8;
        assert_eq!(
            slice_ptr, expected_ptrs[i],
            "Zero-copy regression: Frame {i} slice does not borrow backing buffer directly!"
        );

        // Value & channel order preservation check
        let original_raw = expected_images[i].as_raw();
        for (p_idx, rgba) in slice.iter().enumerate() {
            let offset = p_idx * 4;
            assert_eq!(rgba.r, original_raw[offset]);
            assert_eq!(rgba.g, original_raw[offset + 1]);
            assert_eq!(rgba.b, original_raw[offset + 2]);
            assert_eq!(rgba.a, original_raw[offset + 3]);
        }
    }
}

fn expected_schedule_durations(schedule: &AnimationSchedule) -> Vec<u32> {
    (0..schedule.frame_count())
        .map(|i| frame_timestamp_ms(i + 1, schedule.fps()) - frame_timestamp_ms(i, schedule.fps()))
        .collect()
}

#[test]
fn avif_frame_prep_zero_copy_and_allocations() {
    // ------------------------------------------------------------------------
    // Part A: Single-Frame Verification (800x600)
    // ------------------------------------------------------------------------
    let width = 800;
    let height = 600;
    let pixel_bytes = (width * height * 4) as usize;
    let schedule_1f = AnimationSchedule::new(AnimationPreset::Walk, 1, 1.0).unwrap();
    let expected_durations_1f = expected_schedule_durations(&schedule_1f);

    // 1. Pre-create input image before measurement window
    let mut img_1f = RgbaImage::new(width, height);
    for (x, y, pixel) in img_1f.enumerate_pixels_mut() {
        *pixel = Rgba([(x % 256) as u8, (y % 256) as u8, 120, 255]);
    }
    let expected_ptr_1f = img_1f.as_raw().as_ptr();
    let expected_img_1f = img_1f.clone();
    let mut source_1f = Some(img_1f);

    // 2. Measure production frame preparation window
    ALLOCATOR.start_tracking();
    let prepared_1f = PreparedAvifAnimation::collect_frames((width, height), &schedule_1f, |_| {
        Ok(source_1f.take().expect("Single frame requested once"))
    })
    .expect("collect_frames must succeed");
    let anim_frames_1f = prepared_1f.to_anim_frames();
    std::hint::black_box(&anim_frames_1f);
    let zc_stats_1f = ALLOCATOR.stop_tracking();
    drop(anim_frames_1f);

    // 3. Pointer, dimension, duration, and pixel invariance verification
    assert_pointer_and_view_invariance(
        width,
        height,
        &prepared_1f,
        &[expected_ptr_1f],
        &expected_durations_1f,
        &[expected_img_1f],
    );

    // 4. Allocation verification: only bounded pre-allocated metadata containers, zero pixel buffers
    let budget_1f = expected_prep_metadata_budget(1);
    assert_eq!(
        zc_stats_1f.realloc_calls, 0,
        "Pre-allocated zero-copy prep must not trigger reallocations"
    );
    assert_eq!(
        zc_stats_1f.alloc_zeroed_calls, 0,
        "Zero-copy prep must not zero-allocate buffers"
    );
    assert!(
        zc_stats_1f.total_requested_bytes() <= budget_1f && zc_stats_1f.total_requested_bytes() < pixel_bytes,
        "Zero-copy prep bytes ({} B) must stay within metadata budget ({budget_1f} B) and below 1 pixel buffer ({pixel_bytes} B)",
        zc_stats_1f.total_requested_bytes()
    );
    assert!(
        zc_stats_1f.total_calls() <= 3,
        "Zero-copy frame prep allocates only the 3 metadata containers (got {})",
        zc_stats_1f.total_calls()
    );

    // ------------------------------------------------------------------------
    // Part B: Multi-Frame Symmetric A/B Verification (15 frames, 400x300, 15fps)
    // ------------------------------------------------------------------------
    let frame_count = 15;
    let width_15f = 400;
    let height_15f = 300;
    let frame_pixel_bytes = (width_15f * height_15f * 4) as usize;
    let total_pixel_bytes = frame_count * frame_pixel_bytes;
    let schedule_15f = AnimationSchedule::new(AnimationPreset::Walk, 15, 1.0).unwrap();
    let expected_durations_15f = expected_schedule_durations(&schedule_15f);

    // 1. Pre-create 15 distinct input images with varying RGB and alpha values before tracking
    let images_15f: Vec<RgbaImage> = (0..frame_count)
        .map(|idx| {
            let alpha = match idx % 3 {
                0 => 0,
                1 => 128,
                _ => 255,
            };
            RgbaImage::from_pixel(
                width_15f,
                height_15f,
                Rgba([(idx * 16) as u8, (idx * 8) as u8, 200, alpha]),
            )
        })
        .collect();
    let expected_ptrs_15f: Vec<*const u8> =
        images_15f.iter().map(|img| img.as_raw().as_ptr()).collect();
    let expected_images_15f = images_15f.clone();
    let mut zc_source_iter = images_15f.into_iter();

    // 2. Measure production zero-copy pipeline window
    ALLOCATOR.start_tracking();
    let prepared_15f =
        PreparedAvifAnimation::collect_frames((width_15f, height_15f), &schedule_15f, |_| {
            Ok(zc_source_iter.next().expect("frame count matches schedule"))
        })
        .expect("collect_frames must succeed");
    let anim_frames_15f = prepared_15f.to_anim_frames();
    std::hint::black_box(&anim_frames_15f);
    let zc_stats_15f = ALLOCATOR.stop_tracking();
    drop(anim_frames_15f);

    // 3. View & pointer invariance verification across all 15 frames
    assert_pointer_and_view_invariance(
        width_15f,
        height_15f,
        &prepared_15f,
        &expected_ptrs_15f,
        &expected_durations_15f,
        &expected_images_15f,
    );

    // 4. Symmetric legacy baseline measurement (identical inputs & 3-container structure in window)
    let mut legacy_source_iter = expected_images_15f.into_iter();
    ALLOCATOR.start_tracking();
    let legacy_15f =
        LegacyAvifAnimation::collect_frames((width_15f, height_15f), &schedule_15f, |_| {
            Ok(legacy_source_iter
                .next()
                .expect("frame count matches schedule"))
        })
        .expect("legacy collect_frames must succeed");
    let legacy_anim_frames = legacy_15f.to_anim_frames((width_15f, height_15f));
    std::hint::black_box(&legacy_anim_frames);
    let legacy_stats_15f = ALLOCATOR.stop_tracking();
    drop(legacy_anim_frames);

    // 5. Empirical comparison and regression assertions
    let budget_15f = expected_prep_metadata_budget(frame_count);
    assert_eq!(
        zc_stats_15f.realloc_calls, 0,
        "Multi-frame zero-copy prep must not trigger reallocations"
    );
    assert!(
        zc_stats_15f.total_requested_bytes() <= budget_15f
            && zc_stats_15f.total_requested_bytes() < frame_pixel_bytes,
        "Multi-frame zero-copy bytes ({} B) must stay within metadata budget ({budget_15f} B) and below 1 pixel buffer ({frame_pixel_bytes} B)",
        zc_stats_15f.total_requested_bytes()
    );

    assert!(
        legacy_stats_15f.total_requested_bytes() >= total_pixel_bytes,
        "Legacy baseline must allocate at least {frame_count} pixel buffers ({total_pixel_bytes} B)"
    );

    let net_memory_saved = legacy_stats_15f
        .total_requested_bytes()
        .saturating_sub(zc_stats_15f.total_requested_bytes());
    assert!(
        net_memory_saved >= total_pixel_bytes,
        "Zero-copy eliminates at least {total_pixel_bytes} B of pixel heap allocation (saved {net_memory_saved} B)"
    );

    assert!(
        legacy_stats_15f.total_calls() >= zc_stats_15f.total_calls() + frame_count,
        "Symmetric legacy pipeline must perform at least {frame_count} extra pixel buffer allocations \
         (legacy total_calls={}, zero-copy total_calls={})",
        legacy_stats_15f.total_calls(),
        zc_stats_15f.total_calls()
    );
}
