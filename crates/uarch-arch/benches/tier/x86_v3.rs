use uarch_arch::{Arch, X86V3};
pub(crate) fn run(arch: X86V3) {
    crate::sample(<X86V3 as Arch>::NAME, || {
        arch.bit_gather(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    });
}
