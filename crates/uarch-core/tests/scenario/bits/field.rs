use uarch_core::bits::{BitError, BitField};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct Word(u64);

#[test]
fn shifted_overflow_is_rejected() {
    let high = BitField::<Word, u8, 60, 63>::new();
    assert_eq!(high.try_eq(16), Err(BitError::ValueOverflow));
    assert_eq!(high.try_patch(16), Err(BitError::ValueOverflow));
    let full = BitField::<Word, u64, 0, 63>::new();
    assert_eq!(full.eq(u64::MAX).expected(), u64::MAX);
    assert_eq!(high.eq(15).expected(), 0xf000_0000_0000_0000);
}

#[test]
fn arithmetic_matches_wide_oracle() {
    fn check<const LO: u8, const HI: u8>() {
        let field = BitField::<Word, u64, LO, HI>::new();
        let modulus = 1u128 << (HI - LO + 1);
        let lane = (modulus - 1) << LO;
        let mut seed = 0x1234_5678_9abc_def0u64;
        for _ in 0..512 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let raw = seed;
            let delta = seed.rotate_left(17);
            let value = ((u128::from(raw) >> LO) + u128::from(delta)) % modulus;
            let expected = ((u128::from(raw) & !lane) | (value << LO)) as u64;
            assert_eq!(field.wrapping_add_field(raw, delta), expected);
            if HI == 63 {
                assert_eq!(
                    BitField::<Word, u64, LO, 63>::new().wrapping_add_top(raw, delta),
                    expected
                );
            }
        }
    }
    check::<0, 0>();
    check::<0, 63>();
    check::<8, 11>();
    check::<60, 63>();
}
