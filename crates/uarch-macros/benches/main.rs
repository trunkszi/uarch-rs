mod cache_line;
mod inline_policy;
mod isolated;
mod layout;
mod multiversion;
mod paths;
mod pattern;
mod register_pass;

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

struct CountingAllocator;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// SAFETY: Allocation and deallocation forward the original layouts to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: The caller supplies a valid allocation layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout came from System.alloc.
        unsafe {
            System.dealloc(pointer, layout);
        }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: The caller preserves the allocation and new-size contract.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

fn main() {
    for (name, operation) in [
        ("cache_line", cache_line::run as fn(u64) -> u64),
        ("inline_policy", inline_policy::run),
        ("isolated", isolated::run),
        ("layout", layout::run),
        ("multiversion", multiversion::run),
        ("paths", paths::run),
        ("pattern", pattern::run),
        ("register_pass", register_pass::run),
    ] {
        let mut samples = [0u128; 1024];
        let allocations = ALLOCATIONS.load(Ordering::Relaxed);
        for sample in &mut samples {
            let start = Instant::now();
            for value in 0..256 {
                black_box(operation(black_box(value)));
            }
            *sample = start.elapsed().as_nanos() / 256;
        }
        assert_eq!(
            ALLOCATIONS.load(Ordering::Relaxed),
            allocations,
            "macro-generated hot paths must not allocate"
        );
        samples.sort_unstable();
        println!(
            "{name}: p50={}ns p90={}ns p99={}ns p99.9={}ns",
            samples[512], samples[921], samples[1013], samples[1022]
        );
    }
}
