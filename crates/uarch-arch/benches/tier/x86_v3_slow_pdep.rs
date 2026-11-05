use uarch_arch::{Arch, X86V3SlowPdep};
pub(crate) fn run(arch: X86V3SlowPdep) {
    crate::sample(<X86V3SlowPdep as Arch>::NAME, || {
        arch.bit_gather(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    });
}
