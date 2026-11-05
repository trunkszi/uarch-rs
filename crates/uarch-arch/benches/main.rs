mod dispatch;
mod kernel;
mod tier;

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};
use uarch_arch::{Arch, ArchVisitor, Tier};

struct CountAlloc;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
// SAFETY: All allocation methods forward their unchanged layout/pointer contracts to System.
unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: GlobalAlloc forwards the caller's valid allocation layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Pointer and layout come unchanged from the GlobalAlloc caller.
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountAlloc = CountAlloc;

fn sample<T>(name: &str, mut f: impl FnMut() -> T) {
    let mut durations = Vec::with_capacity(2048);
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for _ in 0..2048 {
        let start = Instant::now();
        for _ in 0..256 {
            std::hint::black_box(f());
        }
        durations.push(start.elapsed().as_nanos());
    }
    assert_eq!(before, ALLOCATIONS.load(Ordering::Relaxed));
    durations.sort_unstable();
    println!(
        "{name}: batch=256 p50={}ns p90={}ns p99={}ns p999={}ns allocations=0",
        durations[1024], durations[1843], durations[2027], durations[2045]
    );
}

struct Measure;
impl ArchVisitor for Measure {
    type Output = ();
    fn visit<A: Arch>(self, arch: A) {
        kernel::run(arch);
    }
}

fn main() -> Result<(), uarch_detect::DetectError> {
    let capability = uarch_detect::detect(uarch_detect::DetectPolicy::default())?;
    dispatch::run(capability);
    capability.dispatch(Measure);
    match capability {
        Tier::Scalar(a) => tier::scalar::run(a),
        Tier::X86V3(a) => tier::x86_v3::run(a),
        Tier::X86V3SlowPdep(a) => tier::x86_v3_slow_pdep::run(a),
        Tier::X86V4(a) => tier::x86_v4::run(a),
        Tier::Aarch64Neon(a) => tier::aarch64_neon::run(a),
    }
    Ok(())
}
