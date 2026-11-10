use std::hint::black_box;

use uarch_core::mem::{L1, Pool, PoolIndex};

use super::{report, time_samples};

pub(super) fn run() {
    let mut storage = [0u64; 1024];
    for (i, slot) in storage.iter_mut().enumerate() {
        *slot = i as u64;
    }
    let pool = match Pool::<u64, 10>::new(&mut storage) {
        Ok(pool) => pool,
        Err(_) => panic!("capacity"),
    };
    let index = match PoolIndex::<u64>::new(17) {
        Some(index) => index,
        None => panic!("slot"),
    };
    let samples = time_samples(|| {
        let value = *pool.get_masked(black_box(index));
        pool.prefetch_read::<L1>(index);
        black_box(value);
    });
    report("mem get_masked", &samples);
}
