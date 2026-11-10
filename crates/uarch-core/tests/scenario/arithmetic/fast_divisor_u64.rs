use core::num::NonZeroU32;
use core::num::NonZeroU64;
use uarch_core::arithmetic::FastDivisorU32;
use uarch_core::arithmetic::FastDivisorU64;

fn nz(value: u64) -> NonZeroU64 {
    match NonZeroU64::new(value) {
        Some(nz) => nz,
        None => panic!("non-zero"),
    }
}

fn check_div(d: u64, n: u64) {
    let fast = FastDivisorU64::new(nz(d));
    let q = fast.div(n);
    let r = fast.rem(n);
    assert_eq!(q, n / d, "div d={d} n={n}");
    assert_eq!(r, n % d, "rem d={d} n={n}");
    assert_eq!(fast.div_rem(n), (n / d, n % d));
    assert!(r < d);
    let rebuilt = (q as u128) * (d as u128) + (r as u128);
    assert_eq!(rebuilt, n as u128);
}

#[test]
fn edges_match_hardware() {
    let divisors = [
        1u64,
        2,
        3,
        4,
        5,
        6,
        7,
        9,
        10,
        10007,
        (1 << 16) - 1,
        1 << 16,
        (1 << 16) + 1,
        (1u64 << 32) - 1,
        1u64 << 32,
        (1u64 << 32) + 1,
        (1u64 << 63) - 1,
        1u64 << 63,
        (1u64 << 63) + 1,
        u64::MAX - 1,
        u64::MAX,
    ];
    for d in divisors {
        for n in [
            0,
            1,
            d - 1,
            d,
            d.saturating_add(1),
            d.saturating_mul(2),
            u64::MAX / 2,
            (1u64 << 32) - 1,
            1u64 << 32,
            1u64 << 63,
            u64::MAX,
        ] {
            check_div(d, n);
        }
    }
}

#[test]
fn random_matches_hardware() {
    let mut state = 0x1234_5678_9ABC_DEF0_u64;
    for _ in 0..4096 {
        state = state
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0x6A09_E667);
        let d = state | 1;
        state = state.wrapping_mul(0xBF58_476D_1CE4_E5B9).wrapping_add(1);
        let n = state;
        check_div(d, n);
    }
    let mut mod_state = 0xA5A5_A5A5_u32;
    for _ in 0..2048 {
        mod_state = mod_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let d = (mod_state % 50_000).saturating_add(1);
        mod_state = mod_state.wrapping_mul(22695477).wrapping_add(1);
        let n = mod_state;
        let fast = FastDivisorU32::new(match NonZeroU32::new(d) {
            Some(nz) => nz,
            None => panic!("non-zero"),
        });
        assert_eq!(fast.div(n), n / d);
        assert_eq!(fast.rem(n), n % d);
        assert_eq!(fast.is_multiple_of(n), n.is_multiple_of(d));
    }
}

#[test]
fn divisor_methods_match_scalar() {
    use core::num::{NonZeroU32, NonZeroU64};
    use uarch_core::arithmetic::{FastDivisorU32, FastDivisorU64};
    const D64: FastDivisorU64 = FastDivisorU64::new(NonZeroU64::MIN);
    const D32: FastDivisorU32 = FastDivisorU32::new(NonZeroU32::MIN);
    const PAIR64: (u64, u64) = D64.div_rem(u64::MAX);
    const PAIR32: (u32, u32) = D32.div_rem(u32::MAX);
    assert_eq!(PAIR64, (u64::MAX, 0));
    assert_eq!(PAIR32, (u32::MAX, 0));
    let mut seed = 0xf00d_1234_9876_4567u64;
    for divisor in [1, 2, 3, 7, 16, 97, u32::MAX as u64, 1 << 63, u64::MAX] {
        let fast = FastDivisorU64::new(
            NonZeroU64::new(divisor).unwrap_or_else(|| panic!("nonzero test input")),
        );
        assert_eq!(fast.divisor().get(), divisor);
        assert_eq!(fast.is_power_of_two(), divisor.is_power_of_two());
        assert_eq!(fast, FastDivisorU64::new(fast.divisor()));
        for numerator in [0, 1, divisor - 1, divisor, u64::MAX] {
            assert_eq!(
                fast.div_rem(numerator),
                (numerator / divisor, numerator % divisor)
            );
            assert_eq!(fast.is_multiple_of(numerator), numerator % divisor == 0);
        }
        for _ in 0..1024 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            assert_eq!(fast.div_rem(seed), (seed / divisor, seed % divisor));
        }
    }
    for divisor in [1, 2, 3, 7, 16, 97, 1 << 31, u32::MAX] {
        let fast = FastDivisorU32::new(
            NonZeroU32::new(divisor).unwrap_or_else(|| panic!("nonzero test input")),
        );
        assert_eq!(fast.divisor().get(), divisor);
        assert_eq!(fast.is_power_of_two(), divisor.is_power_of_two());
        for numerator in [0, 1, divisor - 1, divisor, u32::MAX] {
            assert_eq!(fast.div(numerator), numerator / divisor);
            assert_eq!(fast.rem(numerator), numerator % divisor);
            assert_eq!(
                fast.div_rem(numerator),
                (numerator / divisor, numerator % divisor)
            );
            assert_eq!(fast.is_multiple_of(numerator), numerator % divisor == 0);
        }
    }
}

#[test]
fn representation_has_no_padding() {
    use uarch_core::{arithmetic::FastDivisorU64, layout::assert_no_padding};
    const { assert_no_padding::<FastDivisorU64>() };
    assert_eq!(core::mem::size_of::<FastDivisorU64>(), 24);
    assert_eq!(core::mem::align_of::<FastDivisorU64>(), 8);
}

#[test]
fn every_power_of_two_matches_hardware() {
    let mut state = 0x435f_28bc_53a2_1d07u64;
    for shift in 0..64 {
        let d = 1u64 << shift;
        for _ in 0..128 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            check_div(d, state);
        }
    }
}

#[test]
fn power_tag_matches_divisor() {
    let check = |d| {
        let fast = FastDivisorU64::new(nz(d));
        assert_eq!(fast.is_power_of_two(), fast.divisor().is_power_of_two());
    };
    for shift in 0..64 {
        check(1u64 << shift);
    }
    for d in [
        3,
        7,
        (1u64 << 63) - 1,
        (1u64 << 63) + 1,
        u64::MAX - 1,
        u64::MAX,
    ] {
        check(d);
    }
    let mut state = 0x3ac5_91f0_a9df_e237u64;
    for _ in 0..4096 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        check(state | 1);
    }
}
