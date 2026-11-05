//! Unsigned 64-bit division by an invariant divisor.
//!
//! `d = 1` is the power-of-two shift path. The generator never evaluates
//! `u64::MAX / d + 1`, which overflows when `d` is 1.

use core::num::NonZeroU64;

const ADD_MARKER: u64 = 0x40;
const SHIFT_MASK: u64 = 0x3F;

/// Precomputed unsigned division by a non-zero `u64` divisor.
///
/// The hot path is a high multiply plus shifts. It does not execute `div`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FastDivisorU64 {
    divisor: NonZeroU64,
    magic: u64,
    more: u64,
}

// SAFETY: Three repr(C) eight-byte scalar fields initialize all 24 bytes without padding.
unsafe impl crate::layout::NoPadding for FastDivisorU64 {}
crate::assert_layout!(FastDivisorU64, size = 24, align = 8);

impl FastDivisorU64 {
    /// Original non-zero divisor used to build these parameters.
    #[inline(always)]
    pub const fn divisor(self) -> NonZeroU64 {
        self.divisor
    }

    /// Whether the divisor can use a shift and mask.
    #[inline(always)]
    pub const fn is_power_of_two(self) -> bool {
        // For non-powers of two, l = floor(log2(d)) and p = floor(2^(64+l)/d)
        // satisfy 2^63 <= p <= 2^64-2. The non-add magic p+1 cannot wrap to zero.
        // Add magic 2*p+1 is odd; (2*p+2) modulo 2^64 lies in [2, 2^64-2].
        // Thus new() sets magic=0 exactly for powers of two.
        self.magic == 0
    }

    /// Build the libdivide parameters for `divisor`.
    #[inline]
    pub const fn new(divisor: NonZeroU64) -> Self {
        let d = divisor.get();
        let floor_log_2_d = 63 - d.leading_zeros();
        if d & d.wrapping_sub(1) == 0 {
            return Self {
                divisor,
                magic: 0,
                more: floor_log_2_d as u64,
            };
        }
        let numerator = 1u128 << (floor_log_2_d + 64);
        let den = d as u128;
        let proposed = numerator / den;
        let rem = (numerator % den) as u64;
        let mut proposed_m = proposed as u64;
        let e = d - rem;
        let more = if e < (1u64 << floor_log_2_d) {
            floor_log_2_d as u64
        } else {
            proposed_m = proposed_m.wrapping_add(proposed_m);
            let twice_rem = rem.wrapping_add(rem);
            if twice_rem >= d || twice_rem < rem {
                proposed_m = proposed_m.wrapping_add(1);
            }
            (floor_log_2_d as u64) | ADD_MARKER
        };
        Self {
            divisor,
            magic: proposed_m.wrapping_add(1),
            more,
        }
    }

    /// `floor(n / divisor)`.
    #[inline(always)]
    pub const fn div(self, n: u64) -> u64 {
        self.quotient::<false>(n)
    }

    #[inline(always)]
    const fn quotient<const KEEP_CORRECTION_BRANCH: bool>(self, n: u64) -> u64 {
        let magic = self.magic;
        let more = self.more;
        if magic == 0 {
            n.wrapping_shr(more as u32)
        } else {
            let q = multiply_high_u64(n, magic);
            let correction = more & ADD_MARKER != 0;
            // Remainders need the invariant branch to avoid speculative correction; ordinary division keeps its code layout.
            let correction = if KEEP_CORRECTION_BRANCH {
                crate::flow::unlikely(correction)
            } else {
                correction
            };
            if correction {
                let shifted = n.wrapping_sub(q).wrapping_shr(1);
                shifted
                    .wrapping_add(q)
                    .wrapping_shr((more & SHIFT_MASK) as u32)
            } else {
                q.wrapping_shr(more as u32)
            }
        }
    }

    /// `n % divisor`, from the same quotient as [`Self::div`].
    #[inline(always)]
    pub const fn rem(self, n: u64) -> u64 {
        if self.magic == 0 {
            return n & (self.divisor.get() - 1);
        }
        self.div_rem(n).1
    }

    /// Quotient and remainder. `q * d + r == n` and `r < d` when `div` is exact.
    #[inline(always)]
    pub const fn div_rem(self, n: u64) -> (u64, u64) {
        if self.magic == 0 {
            let d = self.divisor.get();
            return (n.wrapping_shr(self.more as u32), n & (d - 1));
        }
        let q = self.quotient::<true>(n);
        let product = q.wrapping_mul(self.divisor.get());
        (q, n.wrapping_sub(product))
    }
    /// Whether the dividend has zero remainder.
    #[inline(always)]
    pub const fn is_multiple_of(self, dividend: u64) -> bool {
        self.rem(dividend) == 0
    }
}

#[inline(always)]
const fn multiply_high_u64(a: u64, b: u64) -> u64 {
    let wide = (a as u128).wrapping_mul(b as u128);
    (wide >> 64) as u64
}
