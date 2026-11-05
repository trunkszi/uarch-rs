use core::num::NonZeroU64;
use std::hint::black_box;
use uarch::arithmetic::FastDivisorU64;

pub(super) fn bad(data: &[u64; 256]) -> u64 {
    let divisor = black_box(7u64);
    data.iter()
        .fold(0u64, |sum, raw| sum.wrapping_add(black_box(*raw) / divisor))
}
pub(super) fn good(data: &[u64; 256]) -> u64 {
    const DIVISOR: NonZeroU64 = match NonZeroU64::new(7) {
        Some(value) => value,
        None => panic!("nonzero constant divisor"),
    };
    let divisor = FastDivisorU64::new(DIVISOR);
    data.iter().fold(0u64, |sum, raw| {
        sum.wrapping_add(divisor.div(black_box(*raw)))
    })
}
