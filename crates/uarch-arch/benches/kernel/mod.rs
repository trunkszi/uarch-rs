mod bit_gather;
mod masked_scan;
use uarch_arch::Arch;
pub(super) fn run<A: Arch>(arch: A) {
    bit_gather::run(arch);
    masked_scan::run(arch);
}
