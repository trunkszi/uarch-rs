// Concrete probe contracts are validated through ABI, ASM, and MCA gates.

#[test]
fn remainder_probes_match_hardware() {
    use core::num::NonZeroU64;
    for d in [1, 2, 3, 7, 97, 1 << 32, 1 << 63, u64::MAX] {
        let divisor =
            uarch_probes::gate::FastDivisorU64::new(NonZeroU64::new(d).unwrap_or(NonZeroU64::MIN));
        for n in [0, 1, d - 1, d, d.wrapping_add(1), u64::MAX] {
            assert_eq!(uarch_probes::fast_divisor_u64_rem(divisor, n), n % d);
            assert_eq!(
                uarch_probes::fast_divisor_u64_pow2_rem(divisor, n),
                if d.is_power_of_two() { n % d } else { 0 }
            );
        }
    }
}

#[test]
fn division_and_alignment_match_reference() {
    use core::num::NonZeroU32;
    for d in [1, 2, 3, 7, 97, 1 << 31, u32::MAX] {
        let divisor =
            uarch_probes::gate::FastDivisorU32::new(NonZeroU32::new(d).unwrap_or(NonZeroU32::MIN));
        for n in [0, 1, d - 1, d, d.wrapping_add(1), u32::MAX] {
            assert_eq!(
                uarch_probes::arithmetic::fast_divisor_u32_div(divisor, n),
                n / d
            );
        }
    }
    for n in [0, 1, 63, 64, 65, u64::MAX] {
        assert_eq!(
            uarch_probes::arithmetic::power_of_two_align(n),
            (n & !63) ^ (n.wrapping_neg() & 63)
        );
    }
}
