use core::num::NonZeroU64;
use uarch_core::arithmetic::FastDivisorU64;

fn nz(value: u64) -> NonZeroU64 {
    match NonZeroU64::new(value) {
        Some(nz) => nz,
        None => panic!("non-zero"),
    }
}

#[test]
fn div_rem_rebuilds_n() {
    let cases = [
        (7u64, 100u64, 14u64, 2u64),
        (1, u64::MAX, u64::MAX, 0),
        (8, 15, 1, 7),
    ];
    for (d, n, q, r) in cases {
        let fast = FastDivisorU64::new(nz(d));
        assert_eq!(fast.div_rem(n), (q, r));
        let (got_q, got_r) = fast.div_rem(n);
        let rebuilt = (got_q as u128) * (d as u128) + (got_r as u128);
        assert_eq!(rebuilt, n as u128);
    }
}
