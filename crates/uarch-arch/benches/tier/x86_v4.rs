use uarch_arch::{Arch, X86V4};
pub(crate) fn run(arch: X86V4) {
    crate::sample(<X86V4 as Arch>::NAME, || {
        arch.bit_gather(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    });
}
