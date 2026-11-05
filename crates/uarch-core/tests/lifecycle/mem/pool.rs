use uarch_core::mem::{Pool, PoolIndex};

fn index(slot: u32) -> PoolIndex<u16> {
    match PoolIndex::new(slot) {
        Some(index) => index,
        None => panic!("slot {slot}"),
    }
}

#[test]
fn drop_returns_storage() {
    let mut storage = [0u16, 1, 2, 3];
    {
        let mut pool = match Pool::<u16, 2>::new(&mut storage) {
            Ok(pool) => pool,
            Err(_) => panic!("capacity"),
        };
        match pool.get_mut(index(2)) {
            Some(slot) => *slot = 20,
            None => panic!("slot 2"),
        }
        *pool.get_masked_mut(index(6)) = 21;
    }
    assert_eq!(storage, [0, 1, 21, 3]);
}
