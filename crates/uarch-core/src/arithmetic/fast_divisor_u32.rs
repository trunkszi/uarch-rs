//! Lemire 32-bit remainder. Magic uses wrapping add so `d = 1` becomes 0.

use core::num::{NonZeroU32, NonZeroU64};

/// Precomputed unsigned division and remainder by a non-zero `u32` divisor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FastDivisorU32 {
    divisor: NonZeroU64,
    /// `ceil(2^64 / d)` with wrap, so `d = 1` stores `0`.
    magic: u64,
}

// SAFETY: The repr(C) nonzero u64 and u64 fields occupy all 16 bytes without padding.
unsafe impl crate::layout::NoPadding for FastDivisorU32 {}
crate::assert_layout!(FastDivisorU32, size = 16, align = 8);

impl FastDivisorU32 {
    /// Original non-zero divisor used to build these parameters.
    #[inline(always)]
    pub const fn divisor(self) -> NonZeroU32 {
        match NonZeroU32::new(self.divisor.get() as u32) {
            Some(divisor) => divisor,
            None => NonZeroU32::MIN,
        }
    }

    /// Whether the original divisor is a power of two; this implementation uses Lemire parameters.
    #[inline(always)]
    pub const fn is_power_of_two(self) -> bool {
        self.divisor.is_power_of_two()
    }

    /// Build the Lemire multiplier.
    #[inline]
    pub const fn new(divisor: NonZeroU32) -> Self {
        let d = divisor.get();
        let magic = (u64::MAX / (d as u64)).wrapping_add(1);
        let divisor = match NonZeroU64::new(d as u64) {
            Some(divisor) => divisor,
            None => NonZeroU64::MIN,
        };
        Self { divisor, magic }
    }

    /// `n % divisor`.
    #[inline(always)]
    pub const fn rem(self, n: u32) -> u32 {
        let low = self.magic.wrapping_mul(n as u64);
        let wide = (low as u128).wrapping_mul(self.divisor.get() as u128);
        (wide >> 64) as u32
    }

    /// `floor(n / divisor)`.
    ///
    /// Lemire's multiply-high quotient is specified for `d > 1`. `d = 1` has a
    /// zero magic, whose high half would be `0` instead of `n`.
    #[inline(always)]
    pub const fn div(self, n: u32) -> u32 {
        if self.magic == 0 {
            n
        } else {
            let wide = (self.magic as u128).wrapping_mul(n as u128);
            (wide >> 64) as u32
        }
    }

    /// `true` when `n` is a multiple of the divisor.
    #[inline(always)]
    pub const fn is_multiple_of(self, n: u32) -> bool {
        let magic = self.magic;
        (n as u64).wrapping_mul(magic) <= magic.wrapping_sub(1)
    }
    /// Quotient and remainder from the same exact quotient.
    #[inline(always)]
    pub const fn div_rem(self, dividend: u32) -> (u32, u32) {
        let quotient = self.div(dividend);
        let product = (quotient as u64).wrapping_mul(self.divisor.get());
        (quotient, dividend.wrapping_sub(product as u32))
    }
}
