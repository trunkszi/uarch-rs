use super::{report, time_samples};
use std::hint::black_box;
use uarch_core::flow::predicate;

pub(super) fn run() {
    let samples = time_samples(|| {
        let mut acc = 0u64;
        for i in 0..64u64 {
            acc = acc.wrapping_add(
                predicate(black_box(i) & 1 == 0).select_register(i, i.wrapping_neg()),
            );
        }
        black_box(acc);
    });
    report("flow select_register", &samples);
}
