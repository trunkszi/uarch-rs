pub mod __private {
    /// # Safety
    /// Every storage byte is initialized, without implicit padding.
    pub unsafe trait NoPadding: Sized {}
    /// # Safety
    /// The type is a padding-free scalar ABI leaf.
    pub unsafe trait RegisterLeaf: Copy + NoPadding {
        const LEAVES: u8;
    }
    pub const fn assert_no_padding<T: NoPadding>() {}
    pub const fn assert_copy<T: Copy>() {}
    pub const fn assert_register_leaf<T: RegisterLeaf>() {}

    macro_rules! scalar {
        ($($ty:ty),*) => {$(
            // SAFETY: These primitive integer scalars contain no padding.
            unsafe impl NoPadding for $ty {}
            // SAFETY: Each integer scalar contributes one ABI leaf.
            unsafe impl RegisterLeaf for $ty { const LEAVES: u8 = 1; }
        )*};
    }
    scalar!(u8, u16, u32, u64);
    // SAFETY: Integer storage has no implicit padding bytes.
    unsafe impl NoPadding for u128 {}
    // SAFETY: Arrays concatenate padding-free elements without gaps.
    unsafe impl<T: NoPadding, const N: usize> NoPadding for [T; N] {}
}
