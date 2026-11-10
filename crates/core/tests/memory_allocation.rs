//! Automated test to quantify and verify heap allocation reduction for AVIF export (Issue #24).

#![cfg(not(target_arch = "wasm32"))]

use rgb::FromSlice;
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

#[test]
fn avif_single_frame_zero_copy_performs_zero_heap_allocations() {
    let width = 800;
    let height = 600;
    let sample_raw = vec![0u8; width * height * 4];

    // Measure zero-copy slice borrow
    ALLOCATOR.start_tracking();
    let rgba_slice: &[zenravif::RGBA8] = sample_raw.as_slice().as_rgba();
    std::hint::black_box(rgba_slice);
    let (zc_allocs, zc_bytes) = ALLOCATOR.stop_tracking();

    // Measure legacy chunk allocation
    ALLOCATOR.start_tracking();
    let rgba_vec: Vec<zenravif::RGBA8> = sample_raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| zenravif::RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
        .collect();
    std::hint::black_box(rgba_vec);
    let (legacy_allocs, legacy_bytes) = ALLOCATOR.stop_tracking();

    // Empirical verification
    assert_eq!(
        zc_allocs, 0,
        "Zero-copy slice borrowing must perform 0 heap allocations"
    );
    assert_eq!(
        zc_bytes, 0,
        "Zero-copy slice borrowing must allocate 0 bytes on the heap"
    );

    assert!(
        legacy_allocs >= 1,
        "Legacy chunk iteration must allocate on the heap"
    );
    assert_eq!(
        legacy_bytes,
        width * height * 4,
        "Legacy allocation must allocate exactly width*height*4 bytes ({})",
        width * height * 4
    );
}

#[test]
fn avif_animation_frame_prep_pipeline_quantifies_memory_reduction() {
    let frame_count = 15;
    let width = 400;
    let height = 300;
    let raw_frames: Vec<Vec<u8>> = (0..frame_count)
        .map(|_| vec![0u8; width * height * 4])
        .collect();

    // 1. Zero-copy pipeline: borrow slices into AnimFrameRgba
    ALLOCATOR.start_tracking();
    let mut zc_frames: Vec<zenravif::AnimFrameRgba> = Vec::with_capacity(frame_count);
    for raw in &raw_frames {
        let rgba_slice: &[zenravif::RGBA8] = raw.as_slice().as_rgba();
        zc_frames.push(zenravif::AnimFrameRgba {
            rgba: zenravif::Img::new(rgba_slice, width, height),
            duration_ms: 66,
        });
    }
    std::hint::black_box(&zc_frames);
    let (zc_allocs, zc_bytes) = ALLOCATOR.stop_tracking();

    // 2. Legacy pipeline: allocate Vec<RGBA8> for each frame
    ALLOCATOR.start_tracking();
    let mut legacy_buffers: Vec<Vec<zenravif::RGBA8>> = Vec::with_capacity(frame_count);
    for raw in &raw_frames {
        let rgba_vec: Vec<zenravif::RGBA8> = raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| zenravif::RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
            .collect();
        legacy_buffers.push(rgba_vec);
    }
    let mut legacy_frames: Vec<zenravif::AnimFrameRgba> = Vec::with_capacity(frame_count);
    for buf in &legacy_buffers {
        legacy_frames.push(zenravif::AnimFrameRgba {
            rgba: zenravif::Img::new(buf.as_slice(), width, height),
            duration_ms: 66,
        });
    }
    std::hint::black_box(&legacy_frames);
    let (legacy_allocs, legacy_bytes) = ALLOCATOR.stop_tracking();

    let expected_frame_heap_bytes = frame_count * width * height * 4;
    let expected_vector_header_bytes = frame_count * std::mem::size_of::<Vec<zenravif::RGBA8>>();
    assert_eq!(
        legacy_bytes.saturating_sub(zc_bytes),
        expected_frame_heap_bytes + expected_vector_header_bytes,
        "Legacy pipeline must allocate pixel buffers ({}) plus Vec headers ({}) more bytes than zero-copy across {} frames",
        expected_frame_heap_bytes,
        expected_vector_header_bytes,
        frame_count
    );
    assert_eq!(
        zc_allocs, 1,
        "Zero-copy pipeline only performs 1 allocation for the outer AnimFrameRgba slice vector"
    );
    assert_eq!(
        legacy_allocs,
        frame_count + 2,
        "Legacy pipeline performs {} per-frame buffer allocations plus 2 vector container allocations",
        frame_count
    );
}

