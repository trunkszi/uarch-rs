use uarch_core::{
    arithmetic::PowerOfTwo,
    bits::{BitField, BitFlag, BitPatch, BitPattern, FlagClear, FlagSet},
    layout::{NoPadding, RegisterLeaf},
};

#[test]
fn descriptor_storage_has_proofs() {
    fn initialized<T: NoPadding>() {}
    fn leaf<T: RegisterLeaf>(expected: u8) {
        assert_eq!(T::LEAVES, expected);
    }
    initialized::<BitPattern<u64>>();
    initialized::<BitPatch<u64>>();
    assert_eq!(core::mem::size_of::<BitPattern<u64>>(), 16);
    assert_eq!(core::mem::size_of::<BitPatch<u64>>(), 16);
    leaf::<BitFlag<u64>>(1);
    leaf::<FlagSet<u64>>(1);
    leaf::<FlagClear<u64>>(1);
    leaf::<BitField<u64, u8, 8, 15>>(0);
    leaf::<PowerOfTwo<63>>(0);
}
