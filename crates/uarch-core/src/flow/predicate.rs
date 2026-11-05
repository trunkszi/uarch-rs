//! `0 | 1` predicate word. Bitwise `!` would leave `0xFF`, so negation is `xor 1`.

use core::ops::{BitAnd, BitOr, BitXor, Mul, Not};

/// Branchless predicate. The private byte is only ever `0` or `1`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Predicate(u8);

const _: () = {
    assert!(core::mem::size_of::<Predicate>() == 1);
    assert!(core::mem::align_of::<Predicate>() == 1);
};

impl Predicate {
    /// Construct a normalized predicate from a boolean.
    #[inline(always)]
    pub const fn new(cond: bool) -> Self {
        Self(cond as u8)
    }

    #[inline(always)]
    pub(crate) const fn bit(self) -> u8 {
        self.0
    }

    /// `true` when the predicate bit is set.
    #[inline(always)]
    pub const fn as_bool(self) -> bool {
        self.0 != 0
    }

    /// `1` keeps `val`; `0` is wrapping negation.
    ///
    /// `i64::MIN` stays `i64::MIN` because negation is modulo `2^64`.
    #[inline(always)]
    pub const fn select_sign_i64(self, val: i64) -> i64 {
        let mask = (self.0 as i64).wrapping_sub(1);
        (val ^ mask).wrapping_sub(mask)
    }

    /// Compiler predicate hint. This is not a guarantee of `cmov` or `csel`.
    #[inline(always)]
    pub fn select<T>(self, if_true: T, if_false: T) -> T {
        core::hint::select_unpredictable(self.as_bool(), if_true, if_false)
    }

    /// Register conditional move for sealed machine words.
    ///
    /// Both arms are evaluated. This does not make the surrounding code
    /// constant-time, and it does not guard a later arm against faults.
    #[inline(always)]
    pub fn select_register<T: super::RegisterSelectable>(self, if_true: T, if_false: T) -> T {
        T::select_register(self, if_true, if_false)
    }
}

/// Build a predicate. `true` is `1` and `false` is `0`.
#[inline(always)]
pub const fn predicate(cond: bool) -> Predicate {
    Predicate::new(cond)
}

impl From<bool> for Predicate {
    #[inline(always)]
    fn from(cond: bool) -> Self {
        Self::new(cond)
    }
}

// SAFETY: `Predicate` is `#[repr(transparent)]` over one `u8`. Both legal values
// initialize that only byte, so the value has no padding.
unsafe impl crate::layout::NoPadding for Predicate {}

// SAFETY: one `#[repr(transparent)]` scalar leaf. It contributes a single
// register leaf under the Rust scalar ABI, the same as the inner `u8`.
unsafe impl crate::layout::RegisterLeaf for Predicate {
    const LEAVES: u8 = 1;
}

mod mask_seal {
    pub trait Sealed {}
}

/// Integers that can be cleared by a `0` / all-ones mask.
///
/// The mask is `0 - bit`, not a real multiply.
pub trait MaskWord: Copy + mask_seal::Sealed {
    /// `self` when `bit == 1`, and zero when `bit == 0`.
    fn mask(self, bit: u8) -> Self;
}

macro_rules! impl_mask_word {
    ($($ty:ty),+ $(,)?) => {$(
        impl mask_seal::Sealed for $ty {}
        impl MaskWord for $ty {
            #[inline(always)]
            fn mask(self, bit: u8) -> Self {
                self & (bit as Self).wrapping_neg()
            }
        }
    )+};
}

impl_mask_word!(u8, u16, u32, u64, i8, i16, i32, i64, usize, isize);

impl<T: MaskWord> Mul<T> for Predicate {
    type Output = T;

    #[inline(always)]
    fn mul(self, rhs: T) -> T {
        rhs.mask(self.0)
    }
}

impl BitAnd for Predicate {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for Predicate {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitXor for Predicate {
    type Output = Self;

    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}

impl Not for Predicate {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self {
        Self(self.0 ^ 1)
    }
}
