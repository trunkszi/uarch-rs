use core::mem::{align_of, size_of};
use uarch_core::mem::PoolIndex;

fn index<T>(slot: u32) -> PoolIndex<T> {
    match PoolIndex::new(slot) {
        Some(index) => index,
        None => panic!("slot {slot}"),
    }
}

#[test]
fn index_rejects_zero() {
    assert!(PoolIndex::<u32>::new(0).is_none());
    assert_eq!(index::<u32>(1).slot(), 1);
    assert_eq!(index::<u32>(u32::MAX).slot(), u32::MAX);
}

#[test]
fn option_index_is_four() {
    assert_eq!(size_of::<PoolIndex<u8>>(), 4);
    assert_eq!(align_of::<PoolIndex<u8>>(), 4);
    assert_eq!(size_of::<Option<PoolIndex<String>>>(), 4);
    assert_eq!(
        <PoolIndex<u8> as uarch_core::layout::RegisterLeaf>::LEAVES,
        1
    );
    fn assert_copy<T: Copy>() {}
    assert_copy::<PoolIndex<String>>();
}

#[test]
fn payload_needs_no_traits() {
    struct Payload;
    fn traits<T: Copy + Clone + core::fmt::Debug + Eq + Ord + core::hash::Hash>() {}
    traits::<uarch_core::mem::PoolIndex<Payload>>();
    let one = uarch_core::mem::PoolIndex::<Payload>::new(1)
        .unwrap_or_else(|| panic!("nonzero test input"));
    let two = uarch_core::mem::PoolIndex::<Payload>::new(2)
        .unwrap_or_else(|| panic!("nonzero test input"));
    assert!(one < two);
    assert_eq!(one, one);
    use std::hash::{Hash, Hasher};
    let mut a = std::collections::hash_map::DefaultHasher::new();
    let mut b = std::collections::hash_map::DefaultHasher::new();
    one.hash(&mut a);
    one.hash(&mut b);
    assert_eq!(a.finish(), b.finish());
}
