use crate::layout::{NoPadding, RegisterLeaf};
use core::iter::FusedIterator;

/// A scalar bitmap with ascending set-bit traversal.
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitmap64(u64);

// SAFETY: Transparent u64 storage has no padding.
unsafe impl NoPadding for Bitmap64 {}
// SAFETY: Transparent u64 storage contributes one native scalar leaf.
unsafe impl RegisterLeaf for Bitmap64 {
    const LEAVES: u8 = 1;
}

impl Bitmap64 {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(u64::MAX);
    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
    #[inline(always)]
    pub const fn count_ones(self) -> u32 {
        self.0.count_ones()
    }
    #[inline(always)]
    pub const fn clear_lowest_one(self) -> Self {
        Self(self.0 & self.0.wrapping_sub(1))
    }
    #[inline(always)]
    pub const fn highest_one_index(self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            Some(63 - self.0.leading_zeros())
        }
    }
    #[inline(always)]
    pub const fn highest_one_bit(self) -> u64 {
        if self.0 == 0 {
            0
        } else {
            (1u64 << 63) >> self.0.leading_zeros()
        }
    }
    #[inline(always)]
    pub const fn clear_highest_one(self) -> Self {
        Self(self.0 ^ self.highest_one_bit())
    }
    #[inline(always)]
    pub const fn pop_highest_one(&mut self) -> Option<u32> {
        let bit = self.highest_one_index();
        *self = self.clear_highest_one();
        bit
    }
    #[inline(always)]
    pub const fn lowest_zero_index(self) -> Option<u32> {
        Self(!self.0).lowest_one_index()
    }
    #[inline(always)]
    pub const fn next_one_from(self, min_bit: u32) -> Option<u32> {
        if min_bit >= 64 {
            None
        } else {
            Self(self.0 & (u64::MAX << min_bit)).lowest_one_index()
        }
    }
    #[inline(always)]
    pub const fn prev_one_from(self, max_bit: u32) -> Option<u32> {
        if max_bit >= 64 {
            self.highest_one_index()
        } else {
            Self(self.0 & (u64::MAX >> (63 - max_bit))).highest_one_index()
        }
    }
    /// Test a bit by index.
    ///
    /// # Panics
    /// Panics if the bit is outside 0..64, in every profile.
    #[inline(always)]
    pub const fn test(self, bit: u32) -> bool {
        assert!(bit < 64);
        self.0 & (1u64 << bit) != 0
    }
    /// Set a bit by index.
    ///
    /// # Panics
    /// Panics if the bit is outside 0..64, in every profile.
    #[inline(always)]
    pub const fn set(self, bit: u32) -> Self {
        assert!(bit < 64);
        Self(self.0 | (1u64 << bit))
    }
    /// Clear a bit by index.
    ///
    /// # Panics
    /// Panics if the bit is outside 0..64, in every profile.
    #[inline(always)]
    pub const fn clear(self, bit: u32) -> Self {
        assert!(bit < 64);
        Self(self.0 & !(1u64 << bit))
    }
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[inline(always)]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
    #[inline(always)]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
    #[inline(always)]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }
    #[inline(always)]
    pub const fn raw(self) -> u64 {
        self.0
    }
    #[inline(always)]
    pub const fn lowest_one_index(self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros())
        }
    }
    #[inline(always)]
    pub const fn pop_lowest_one(&mut self) -> Option<u32> {
        let first = self.lowest_one_index();
        self.0 &= self.0.wrapping_sub(1);
        first
    }
    #[inline(always)]
    pub const fn lowest_one_bit(self) -> u64 {
        self.0 & self.0.wrapping_neg()
    }
    /// Returns the lowest `n` bits, saturating at 64.
    #[inline(always)]
    pub const fn low_mask(n: u32) -> u64 {
        if n >= 64 { u64::MAX } else { (1u64 << n) - 1 }
    }
    #[inline(always)]
    pub const fn iter_ones(self) -> Ones {
        Ones(self)
    }
}

impl IntoIterator for Bitmap64 {
    type Item = u32;
    type IntoIter = Ones;
    #[inline(always)]
    fn into_iter(self) -> Ones {
        self.iter_ones()
    }
}

/// Visits each set bit once in ascending index order.
#[derive(Debug, Clone)]
pub struct Ones(Bitmap64);

impl Iterator for Ones {
    type Item = u32;
    #[inline(always)]
    fn next(&mut self) -> Option<u32> {
        self.0.pop_lowest_one()
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.0.raw().count_ones() as usize;
        (count, Some(count))
    }
}
impl ExactSizeIterator for Ones {}
impl FusedIterator for Ones {}

impl core::ops::Not for Bitmap64 {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self {
        Self(!self.0)
    }
}
impl core::ops::BitOr for Bitmap64 {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl core::ops::BitOrAssign for Bitmap64 {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
impl core::ops::BitAnd for Bitmap64 {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}
impl core::ops::BitAndAssign for Bitmap64 {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}
impl core::ops::BitXor for Bitmap64 {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}
impl core::ops::BitXorAssign for Bitmap64 {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}
