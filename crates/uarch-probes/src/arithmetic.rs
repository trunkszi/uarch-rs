//! Audit symbols for invariant division. The body is only the hot path.

use uarch_core::arithmetic::{FastDivisorU32, FastDivisorU64};

// Monomorphic symbol. The asm gate forbids `div`, `idiv`, and `__udivti3`
// in this body. Construction of `FastDivisorU64` stays outside the symbol.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn fast_divisor_u64_div(divisor: FastDivisorU64, n: u64) -> u64 {
    divisor.div(n)
}

// Monomorphic symbol for the Lemire remainder multiply.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn fast_divisor_u32_rem(divisor: FastDivisorU32, n: u32) -> u32 {
    divisor.rem(n)
}

/// Concrete remainder path with a runtime divisor prepared outside this symbol.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn fast_divisor_u64_rem(divisor: FastDivisorU64, n: u64) -> u64 {
    divisor.rem(n)
}

/// Runtime power-of-two remainder path, with construction outside the audited symbol.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn fast_divisor_u64_pow2_rem(divisor: FastDivisorU64, n: u64) -> u64 {
    if !divisor.is_power_of_two() {
        return 0;
    }
    divisor.rem(n)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn fast_divisor_u32_div(divisor: FastDivisorU32, n: u32) -> u32 {
    divisor.div(n)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn power_of_two_align(n: u64) -> u64 {
    use uarch_core::arithmetic::PowerOfTwo;
    PowerOfTwo::<6>::align_down_u64(n) ^ PowerOfTwo::<6>::alignment_offset_u64(n)
}
