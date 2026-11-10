use uarch::{Arch, Scalar};
use uarch_macros::multiversion;

#[multiversion]
fn kernel<A: Arch>(arch: A, value: u64) -> u64 {
    arch.bit_gather(value, u64::MAX)
}

pub(super) fn run(value: u64) -> u64 {
    // SAFETY: Scalar needs no optional ISA features.
    kernel(unsafe { Scalar::new_unchecked() }, value)
}
