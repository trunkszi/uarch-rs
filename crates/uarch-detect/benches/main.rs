mod cpuid;
mod host;
mod hwcap;
mod policy;
mod quirks;

fn sample<T>(name: &str, mut f: impl FnMut() -> T) {
    let mut timings = Vec::with_capacity(256);
    for _ in 0..256 {
        let start = std::time::Instant::now();
        std::hint::black_box(f());
        timings.push(start.elapsed().as_nanos());
    }
    timings.sort_unstable();
    println!(
        "{name}: p50={}ns p90={}ns p99={}ns p999={}ns",
        timings[128], timings[230], timings[253], timings[255]
    );
}
fn main() {
    cpuid::run();
    policy::run();
    quirks::run();
    hwcap::run();
    host::run();
}
