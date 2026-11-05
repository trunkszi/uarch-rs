pub(super) fn must_pool(storage: &mut [u64; 4]) -> uarch::mem::Pool<'_, u64, 2> {
    match uarch::mem::Pool::<u64, 2>::new(storage) {
        Ok(pool) => pool,
        Err(_err) => panic!("length must be 1 << LOG2"),
    }
}

pub(super) fn must_idx(slot: u32) -> uarch::mem::PoolIndex<u64> {
    match uarch::mem::PoolIndex::new(slot) {
        Some(index) => index,
        None => panic!("slot 0 is the sentinel"),
    }
}
