#[path = "../support/pool.rs"]
mod pool;

use crate::task_meta::TaskMeta;
use uarch::bits::BitPattern;
use uarch::mem::{Pool, PoolIndex};
use uarch::prelude::*;

#[test]
fn conflict_pattern_is_err() {
    let yes = BitPattern::<TaskMeta>::ANY.and_flag(TaskMeta::READY);
    let no = BitPattern::<TaskMeta>::ANY.and_not(TaskMeta::READY);
    assert!(yes.try_and(no).is_err());
}

#[test]
fn wrapped_field_keeps_neighbor() {
    let armed = TaskMeta::default().patch(patch!(TaskMeta { STATE: 15, SEQ: 3 }));
    let next = armed.wrapping_add_field(TaskMeta::STATE, 1u64);
    assert!(next.matches(pattern!(TaskMeta { STATE == 0 & SEQ == 3 })));
}

#[test]
fn zero_slot_rejected() {
    assert!(PoolIndex::<u64>::new(0).is_none());
    assert!(PoolIndex::<u64>::new(1).is_some());
}

#[test]
fn short_storage_is_err() {
    let mut storage = [0u64; 2];
    assert!(Pool::<u64, 2>::new(&mut storage).is_err());
}

#[test]
fn masked_get_wraps_index() {
    let mut storage = [10u64, 20, 30, 40];
    let pool = pool::must_pool(&mut storage);
    let index = pool::must_idx(5);
    assert!(pool.get(index).is_none());
    assert_eq!(*pool.get_masked(index), 20);
}
