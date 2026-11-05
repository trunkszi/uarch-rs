use uarch_arch::{Arch, Scalar};
pub(crate) fn run(arch: Scalar) {
    crate::sample(<Scalar as Arch>::NAME, || {
        arch.bit_gather(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    });
}
