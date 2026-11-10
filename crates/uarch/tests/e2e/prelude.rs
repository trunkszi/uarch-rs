use crate::task_meta::TaskMeta;
use uarch::prelude::*;

#[test]
fn patch_selects_ready_seq() {
    let armed = TaskMeta::default().patch(patch!(TaskMeta {
        READY: true,
        STATE: 2,
        SEQ: 5
    }));
    let hot = pattern!(TaskMeta { READY & !URGENT & STATE == 2 });
    let taken = predicate(armed.matches(hot)).select(1u64, 0);
    assert_eq!(taken, 1);
    let next = armed.wrapping_add_top_field(TaskMeta::SEQ, 1u64);
    assert!(next.matches(pattern!(TaskMeta { SEQ == 6 & READY })));
    let sum = reduce_lanes::<4, u64, u64>(
        &[1u64, 2, 3, 4],
        0u64,
        0u64,
        |acc, value| acc.wrapping_add(*value),
        |lhs, rhs| lhs.wrapping_add(rhs),
    );
    assert_eq!(sum, 10);
    let divisor = match core::num::NonZeroU64::new(7) {
        Some(value) => value,
        None => panic!("nonzero constant divisor"),
    };
    let quotient = FastDivisorU64::new(divisor).div(sum);
    let mut storage = [0u64; 4];
    let mut pool = match Pool::<u64, 2>::new(&mut storage) {
        Ok(pool) => pool,
        Err(error) => panic!("valid storage: {error}"),
    };
    let index = match PoolIndex::new(1) {
        Some(index) => index,
        None => panic!("nonzero slot"),
    };
    match pool.get_mut(index) {
        Some(slot) => *slot = quotient,
        None => panic!("slot is in range"),
    }
    let scalar = Scalar::new();
    assert_eq!(scalar.bit_gather(*pool.get_masked(index), 1), 1);
}

#[test]
fn ilp_all_evaluates_arms() {
    let mut hits = 0u32;
    let all = ilp_all!(
        {
            hits += 1;
            false
        },
        {
            hits += 1;
            true
        }
    );
    assert!(!all);
    assert_eq!(hits, 2);
}

#[test]
fn prelude_exports_prefetch_localities() {
    let value = 42u64;
    prefetch_read::<L1, _>(&value);
    prefetch_read::<L2, _>(&value);
    prefetch_read::<L3, _>(&value);
    prefetch_read::<NonTemporal, _>(&value);
    assert_eq!(value, 42);
}
