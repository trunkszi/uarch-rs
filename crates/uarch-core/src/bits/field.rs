use super::{BitError, BitPatch, BitPattern};
use core::marker::PhantomData;

mod sealed {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
}

/// Unsigned values supported by typed bit fields.
pub trait BitValue: sealed::Sealed + Copy {
    const BITS: u8;
    fn into_u64(self) -> u64;
    fn from_u64(value: u64) -> Self;
}

macro_rules! value {
    ($($ty:ty),*) => {$(
        impl BitValue for $ty {
            const BITS: u8 = <$ty>::BITS as u8;
            #[inline(always)]
            fn into_u64(self) -> u64 { self as u64 }
            #[inline(always)]
            fn from_u64(value: u64) -> Self { value as Self }
        }
    )*};
}
value!(u8, u16, u32, u64);

/// An inclusive bit range whose bounds participate in type checking.
///
/// ```compile_fail
/// const BAD: uarch_core::bits::BitField<u8, u16, 0, 8> = uarch_core::bits::BitField::new();
/// ```
/// ```compile_fail
/// const BAD: uarch_core::bits::BitField<u16, u32, 0, 16> = uarch_core::bits::BitField::new();
/// ```
/// ```compile_fail
/// const BAD: uarch_core::bits::BitField<u32, u64, 0, 32> = uarch_core::bits::BitField::new();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct BitField<W, V, const LO: u8, const HI: u8> {
    _marker: PhantomData<fn() -> (W, V)>,
}

impl<W, V, const LO: u8, const HI: u8> Copy for BitField<W, V, LO, HI> {}
impl<W, V, const LO: u8, const HI: u8> Clone for BitField<W, V, LO, HI> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<W, V: BitValue, const LO: u8, const HI: u8> Default for BitField<W, V, LO, HI> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<W, V: BitValue, const LO: u8, const HI: u8> BitField<W, V, LO, HI> {
    /// # Panics
    /// Panics for invalid bounds or a value type narrower than the field.
    #[doc(hidden)]
    #[inline(always)]
    pub const fn new() -> Self {
        assert!(LO <= HI && HI < 64, "invalid bit-field bounds");
        assert!(
            (HI as usize) < core::mem::size_of::<W>() * 8,
            "field upper bit exceeds backing word width"
        );
        assert!(HI - LO < V::BITS, "value type is narrower than field");
        Self {
            _marker: PhantomData,
        }
    }
    #[inline(always)]
    pub const fn get(self, raw: u64) -> V {
        value_from_u64::<V>(self.extract_raw(raw))
    }
}

impl<W, V, const LO: u8, const HI: u8> BitField<W, V, LO, HI> {
    /// Extract the field as an unsigned raw value without a trait conversion.
    #[inline(always)]
    pub const fn extract_raw(self, word: u64) -> u64 {
        (word & self.mask()) >> LO
    }

    #[inline(always)]
    pub const fn mask(&self) -> u64 {
        (u64::MAX >> (63 - (HI - LO))) << LO
    }

    /// # Errors
    /// Returns `ValueOverflow` before shifting if the raw value exceeds the field.
    #[inline(always)]
    pub const fn try_encode(&self, raw: u64) -> Result<u64, BitError> {
        if raw > (self.mask() >> LO) {
            Err(BitError::ValueOverflow)
        } else {
            Ok(raw << LO)
        }
    }
    /// Maximum unsigned value representable by this field.
    #[inline(always)]
    pub const fn max_raw(&self) -> u64 {
        self.mask() >> LO
    }

    /// Adds modulo the field width while preserving all neighboring bits.
    #[inline(always)]
    pub const fn wrapping_add_field(self, raw: u64, delta: u64) -> u64 {
        let mask = self.mask();
        (raw & !mask) | ((raw & mask).wrapping_add(delta.wrapping_shl(LO as u32)) & mask)
    }
}

impl<W, V, const LO: u8> BitField<W, V, LO, 63> {
    /// Adds to a topmost field with a single wrapping word addition.
    /// This operation requires `HI == 63`, the MSB of a 64-bit backing word.
    #[inline(always)]
    pub const fn wrapping_add_top(self, raw: u64, delta: u64) -> u64 {
        raw.wrapping_add(delta.wrapping_shl(LO as u32))
    }
}

impl<W: Copy, V: BitValue, const LO: u8, const HI: u8> BitField<W, V, LO, HI> {
    /// # Errors
    /// Returns `ValueOverflow` if the value exceeds the field width.
    #[inline(always)]
    pub const fn try_eq(self, value: V) -> Result<BitPattern<W>, BitError> {
        match self.try_encode(value_into_u64(value)) {
            Ok(bits) => Ok(BitPattern::from_parts(self.mask(), bits)),
            Err(error) => Err(error),
        }
    }
    /// # Panics
    /// Panics if the value exceeds the field width, in every build profile.
    #[inline(always)]
    pub const fn eq(self, value: V) -> BitPattern<W> {
        match self.try_eq(value) {
            Ok(p) => p,
            Err(_) => panic!("bit-field value overflow"),
        }
    }
    /// # Errors
    /// Returns `ValueOverflow` if the value exceeds the field width.
    #[inline(always)]
    pub const fn try_patch(self, value: V) -> Result<BitPatch<W>, BitError> {
        match self.try_encode(value_into_u64(value)) {
            Ok(bits) => Ok(BitPatch::from_parts(self.mask(), bits)),
            Err(error) => Err(error),
        }
    }
    /// # Panics
    /// Panics if the value exceeds the field width, in every build profile.
    #[inline(always)]
    pub const fn patch(self, value: V) -> BitPatch<W> {
        match self.try_patch(value) {
            Ok(p) => p,
            Err(_) => panic!("bit-field value overflow"),
        }
    }
}

// The sealed BitValue set contains only u8/u16/u32/u64. Selecting the native
// source width preserves byte order and initializes every byte of the result.
#[inline(always)]
const fn value_from_u64<V: BitValue>(value: u64) -> V {
    // SAFETY: Each branch selects exactly V's primitive width; all bit patterns
    // of every sealed implementor are valid, with no padding or references.
    unsafe {
        match V::BITS {
            8 => core::mem::transmute_copy(&(value as u8)),
            16 => core::mem::transmute_copy(&(value as u16)),
            32 => core::mem::transmute_copy(&(value as u32)),
            64 => core::mem::transmute_copy(&value),
            _ => unreachable!(),
        }
    }
}

#[inline(always)]
const fn value_into_u64<V: BitValue>(value: V) -> u64 {
    // SAFETY: The sealed implementor's exact width selects an identically sized
    // unsigned primitive. This copies initialized scalar bytes without aliasing.
    unsafe {
        match V::BITS {
            8 => core::mem::transmute_copy::<V, u8>(&value) as u64,
            16 => core::mem::transmute_copy::<V, u16>(&value) as u64,
            32 => core::mem::transmute_copy::<V, u32>(&value) as u64,
            64 => core::mem::transmute_copy::<V, u64>(&value),
            _ => unreachable!(),
        }
    }
}

// SAFETY: PhantomData is zero-sized, so the descriptor has no storage bytes or padding.
unsafe impl<W, V, const LO: u8, const HI: u8> crate::layout::NoPadding for BitField<W, V, LO, HI> {}
// SAFETY: The zero-sized descriptor contributes no scalar leaves regardless of marker types.
unsafe impl<W, V, const LO: u8, const HI: u8> crate::layout::RegisterLeaf
    for BitField<W, V, LO, HI>
{
    const LEAVES: u8 = 0;
}
