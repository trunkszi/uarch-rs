use core::{marker::PhantomData, num::NonZeroU64};
use uarch_core::{
    bits::Bitmap64,
    layout::{NoPadding, RegisterLeaf, assert_copy, assert_no_padding, assert_register_leaf},
};

#[test]
fn proofs_accept_scalar_families() {
    const {
        assert_copy::<Bitmap64>();
        assert_register_leaf::<Bitmap64>();
    }
    fn leaf<T: RegisterLeaf>(_: T) -> u8 {
        T::LEAVES
    }
    assert_eq!(leaf(NonZeroU64::MIN), 1);
    assert_eq!(leaf(PhantomData::<[u8]>), 0);
    fn tight<T: NoPadding>() {
        assert_no_padding::<T>();
    }
    tight::<[u128; 3]>();
    tight::<bool>();
    tight::<char>();
    tight::<core::sync::atomic::AtomicU64>();
}
