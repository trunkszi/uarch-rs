use uarch_core::mem::L3;
use uarch_core::mem::{Pool, PoolIndex};

fn index(slot: u32) -> PoolIndex<u8> {
    match PoolIndex::new(slot) {
        Some(index) => index,
        None => panic!("slot {slot}"),
    }
}

#[test]
fn oversize_index_no_alias() {
    let mut storage = [0u8, 1, 2, 3, 4, 5, 6, 7];
    {
        let pool = match Pool::<u8, 3>::new(&mut storage) {
            Ok(pool) => pool,
            Err(_) => panic!("capacity"),
        };
        assert!(pool.get(index(8)).is_none());
        assert!(pool.get(index(9)).is_none());
        assert!(pool.get(index(16)).is_none());
        assert_eq!(pool.get(index(7)).copied(), Some(7));
        assert_eq!(*pool.get_masked(index(8)), 0);
        assert_eq!(*pool.get_masked(index(9)), 1);
        assert_eq!(*pool.get_masked(index(16)), 0);
        pool.prefetch_read::<L3>(index(16));
    }
    assert_eq!(storage[0], 0);
}
