use uarch_core::mem::{Pool, PoolError};

#[test]
fn new_rejects_bad_len() {
    let mut storage = [1u8, 2, 3];
    let err = match Pool::<u8, 2>::new(&mut storage) {
        Ok(_) => panic!("len 3 is not 4"),
        Err(err) => err,
    };
    assert_eq!(err, PoolError::Capacity { len: 3, cap: 4 });
}
