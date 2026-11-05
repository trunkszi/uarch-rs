use uarch_core::arithmetic::PowerOfTwo;

#[test]
fn power_of_two_wraps_low_bits() {
    const WRAPPED: usize = PowerOfTwo::<12>::wrap_usize(0x1_0ABC);
    assert_eq!(WRAPPED, 0xABC);
    assert_eq!(PowerOfTwo::<0>::wrap_usize(usize::MAX), 0);
    assert_eq!(PowerOfTwo::<0>::VALUE, 1);
    assert_eq!(PowerOfTwo::<0>::wrap_u64(u64::MAX), 0);
    assert_eq!(PowerOfTwo::<12>::MASK_U64, (1u64 << 12) - 1);
    assert_eq!(PowerOfTwo::<12>::wrap_u64(0x1_0ABC), 0xABC);
    assert_eq!(PowerOfTwo::<12>::wrap_usize(usize::MAX), (1usize << 12) - 1);
    assert_eq!(PowerOfTwo::<63>::VALUE_U64, 1u64 << 63);
}
