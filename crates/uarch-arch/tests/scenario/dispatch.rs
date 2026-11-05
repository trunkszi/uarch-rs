use uarch_arch::{Arch, ArchVisitor, Scalar, Tier, TierId, retag_unchecked};

struct Gather;
impl ArchVisitor for Gather {
    type Output = (TierId, u64);
    fn visit<A: Arch>(self, arch: A) -> Self::Output {
        (A::TIER, arch.bit_gather(0xa5, 0xf0))
    }
}

#[test]
fn visitor_preserves_capability() {
    const ARCH: Scalar = Scalar::new();
    let arch = ARCH;
    // SAFETY: The source and destination are the same Scalar tier.
    let retagged = unsafe { retag_unchecked::<Scalar, Scalar>(arch) };
    assert_eq!(
        Tier::Scalar(retagged).dispatch(Gather),
        (TierId::Scalar, 10)
    );
    assert_eq!(
        Tier::Scalar(Scalar::default()).dispatch(Gather),
        (TierId::Scalar, 10)
    );
}
