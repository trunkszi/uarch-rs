use core::num::NonZeroU32;
use uarch_core::arithmetic::FastDivisorU32;

#[test]
fn mod_one_is_identity() {
    let fast = FastDivisorU32::new(NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN));
    for n in [0, 1, 7, 255, u32::MAX] {
        assert_eq!(fast.rem(n), 0);
        assert_eq!(fast.div(n), n);
        assert!(fast.is_multiple_of(n));
    }
    let seven = FastDivisorU32::new(NonZeroU32::MIN.saturating_add(6));
    assert_eq!(seven.rem(100), 2);
    assert_eq!(seven.div(100), 14);
    assert!(seven.is_multiple_of(98));
    assert!(!seven.is_multiple_of(99));
}
