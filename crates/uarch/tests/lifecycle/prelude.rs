#[path = "../support/pool.rs"]
mod pool;

use uarch::layout::Isolated;
use uarch::mem::PoolIndex;

#[test]
fn pool_borrow_ends_on_drop() {
    let mut storage = [1u64, 2, 3, 4];
    {
        let pool = pool::must_pool(&mut storage);
        let index = pool::must_idx(2);
        let slot = match pool.get(index) {
            Some(value) => *value,
            None => panic!("slot 2 is inside a 4-wide pool"),
        };
        assert_eq!(slot, 3);
    }
    storage[2] = 9;
    assert_eq!(storage[2], 9);
}

#[test]
fn isolated_keeps_aligned_cell() {
    let mut cell = Isolated::new(5u64);
    assert_eq!(*cell.get(), 5);
    *cell.get_mut() = 6;
    assert_eq!(*cell.get(), 6);
    assert_eq!(core::mem::align_of::<Isolated<u64>>(), 128);
    assert!(core::mem::size_of::<Isolated<u64>>() >= 128);
    let inner = cell.into_inner();
    assert_eq!(inner, 6);
    assert!(PoolIndex::<u64>::new(0).is_none());
}
