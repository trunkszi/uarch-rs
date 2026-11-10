use uarch_arch::Arch;
pub(super) fn run<A: Arch>(arch: A) {
    crate::sample("gather", || {
        arch.bit_gather(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    });
    crate::sample("scatter", || {
        arch.bit_scatter(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    });
}
