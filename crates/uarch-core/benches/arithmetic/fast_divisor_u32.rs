use super::{report, time_samples};
use core::num::NonZeroU32;
use std::hint::black_box;
use uarch_core::arithmetic::FastDivisorU32;

pub(super) fn run() {
    let modulo = FastDivisorU32::new(match NonZeroU32::new(97) {
        Some(nz) => nz,
        None => panic!("non-zero"),
    });
    let samples = time_samples(|| {
        let mut acc = 0u32;
        let mut n = 1u32;
        for _ in 0..32 {
            acc = acc.wrapping_add(modulo.rem(black_box(n)));
            n = n.wrapping_add(17);
        }
        black_box(acc);
    });
    report("arithmetic fast_divisor_u32", &samples);
}
