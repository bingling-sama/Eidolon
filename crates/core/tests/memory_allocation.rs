//! Automated verification and memory allocation quantification for AVIF export (Issue #24).
//!
//! Evaluates the shared production implementation (`PreparedAvifAnimation`) against
//! maintainer @bingling-sama's test design requirements:
//! - T1: Verifies encoder input views directly borrow backing raw buffers via pointer invariance.
//! - T2: Quantifies allocation volume during production frame preparation under strict boundaries.
//!
//! Note: T3 (end-to-end CPU export correctness) is implemented in `crates/core/src/output.rs::tests`.

#![cfg(not(target_arch = "wasm32"))]

use eidolon_core::animation::{AnimationPreset, AnimationSchedule};
use eidolon_core::output::PreparedAvifAnimation;
use image::{Rgba, RgbaImage};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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

/// Verifies T1: Input views directly borrow raw buffers (pointer invariance and pixel fidelity).
fn assert_pointer_and_pixel_invariance(
    width: u32,
    height: u32,
    prepared: &PreparedAvifAnimation,
    expected_ptrs: &[*const u8],
    expected_images: &[RgbaImage],
) {
    let anim_frames = prepared.to_anim_frames((width, height));
    assert_eq!(
        anim_frames.len(),
        expected_ptrs.len(),
        "Frame count must match expected"
    );

    for (i, frame) in anim_frames.iter().enumerate() {
        let slice: &[zenravif::RGBA8] = frame.rgba.buf;
        assert_eq!(
            slice.len(),
            (width * height) as usize,
            "Frame {i} slice length must equal width * height"
        );

        // Core pointer identity assertion: slice pointer MUST equal original backing buffer pointer
        let slice_ptr = slice.as_ptr() as *const u8;
        assert_eq!(
            slice_ptr, expected_ptrs[i],
            "T1 Regression Failure: Frame {i} slice does not borrow backing buffer directly!"
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

#[test]
fn avif_t1_and_t2_production_pipeline_verification() {
    // ------------------------------------------------------------------------
    // Part A: T1 & T2 Single-Frame Verification (800x600)
    // ------------------------------------------------------------------------
    let width = 800;
    let height = 600;
    let pixel_bytes = (width * height * 4) as usize;
    let schedule_1f = AnimationSchedule::new(AnimationPreset::Walk, 1, 1.0).unwrap();

    // 1. Pre-create input image before measurement window
    let mut img_1f = RgbaImage::new(width, height);
    for (x, y, pixel) in img_1f.enumerate_pixels_mut() {
        *pixel = Rgba([(x % 256) as u8, (y % 256) as u8, 120, 255]);
    }
    let expected_ptr_1f = img_1f.as_raw().as_ptr();
    let expected_img_1f = img_1f.clone();

    // 2. Measure production frame preparation window (T2)
    ALLOCATOR.start_tracking();
    let mut source_1f = Some(img_1f);
    let prepared_1f = PreparedAvifAnimation::collect_frames((width, height), &schedule_1f, |_| {
        Ok(source_1f.take().expect("Single frame requested once"))
    })
    .expect("collect_frames must succeed");
    let anim_frames_1f = prepared_1f.to_anim_frames((width, height));
    std::hint::black_box(&anim_frames_1f);
    let zc_stats_1f = ALLOCATOR.stop_tracking();

    // 3. T1 Verification: Assert pointer invariance and pixel values
    assert_pointer_and_pixel_invariance(
        width,
        height,
        &prepared_1f,
        &[expected_ptr_1f],
        &[expected_img_1f],
    );

    // 4. T2 Allocation assertion: zero pixel buffer allocations
    // Total requested bytes must be orders of magnitude below a single pixel buffer (1,920,000 B)
    assert!(
        zc_stats_1f.total_requested_bytes() < pixel_bytes,
        "Production frame prep must not allocate pixel buffers (allocated {} bytes, frame is {} bytes)",
        zc_stats_1f.total_requested_bytes(),
        pixel_bytes
    );
    // Specifically, only container metadata is allocated (raw_frames + durations + anim_frames)
    assert!(
        zc_stats_1f.total_calls() <= 3,
        "Zero-copy frame prep only allocates necessary metadata containers"
    );

    // ------------------------------------------------------------------------
    // Part B: T1 & T2 Multi-Frame Verification (15 frames, 400x300, 15fps)
    // ------------------------------------------------------------------------
    let frame_count = 15;
    let width_15f = 400;
    let height_15f = 300;
    let frame_pixel_bytes = (width_15f * height_15f * 4) as usize;
    let total_pixel_bytes = frame_count * frame_pixel_bytes;
    let schedule_15f = AnimationSchedule::new(AnimationPreset::Walk, 15, 1.0).unwrap();

    // 1. Pre-create 15 distinct input images with varying RGB and alpha values
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

    // 2. Measure production zero-copy pipeline window (T2)
    ALLOCATOR.start_tracking();
    let mut source_15f = images_15f;
    let prepared_15f =
        PreparedAvifAnimation::collect_frames((width_15f, height_15f), &schedule_15f, |_| {
            Ok(source_15f.remove(0))
        })
        .expect("collect_frames must succeed");
    let anim_frames_15f = prepared_15f.to_anim_frames((width_15f, height_15f));
    std::hint::black_box(&anim_frames_15f);
    let zc_stats_15f = ALLOCATOR.stop_tracking();

    // 3. T1 Verification: Check all 15 frames for pointer invariance & channel order
    assert_pointer_and_pixel_invariance(
        width_15f,
        height_15f,
        &prepared_15f,
        &expected_ptrs_15f,
        &expected_images_15f,
    );

    // 4. Measure legacy baseline under identical inputs and symmetric container boundaries
    let mut legacy_source: Vec<RgbaImage> =
        expected_images_15f.iter().map(|img| img.clone()).collect();
    let legacy_durations = prepared_15f.frame_durations.clone();
    ALLOCATOR.start_tracking();
    let mut legacy_raw: Vec<Vec<u8>> = Vec::with_capacity(frame_count);
    for _ in 0..frame_count {
        let img = legacy_source.remove(0);
        legacy_raw.push(img.into_raw());
    }
    // Legacy pixel copy loop into Vec<RGBA8>
    let mut legacy_buffers: Vec<Vec<zenravif::RGBA8>> = Vec::with_capacity(frame_count);
    for raw in &legacy_raw {
        let rgba_vec: Vec<zenravif::RGBA8> = raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| zenravif::RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
            .collect();
        legacy_buffers.push(rgba_vec);
    }
    let mut legacy_anim_frames: Vec<zenravif::AnimFrameRgba> = Vec::with_capacity(frame_count);
    for (buf, &duration_ms) in legacy_buffers.iter().zip(legacy_durations.iter()) {
        legacy_anim_frames.push(zenravif::AnimFrameRgba {
            rgba: zenravif::Img::new(buf.as_slice(), width_15f as usize, height_15f as usize),
            duration_ms,
        });
    }
    std::hint::black_box(&legacy_anim_frames);
    let legacy_stats_15f = ALLOCATOR.stop_tracking();

    // 5. Empirical comparison and regression assertions
    // Zero-copy requested bytes MUST exclude pixel buffers (total < 1 frame's pixel buffer)
    assert!(
        zc_stats_15f.total_requested_bytes() < frame_pixel_bytes,
        "Multi-frame zero-copy must not allocate pixel buffers (total allocated: {} B, 1 frame is {} B)",
        zc_stats_15f.total_requested_bytes(),
        frame_pixel_bytes
    );

    // Legacy requested bytes must include all 15 pixel buffers (>= 7.20 MB)
    assert!(
        legacy_stats_15f.total_requested_bytes() >= total_pixel_bytes,
        "Legacy baseline must allocate at least 15 pixel buffers ({total_pixel_bytes} B)"
    );

    // Difference between legacy and zero-copy must be >= the 15 frame pixel buffers (7.20 MB)
    let net_memory_saved = legacy_stats_15f
        .total_requested_bytes()
        .saturating_sub(zc_stats_15f.total_requested_bytes());
    assert!(
        net_memory_saved >= total_pixel_bytes,
        "Zero-copy eliminates at least {total_pixel_bytes} bytes of pixel heap allocation (saved {net_memory_saved} B)"
    );

    // Allocation calls: legacy performs 15 extra pixel buffer allocations
    assert_eq!(
        legacy_stats_15f
            .alloc_calls
            .saturating_sub(zc_stats_15f.alloc_calls),
        frame_count,
        "Legacy pipeline performs exactly {frame_count} extra heap allocations for pixel buffers"
    );
}
