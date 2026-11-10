use core::num::NonZeroU64;
use uarch_core::arithmetic::FastDivisorU64;

#[test]
fn reused_divisor_stable() {
    let fast = FastDivisorU64::new(match NonZeroU64::new(10) {
        Some(nz) => nz,
        None => panic!("non-zero"),
    });
    let again = fast;
    assert_eq!(fast.div(25), 2);
    assert_eq!(again.div(25), 2);
    assert_eq!(fast.rem(25), 5);
    assert_eq!(again.div_rem(25), (2, 5));
}
