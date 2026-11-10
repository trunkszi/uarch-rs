use uarch_core::layout::{CacheAligned, Isolated, assert_layout};

#[repr(C)]
struct Wire {
    low: u32,
    high: u32,
}
assert_layout!(Wire,size=8,align=4,fields={low:0,high:4});
assert_layout!(CacheAligned<u64>, size = 64, align = 64);
assert_layout!(Isolated<u64>, size = 128, align = 128);
assert_layout!(CacheAligned<[u8; 192]>, size = 192, align = 64);

#[test]
fn aligned_storage_preserves_value() {
    fn traits<T: Copy + Clone + core::fmt::Debug + Default + Eq + Ord + core::hash::Hash>() {}
    traits::<CacheAligned<u64>>();
    traits::<Isolated<u64>>();
    let mut cache = CacheAligned::from(3u64);
    *cache.as_mut() = 4;
    assert_eq!(*cache.as_ref(), 4);
    *cache = 5;
    assert_eq!(cache.into_inner(), 5);
    let mut isolated = Isolated::from(6u64);
    *isolated.as_mut() = 7;
    assert_eq!(*isolated.as_ref(), 7);
    *isolated = 8;
    assert_eq!(isolated.into_inner(), 8);
    const CACHE: CacheAligned<u64> = CacheAligned::new(9);
    const VALUE: u64 = *CACHE.get();
    assert_eq!(VALUE, 9);
    let wire = Wire { low: 1, high: 2 };
    assert_eq!(wire.low + wire.high, 3);
}
