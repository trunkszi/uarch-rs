use uarch_core::mem::{L1, L2, NonTemporal};
use uarch_core::mem::{Pool, PoolIndex};

fn index(slot: u32) -> PoolIndex<u32> {
    match PoolIndex::new(slot) {
        Some(index) => index,
        None => panic!("slot {slot}"),
    }
}

#[test]
fn masked_read_after_fill() {
    let mut storage = [10u32, 11, 12, 13];
    {
        let pool = match Pool::<u32, 2>::new(&mut storage) {
            Ok(pool) => pool,
            Err(_) => panic!("capacity"),
        };
        assert_eq!(pool.get(index(1)).copied(), Some(11));
        assert_eq!(*pool.get_masked(index(1)), 11);
        pool.prefetch_read::<L1>(index(1));
        pool.prefetch_read::<L2>(index(2));
        pool.prefetch_read::<NonTemporal>(index(3));
        assert_eq!(*pool.get_masked(index(5)), 11);
    }
    assert_eq!(storage[1], 11);
}
