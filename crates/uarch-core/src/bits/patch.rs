use super::{BitError, BitField, BitFlag};
use core::marker::PhantomData;

/// A write-only clear-and-replace update.
///
/// ```compile_fail
/// use uarch_core::bits::{BitField, BitPatch};
/// const BAD: BitPatch<u64> = BitPatch::EMPTY
///     .with_field(BitField::<u64, u8, 60, 63>::new(), 16);
/// ```
/// ```compile_fail
/// use uarch_core::bits::{BitFlag, BitPatch};
/// const BAD: BitPatch<u64> = BitPatch::EMPTY
///     .with_flag(BitFlag::new(0), true).with_flag(BitFlag::new(0), false);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitPatch<W> {
    clear_mask: u64,
    value_bits: u64,
    _w: PhantomData<fn() -> W>,
}

impl<W: Copy> BitPatch<W> {
    pub const EMPTY: Self = Self::from_parts(0, 0);
    #[inline(always)]
    pub(super) const fn from_parts(clear_mask: u64, value_bits: u64) -> Self {
        Self {
            clear_mask,
            value_bits,
            _w: PhantomData,
        }
    }
    #[inline(always)]
    pub const fn clear_mask(self) -> u64 {
        self.clear_mask
    }
    #[inline(always)]
    pub const fn value_bits(self) -> u64 {
        self.value_bits
    }
    #[inline(always)]
    pub const fn apply(self, raw: u64) -> u64 {
        (raw & !self.clear_mask) | self.value_bits
    }

    /// # Errors
    /// Returns `Conflict` if overlapping updates disagree.
    #[inline(always)]
    pub const fn try_and(self, other: Self) -> Result<Self, BitError> {
        if ((self.value_bits ^ other.value_bits) & self.clear_mask & other.clear_mask) != 0 {
            Err(BitError::Conflict)
        } else {
            Ok(Self::from_parts(
                self.clear_mask | other.clear_mask,
                self.value_bits | other.value_bits,
            ))
        }
    }
    /// # Panics
    /// Panics if overlapping updates disagree, in every build profile.
    #[inline(always)]
    pub const fn and(self, other: Self) -> Self {
        match self.try_and(other) {
            Ok(p) => p,
            Err(_) => panic!("conflicting bit patch"),
        }
    }
    /// Composes updates in order; `other` wins on all overlapping bits.
    #[inline(always)]
    pub const fn then(self, other: Self) -> Self {
        Self::from_parts(
            self.clear_mask | other.clear_mask,
            (self.value_bits & !other.clear_mask) | other.value_bits,
        )
    }
    /// # Panics
    /// Panics if the flag contradicts an existing update.
    #[inline(always)]
    pub const fn with_flag(self, flag: BitFlag<W>, enabled: bool) -> Self {
        let bits = if enabled { flag.mask() } else { 0 };
        self.and(Self::from_parts(flag.mask(), bits))
    }
    /// # Panics
    /// Panics if the raw value overflows or contradicts an existing update.
    #[inline(always)]
    pub const fn with_field<V, const LO: u8, const HI: u8>(
        self,
        field: BitField<W, V, LO, HI>,
        raw_val: u64,
    ) -> Self {
        let bits = match field.try_encode(raw_val) {
            Ok(bits) => bits,
            Err(_) => panic!("bit-field value overflow"),
        };
        self.and(Self::from_parts(field.mask(), bits))
    }
}

// SAFETY: Two repr(C) u64 fields occupy 16 bytes; the PhantomData marker has no storage.
unsafe impl<W> crate::layout::NoPadding for BitPatch<W> {}
