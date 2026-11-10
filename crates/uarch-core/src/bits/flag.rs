use core::marker::PhantomData;

/// A single readable or writable flag in a particular word type.
///
/// ```compile_fail
/// const BAD: uarch_core::bits::BitFlag<u8> = uarch_core::bits::BitFlag::new(8);
/// ```
/// ```compile_fail
/// const BAD: uarch_core::bits::BitFlag<u16> = uarch_core::bits::BitFlag::new(16);
/// ```
/// ```compile_fail
/// const BAD: uarch_core::bits::BitFlag<u32> = uarch_core::bits::BitFlag::new(32);
/// ```
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitFlag<W> {
    mask: u64,
    _w: PhantomData<fn() -> W>,
}

impl<W> BitFlag<W> {
    /// # Panics
    /// Panics if the index exceeds 63 or the backing word storage width.
    #[doc(hidden)]
    #[inline(always)]
    pub const fn new(index: u8) -> Self {
        assert!(index < 64, "flag index exceeds word width");
        assert!(
            (index as usize) < core::mem::size_of::<W>() * 8,
            "bit index exceeds backing word width"
        );
        Self {
            mask: 1u64 << index,
            _w: PhantomData,
        }
    }
    /// Test this flag in a raw integer word during constant evaluation.
    #[inline(always)]
    pub const fn test_raw(self, raw_word: u64) -> bool {
        raw_word & self.mask != 0
    }
    #[inline(always)]
    pub const fn mask(&self) -> u64 {
        self.mask
    }
}

/// A set of flags for setting or toggling bits.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagSet<W> {
    pub(super) mask: u64,
    pub(super) _w: PhantomData<fn() -> W>,
}

impl<W> FlagSet<W> {
    #[inline(always)]
    pub const fn mask(&self) -> u64 {
        self.mask
    }
}

/// Flags to clear, also usable as negative query terms.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagClear<W> {
    pub(super) clear_mask: u64,
    pub(super) _w: PhantomData<fn() -> W>,
}

impl<W> FlagClear<W> {
    #[inline(always)]
    pub const fn mask(&self) -> u64 {
        self.clear_mask
    }
}

// SAFETY: Transparent u64 storage and a zero-sized marker contain no padding.
unsafe impl<W> crate::layout::NoPadding for BitFlag<W> {}
// SAFETY: The transparent u64 representation contributes exactly one scalar leaf.
unsafe impl<W: Copy> crate::layout::RegisterLeaf for BitFlag<W> {
    const LEAVES: u8 = 1;
}

// SAFETY: Transparent u64 storage and a zero-sized marker contain no padding.
unsafe impl<W> crate::layout::NoPadding for FlagSet<W> {}
// SAFETY: The transparent u64 representation contributes exactly one scalar leaf.
unsafe impl<W: Copy> crate::layout::RegisterLeaf for FlagSet<W> {
    const LEAVES: u8 = 1;
}

// SAFETY: Transparent u64 storage and a zero-sized marker contain no padding.
unsafe impl<W> crate::layout::NoPadding for FlagClear<W> {}
// SAFETY: The transparent u64 representation contributes exactly one scalar leaf.
unsafe impl<W: Copy> crate::layout::RegisterLeaf for FlagClear<W> {
    const LEAVES: u8 = 1;
}
