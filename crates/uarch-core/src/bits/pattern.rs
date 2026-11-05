use super::{BitError, BitField, BitFlag};
use core::marker::PhantomData;

/// A read-only conjunction of required bit values.
///
/// Contradictory constants fail during constant evaluation:
/// ```compile_fail
/// use uarch_core::bits::{BitFlag, BitPattern};
/// const BAD: BitPattern<u64> = BitPattern::ANY
///     .and_flag(BitFlag::new(0)).and_not(BitFlag::new(0));
/// ```
/// ```compile_fail
/// use uarch_core::bits::{BitField, BitPattern};
/// const BAD: BitPattern<u64> = BitPattern::ANY
///     .and_eq(BitField::<u64, u8, 60, 63>::new(), 16);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitPattern<W> {
    mask: u64,
    expected: u64,
    _w: PhantomData<fn() -> W>,
}

impl<W: Copy> BitPattern<W> {
    pub const ANY: Self = Self::from_parts(0, 0);
    #[inline(always)]
    pub(super) const fn from_parts(mask: u64, expected: u64) -> Self {
        Self {
            mask,
            expected,
            _w: PhantomData,
        }
    }
    #[inline(always)]
    pub const fn mask(self) -> u64 {
        self.mask
    }
    #[inline(always)]
    pub const fn expected(self) -> u64 {
        self.expected
    }
    #[inline(always)]
    pub const fn matches(self, raw: u64) -> bool {
        (raw & self.mask) == self.expected
    }

    /// # Errors
    /// Returns `Conflict` if overlapping required values differ.
    #[inline(always)]
    pub const fn try_and(self, other: Self) -> Result<Self, BitError> {
        if ((self.expected ^ other.expected) & self.mask & other.mask) != 0 {
            Err(BitError::Conflict)
        } else {
            Ok(Self::from_parts(
                self.mask | other.mask,
                self.expected | other.expected,
            ))
        }
    }
    /// # Panics
    /// Panics if overlapping requirements disagree.
    #[inline(always)]
    pub const fn and(self, other: Self) -> Self {
        match self.try_and(other) {
            Ok(p) => p,
            Err(_) => panic!("conflicting bit pattern"),
        }
    }
    /// # Panics
    /// Panics if the flag is already required to be clear.
    #[inline(always)]
    pub const fn and_flag(self, flag: BitFlag<W>) -> Self {
        self.and(Self::from_parts(flag.mask(), flag.mask()))
    }
    /// # Panics
    /// Panics if the flag is already required to be set.
    #[inline(always)]
    pub const fn and_not(self, flag: BitFlag<W>) -> Self {
        self.and(Self::from_parts(flag.mask(), 0))
    }
    /// # Panics
    /// Panics if the raw value overflows or contradicts an existing requirement.
    #[inline(always)]
    pub const fn and_eq<V, const LO: u8, const HI: u8>(
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
unsafe impl<W> crate::layout::NoPadding for BitPattern<W> {}
