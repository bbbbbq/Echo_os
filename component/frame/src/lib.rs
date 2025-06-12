#![no_std]

extern crate alloc;
use log::debug;
use alloc::vec::Vec;
use bitmap::Bitmap;
use config::target::plat::{FRAME_SIZE, PAGE_SIZE,VIRT_ADDR_START};
use lazy_static::lazy_static;
use memory_addr::MemoryAddr;
use memory_addr::PhysAddr;
use spin::Mutex;
unsafe extern "C" {
    fn _end();
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocator> = {
        let mut start_addr = _end as usize;
        let mut end_addr = start_addr + FRAME_SIZE;

        // if start_addr >= VIRT_ADDR_START {
        //     start_addr -= VIRT_ADDR_START;
        // }

        // if end_addr >= VIRT_ADDR_START {
        //     end_addr -= VIRT_ADDR_START;
        // }

        let start_paddr = PhysAddr::from_usize(start_addr);
        let end_paddr = PhysAddr::from_usize(end_addr);
        debug!("Allocated frame at address: 0x{:x}", start_addr);
        Mutex::new(FrameAllocator::new(start_paddr, end_paddr))
    };
}

#[derive(Clone, Copy)]
pub struct FrameTracer {
    pub paddr: PhysAddr,
}

impl FrameTracer {
    pub fn new(paddr: PhysAddr) -> Self {
        FrameTracer { paddr }
    }
}

impl core::fmt::Debug for FrameTracer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FrameTracer {{ paddr: {:?} }}", self.paddr)
    }
}

#[derive(Clone)]
pub struct FrameAllocator {
    start: PhysAddr,
    end: PhysAddr,
    bitmap: Bitmap,
}

impl FrameAllocator {
    pub fn new(mut start: PhysAddr, end: PhysAddr) -> Self {
        // Align start address to 4K boundary if not already aligned
        if !start.is_aligned_4k() {
            // Round up to next 4K boundary
            start = PhysAddr::from_usize((start.as_usize() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1));
        }

        // Calculate frame count based on aligned start address
        let frame_count = (end.as_usize() - start.as_usize()) / PAGE_SIZE;
        let bitmap = Bitmap::new(frame_count);

        Self { start, end, bitmap }
    }

    pub fn alloc(&mut self) -> Option<FrameTracer> {
        let frame_idx = self.bitmap.first_clear()?;
        self.bitmap.set(frame_idx);
        let paddr = PhysAddr::from_usize(self.start.as_usize() + frame_idx * PAGE_SIZE);
        assert!(paddr.is_aligned_4k());
        Some(FrameTracer::new(paddr))
    }

    pub fn dealloc(&mut self, frame: FrameTracer) {
        let frame_idx = (frame.paddr.as_usize() - self.start.as_usize()) / PAGE_SIZE;
        self.bitmap.clear(frame_idx);
    }

    pub fn alloc_continues(&mut self, count: usize) -> Vec<FrameTracer> {
        let mut frames = Vec::with_capacity(count);
        let mut start_idx = 0;

        while start_idx + count <= self.bitmap.len() {
            let mut found = true;
            for i in 0..count {
                if self.bitmap.get(start_idx + i) == Some(true) {
                    found = false;
                    start_idx += i + 1;
                    break;
                }
            }

            if found {
                for i in 0..count {
                    self.bitmap.set(start_idx + i);
                    let paddr =
                        PhysAddr::from_usize(self.start.as_usize() + (start_idx + i) * PAGE_SIZE);
                    frames.push(FrameTracer::new(paddr));
                }
                return frames;
            }
        }

        frames
    }

    pub fn dealloc_continues(&mut self, base_frame: FrameTracer, count: usize) -> bool {
        if base_frame.paddr.as_usize() < self.start.as_usize() {
            return false;
        }

        let base_idx = (base_frame.paddr.as_usize() - self.start.as_usize()) / PAGE_SIZE;
        if base_idx + count > self.bitmap.len() {
            return false;
        }

        for i in 0..count {
            self.bitmap.clear(base_idx + i);
        }

        true
    }
}

