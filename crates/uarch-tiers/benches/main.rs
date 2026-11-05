use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use uarch_tiers::TierId;

struct CountingAllocator;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: Every request is forwarded unchanged to System, with matching layout and ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: The caller supplies the GlobalAlloc layout contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The allocation and original layout belong to this System allocator.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn main() {
    let mut samples = [0_u128; 1024];
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for (index, sample) in samples.iter_mut().enumerate() {
        let tier = black_box(TierId::ALL)[index % 5];
        let start = Instant::now();
        for _ in 0..256 {
            black_box(black_box(tier).features());
        }
        *sample = start.elapsed().as_nanos();
    }
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;
    assert_eq!(allocations, 0);
    let elapsed: u128 = samples.iter().sum();
    samples.sort_unstable();
    println!(
        "tier_features: P50={}ns P90={}ns P99={}ns P99.9={}ns throughput={:.0}ops/s allocations={allocations}",
        samples[512] as f64 / 256.0,
        samples[921] as f64 / 256.0,
        samples[1013] as f64 / 256.0,
        samples[1022] as f64 / 256.0,
        1024.0 * 256.0 * 1e9 / elapsed as f64
    );
}
