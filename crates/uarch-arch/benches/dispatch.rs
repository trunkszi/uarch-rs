use uarch_arch::{Arch, ArchVisitor, Tier};
struct Gather;
impl ArchVisitor for Gather {
    type Output = u64;
    fn visit<A: Arch>(self, arch: A) -> u64 {
        arch.bit_gather(
            std::hint::black_box(0xdead_beef),
            std::hint::black_box(0x5555_5555),
        )
    }
}
pub(super) fn run(tier: Tier) {
    crate::sample("dispatch", || std::hint::black_box(tier).dispatch(Gather));
}
