use super::{report, time_samples};
use std::hint::black_box;
use uarch_core::flow::reduce_lanes;

pub(super) fn run() {
    let data = [1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let samples = time_samples(|| {
        let got = reduce_lanes::<4, _, _>(
            black_box(&data),
            0u64,
            0u64,
            |acc, item| acc.wrapping_add(*item),
            |a, b| a.wrapping_add(b),
        );
        black_box(got);
    });
    report("flow reduce_lanes", &samples);
}
