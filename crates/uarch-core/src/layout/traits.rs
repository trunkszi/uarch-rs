use core::{marker::PhantomData, num::*};

/// Proves the absence of implicit storage padding.
///
/// # Safety
/// Every storage byte in a valid instance must be initialized. This proof does
/// not permit byte views, arbitrary bit patterns, copying, or non-atomic reads.
///
/// Isolation can introduce tail padding:
/// ```compile_fail
/// uarch_core::layout::assert_no_padding::<uarch_core::layout::Isolated<u64>>();
/// ```
pub unsafe trait NoPadding: Sized {}

/// Proves the number of scalar leaves contributed as a direct aggregate field.
///
/// # Safety
/// The type must be a native scalar or a transparent scalar wrapper with one
/// ABI leaf, or a zero-sized marker with zero leaves. C aggregates and arrays
/// are excluded even when their size equals a scalar's size.
///
/// ```compile_fail
/// uarch_core::layout::assert_register_leaf::<u128>();
/// ```
/// ```compile_fail
/// uarch_core::layout::assert_register_leaf::<[u8; 8]>();
/// ```
/// ```compile_fail
/// #[repr(C)]
/// #[derive(Clone, Copy)]
/// struct Aggregate { value: u64 }
/// // SAFETY: The sole field occupies all storage bytes.
/// unsafe impl uarch_core::layout::NoPadding for Aggregate {}
/// uarch_core::layout::assert_register_leaf::<Aggregate>();
/// ```
pub unsafe trait RegisterLeaf: Copy + NoPadding {
    const LEAVES: u8;
}

macro_rules! scalar {
    ($($ty:ty),* $(,)?) => {$(
        // SAFETY: Native scalars have no implicit storage padding.
        unsafe impl NoPadding for $ty {}
        // SAFETY: Each listed scalar is at most 64 bits and contributes one leaf.
        unsafe impl RegisterLeaf for $ty { const LEAVES: u8 = 1; }
    )*};
}
scalar!(
    u8,
    u16,
    u32,
    u64,
    usize,
    i8,
    i16,
    i32,
    i64,
    isize,
    f32,
    f64,
    bool,
    char,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroUsize,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroIsize
);

// SAFETY: A 128-bit integer has no implicit padding, though it is not an ABI leaf.
unsafe impl NoPadding for u128 {}
// SAFETY: A 128-bit integer has no implicit padding, though it is not an ABI leaf.
unsafe impl NoPadding for i128 {}
// SAFETY: Nonzero integers retain their primitive integer storage layout.
unsafe impl NoPadding for NonZeroU128 {}
// SAFETY: Nonzero integers retain their primitive integer storage layout.
unsafe impl NoPadding for NonZeroI128 {}
// SAFETY: Arrays contain adjacent elements without inter-element or tail padding.
unsafe impl<T: NoPadding, const N: usize> NoPadding for [T; N] {}
// SAFETY: PhantomData has no storage bytes.
unsafe impl<T: ?Sized> NoPadding for PhantomData<T> {}
// SAFETY: PhantomData has no storage and contributes no ABI leaves.
unsafe impl<T: ?Sized> RegisterLeaf for PhantomData<T> {
    const LEAVES: u8 = 0;
}

macro_rules! atomic {
    ($width:literal, $($ty:ident),* $(,)?) => {$(
        #[cfg(target_has_atomic = $width)]
        // SAFETY: Atomic storage has the same size as the corresponding scalar.
        unsafe impl NoPadding for core::sync::atomic::$ty {}
    )*};
}
atomic!("8", AtomicU8, AtomicI8, AtomicBool);
atomic!("16", AtomicU16, AtomicI16);
atomic!("32", AtomicU32, AtomicI32);
atomic!("64", AtomicU64, AtomicI64);
atomic!("ptr", AtomicUsize, AtomicIsize);
