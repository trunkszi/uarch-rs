use uarch_arch::{Aarch64Neon, Arch};
pub(crate) fn run(arch: Aarch64Neon) {
    crate::sample(<Aarch64Neon as Arch>::NAME, || {
        arch.bit_gather(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    });
}
