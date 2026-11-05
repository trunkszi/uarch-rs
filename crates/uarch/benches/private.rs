use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

struct Counted;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
#[global_allocator]
static ALLOCATOR: Counted = Counted;

// SAFETY: Allocation ownership and layouts are forwarded unchanged to System.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: The caller supplies a valid allocation layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout originate from the delegated allocation.
        unsafe { System.dealloc(pointer, layout) }
    }
}

pub(super) fn measure(
    name: &str,
    variant: &str,
    run: fn(&[u64; 256]) -> u64,
    data: &[u64; 256],
    work_items: usize,
) -> [u64; 1024] {
    black_box(run(black_box(data)));
    let mut samples = [0u64; 1024];
    let mut results = [0u64; 1024];
    let mut stream = *data;
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for (sample, result) in samples.iter_mut().zip(&mut results) {
        // Identical evolving inputs prevent repeated samples from training the branch predictor.
        for raw in &mut stream {
            *raw ^= *raw << 13;
            *raw ^= *raw >> 7;
            *raw ^= *raw << 17;
        }
        let start = Instant::now();
        let value = black_box(run(black_box(&stream)));
        *sample = start.elapsed().as_nanos() as u64;
        *result = value;
    }
    assert_eq!(
        ALLOCATIONS.load(Ordering::Relaxed),
        before,
        "hot path allocated"
    );
    let total_ns: u64 = samples.iter().sum();
    samples.sort_unstable();
    println!(
        "{name}/{variant}: work_items={work_items} batch_ns p50={} p90={} p99={} p99.9={} ops/s={:.0} allocations=0",
        samples[512],
        samples[921],
        samples[1013],
        samples[1022],
        (work_items as f64 * 1024.0 * 1e9) / total_ns.max(1) as f64
    );
    results
}
