#[test]
fn checked_and_masked_preserve_bounds() -> Result<(), uarch_core::mem::PoolError> {
    for slot in [1, 255, 256, 257, u32::MAX] {
        let mut slots = core::array::from_fn::<_, 256, _>(|i| i as u64 * 17);
        let Some(index) = uarch_core::mem::PoolIndex::new(slot) else {
            panic!("nonzero index")
        };
        let mut pool = uarch_core::mem::Pool::<_, 8>::new(&mut slots)?;
        assert_eq!(
            uarch_probes::mem::pool_get_checked(&pool, index),
            if slot < 256 {
                Some(u64::from(slot) * 17)
            } else {
                None
            }
        );
        assert_eq!(
            uarch_probes::mem::pool_get_masked(&pool, index),
            u64::from(slot & 255) * 17
        );
        if let Some(value) = pool.get_mut(index) {
            *value += 1;
        }
        assert_eq!(
            pool.get(index).copied(),
            if slot < 256 {
                Some(u64::from(slot) * 17 + 1)
            } else {
                None
            }
        );
    }
    Ok(())
}
