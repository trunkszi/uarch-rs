//! Layout and const evaluation at the invariant-divisor boundary.

#[test]
fn representation_has_no_padding() {
    use uarch_core::{arithmetic::FastDivisorU32, layout::assert_no_padding};
    const { assert_no_padding::<FastDivisorU32>() };
    assert_eq!(core::mem::size_of::<FastDivisorU32>(), 16);
    assert_eq!(core::mem::align_of::<FastDivisorU32>(), 8);
}

#[test]
fn widened_divisor_round_trips_in_const() {
    use core::num::NonZeroU32;
    use uarch_core::arithmetic::FastDivisorU32;
    const MAXIMUM: NonZeroU32 = match NonZeroU32::new(u32::MAX) {
        Some(divisor) => divisor,
        None => panic!("nonzero maximum divisor"),
    };
    const DIVISOR: NonZeroU32 = FastDivisorU32::new(MAXIMUM).divisor();
    const PAIR: (u32, u32) = FastDivisorU32::new(MAXIMUM).div_rem(u32::MAX);
    assert_eq!(DIVISOR, MAXIMUM);
    assert_eq!(PAIR, (1, 0));
    let inspect: fn(FastDivisorU32) -> NonZeroU32 = FastDivisorU32::divisor;
    assert_eq!(inspect(FastDivisorU32::new(MAXIMUM)), MAXIMUM);
}