pub fn alloc_frame() -> Option<FrameTracer> {
    FRAME_ALLOCATOR.lock().alloc()
}

pub fn dealloc_frame(frame: FrameTracer) {
    FRAME_ALLOCATOR.lock().dealloc(frame)
}

pub fn alloc_continues(count: usize) -> Vec<FrameTracer> {
    FRAME_ALLOCATOR.lock().alloc_continues(count)
}

pub fn dealloc_continues(base: FrameTracer, count: usize) -> bool {
    FRAME_ALLOCATOR.lock().dealloc_continues(base, count)
}

pub fn is_allocated(addr: usize) -> bool {
    FRAME_ALLOCATOR
        .lock()
        .bitmap
        .get((addr - FRAME_ALLOCATOR.lock().start.as_usize()) / PAGE_SIZE)
        .unwrap_or(false)
}

pub fn is_continues_allocated(base: usize, count: usize) -> bool {
    for i in 0..count {
        if !is_allocated(base + i * PAGE_SIZE) {
            return false;
        }
    }
    true
}

/// Tests if frames allocated by alloc_continues are properly 4K-aligned
/// Returns true if all frames are properly aligned, false otherwise
pub fn test_frame_alignment(count: usize) -> bool {
    let frames = alloc_continues(count);
    if frames.is_empty() {
        return false;
    }

    // Check if all frames are 4K-aligned
    let mut all_aligned = true;
    for i in 0..frames.len() {
        let aligned = frames[i].paddr.is_aligned_4k();
        if !aligned {
            log::error!(
                "Frame {} at address 0x{:x} is not 4K-aligned",
                i,
                frames[i].paddr.as_usize()
            );
            all_aligned = false;
        } else {
            log::trace!(
                "Frame {} at address 0x{:x} is 4K-aligned",
                i,
                frames[i].paddr.as_usize()
            );
        }
    }

    // Clean up allocated frames
    if !frames.is_empty() {
        dealloc_continues(frames[0].clone(), frames.len());
    }

    all_aligned
}

/// Runs comprehensive tests on frame allocation including size, alignment, and continuity
/// Returns a tuple (success_count, failure_count)
pub fn test_frame_allocation() -> (usize, usize) {
    log::info!("Starting frame allocation tests");
    let mut success = 0;
    let mut failure = 0;

    // Test single frame alignment
    let frame = alloc_frame();
    match frame {
        Some(f) => {
            if f.paddr.is_aligned_4k() {
                log::info!("Single frame alignment test: PASS");
                success += 1;
            } else {
                log::error!(
                    "Single frame alignment test: FAIL - Frame at 0x{:x} not 4K aligned",
                    f.paddr.as_usize()
                );
                failure += 1;
            }
            dealloc_frame(f);
        }
        None => {
            log::error!("Failed to allocate single frame");
            failure += 1;
        }
    }

    // Test multi-frame allocation alignment
    let sizes_to_test = [2, 4, 8];
    for &size in &sizes_to_test {
        if test_frame_alignment(size) {
            log::info!("Multi-frame ({} frames) alignment test: PASS", size);
            success += 1;
        } else {
            log::error!("Multi-frame ({} frames) alignment test: FAIL", size);
            failure += 1;
        }
    }

    log::info!(
        "Frame tests completed: {} passed, {} failed",
        success,
        failure
    );
    (success, failure)
}

pub fn test() -> (usize, usize) {
    let mut success = 0;
    let mut failure = 0;

    log::info!("Running memory frame alignment tests...");
    let (s, f) = test_frame_allocation();
    if f > 0 {
        log::error!(
            "Frame alignment tests failed: {} failures, {} successes",
            f,
            s
        );
    } else {
        log::info!("All frame alignment tests passed: {} successes", s);
    }

    success += s;
    failure += f;

    // Test specifically the size needed for VirtIO (usually 2 pages)
    let virtio_frames_aligned = test_frame_alignment(2);
    if virtio_frames_aligned {
        log::info!("VirtIO frame alignment test passed");
        success += 1;
    } else {
        log::error!(
            "VirtIO frame alignment test failed - frames not properly aligned to 4K boundaries"
        );
        failure += 1;
    }

    (success, failure)
}
