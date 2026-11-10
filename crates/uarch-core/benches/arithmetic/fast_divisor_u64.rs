use super::{report, time_samples};
use core::num::NonZeroU64;
use std::hint::black_box;
use uarch_core::arithmetic::FastDivisorU64;

pub(super) fn run() {
    let fast = FastDivisorU64::new(match NonZeroU64::new(10_007) {
        Some(nz) => nz,
        None => panic!("non-zero"),
    });
    let samples = time_samples(|| {
        let mut acc = 0u64;
        let mut n = 1u64;
        for _ in 0..32 {
            acc = acc.wrapping_add(fast.div(black_box(n)));
            n = n.wrapping_add(0x9E37);
        }
        black_box(acc);
    });
    report("arithmetic fast_divisor_u64", &samples);
}
