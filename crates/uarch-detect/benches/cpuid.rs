use uarch_detect::{CpuidSource, HardwareCpuid};
pub(super) fn run() {
    crate::sample("cpuid", || HardwareCpuid.cpuid(std::hint::black_box(1), 0));
}
