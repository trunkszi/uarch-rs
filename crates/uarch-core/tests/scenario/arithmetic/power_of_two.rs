//! Quotient, remainder and alignment at power-of-two boundaries.

#[test]
fn alignment_matches_wide_oracle() {
    use uarch_core::arithmetic::PowerOfTwo;
    fn check<const N: u32>() {
        let divisor = 1u128 << N;
        for value in [0, 1, 7, 8, 9, u64::MAX - 7, u64::MAX] {
            type Output = u64;
            let wide = value as u128;
            assert_eq!(PowerOfTwo::<N>::div_u64(value), (wide / divisor) as Output);
            assert_eq!(PowerOfTwo::<N>::rem_u64(value), (wide % divisor) as Output);
            assert_eq!(
                PowerOfTwo::<N>::div_rem_u64(value),
                ((wide / divisor) as u64, (wide % divisor) as u64)
            );
            assert_eq!(
                PowerOfTwo::<N>::is_multiple_of_u64(value),
                wide.is_multiple_of(divisor)
            );
            let down = wide / divisor * divisor;
            let up = wide.div_ceil(divisor) * divisor;
            assert_eq!(PowerOfTwo::<N>::align_down_u64(value), down as u64);
            assert_eq!(
                PowerOfTwo::<N>::checked_align_up_u64(value),
                u64::try_from(up).ok()
            );
            assert_eq!(
                PowerOfTwo::<N>::alignment_offset_u64(value),
                (up - wide) as u64
            );
            if N < usize::BITS {
                let native = value as usize;
                let d = 1usize << N;
                assert_eq!(PowerOfTwo::<N>::div_usize(native), native / d);
                assert_eq!(PowerOfTwo::<N>::rem_usize(native), native % d);
                assert_eq!(
                    PowerOfTwo::<N>::div_rem_usize(native),
                    (native / d, native % d)
                );
                assert_eq!(
                    PowerOfTwo::<N>::is_multiple_of_usize(native),
                    native.is_multiple_of(d)
                );
                assert_eq!(PowerOfTwo::<N>::align_down_usize(native), native / d * d);
                assert_eq!(
                    PowerOfTwo::<N>::checked_align_up_usize(native),
                    native.checked_add(d - 1).map(|x| x / d * d)
                );
                assert_eq!(
                    PowerOfTwo::<N>::alignment_offset_usize(native),
                    native.wrapping_neg() & (d - 1)
                );
            }
        }
    }
    const ALIGNED: Option<u64> = PowerOfTwo::<3>::checked_align_up_u64(9);
    assert_eq!(ALIGNED, Some(16));
    check::<0>();
    check::<1>();
    check::<3>();
    check::<31>();
    check::<63>();
}

#[test]
fn quotient_pairs_are_const() {
    use uarch_core::{arithmetic::PowerOfTwo, layout::RegisterLeaf};
    const PAIR64: (u64, u64) = PowerOfTwo::<63>::div_rem_u64(u64::MAX);
    const NATIVE: (usize, usize) = PowerOfTwo::<0>::div_rem_usize(usize::MAX);
    assert_eq!(PAIR64, (1, i64::MAX as u64));
    assert_eq!(NATIVE, (usize::MAX, 0));
    assert_eq!(<PowerOfTwo<3> as RegisterLeaf>::LEAVES, 0);
}
