use uarch_arch::Arch;
pub(super) fn run<A: Arch>(arch: A) {
    let keys = [-9, 7, 11, 0];
    let vals = [1, 2, 3, 4];
    crate::sample("masked_sum", || {
        arch.masked_sum_lt_i64x4(
            std::hint::black_box(&keys),
            std::hint::black_box(&vals),
            std::hint::black_box(10),
        )
    });
    crate::sample("first_ge", || {
        arch.find_first_ge_i64x4(std::hint::black_box(&keys), std::hint::black_box(10))
    });
}
