use std::time::Instant;
mod fast_divisor_u32;
mod fast_divisor_u64;
mod power_of_two;

pub(crate) fn run() {
    fast_divisor_u64::run();
    fast_divisor_u32::run();
}

fn time_samples(mut body: impl FnMut()) -> Vec<u64> {
    for _ in 0..32 {
        body();
    }
    let mut samples = Vec::with_capacity(128);
    for _ in 0..128 {
        let start = Instant::now();
        for _ in 0..256 {
            body();
        }
        samples.push(start.elapsed().as_nanos() as u64);
    }
    samples
}

fn report(name: &str, samples: &[u64]) {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let at = |q: f64| sorted[((sorted.len() as f64 - 1.0) * q) as usize];
    println!(
        "{name} ns/batch p50={} p90={} p99={} p99.9={}",
        at(0.50),
        at(0.90),
        at(0.99),
        at(0.999)
    );
}
