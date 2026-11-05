//! Physical slot numbers start at 1. Zero is the pool sentinel and is not a name.

use core::marker::PhantomData;
use core::num::NonZeroU32;

/// Index of an initialized pool slot. Slot `0` cannot be named.
#[repr(transparent)]
pub struct PoolIndex<T> {
    raw: NonZeroU32,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Copy for PoolIndex<T> {}

// `T` appears in the marker, so `derive(Copy)` would require `T: Copy`.
impl<T> Clone for PoolIndex<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

const _: () = {
    assert!(core::mem::size_of::<PoolIndex<u8>>() == 4);
    assert!(core::mem::align_of::<PoolIndex<u8>>() == 4);
    assert!(core::mem::size_of::<Option<PoolIndex<u8>>>() == 4);
};

impl<T> PoolIndex<T> {
    /// Name `slot` when it is non-zero.
    #[inline(always)]
    pub const fn new(slot: u32) -> Option<Self> {
        match NonZeroU32::new(slot) {
            Some(raw) => Some(Self {
                raw,
                _ty: PhantomData,
            }),
            None => None,
        }
    }

    /// Physical slot number. Always `>= 1`.
    #[inline(always)]
    pub const fn slot(self) -> u32 {
        self.raw.get()
    }
}

impl<T> core::fmt::Debug for PoolIndex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("PoolIndex").field(&self.raw.get()).finish()
    }
}

// SAFETY: `PoolIndex<T>` is `#[repr(transparent)]` over `NonZeroU32`.
// `PhantomData<fn() -> T>` is zero-sized, so every stored byte is the index.
unsafe impl<T> crate::layout::NoPadding for PoolIndex<T> {}

// SAFETY: one transparent scalar leaf. The function-pointer marker adds no
// register leaf of its own; the value contributes the single `NonZeroU32`.
unsafe impl<T> crate::layout::RegisterLeaf for PoolIndex<T> {
    const LEAVES: u8 = 1;
}

impl<T> PartialEq for PoolIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<T> Eq for PoolIndex<T> {}
impl<T> PartialOrd for PoolIndex<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for PoolIndex<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl<T> core::hash::Hash for PoolIndex<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
