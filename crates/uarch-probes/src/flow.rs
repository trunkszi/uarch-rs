//! Audit symbols for predicate select and cold hints.

use uarch_core::flow::{cold_path, predicate};

// Monomorphic symbol so the asm gate can find `test`/`cmovne` or `csel`.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn flow_select_register_u64(cond: bool, if_true: u64, if_false: u64) -> u64 {
    predicate(cond).select_register(if_true, if_false)
}

// Monomorphic symbol for the `select_unpredictable` hint path.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn flow_select_u64(cond: bool, if_true: u64, if_false: u64) -> u64 {
    predicate(cond).select(if_true, if_false)
}

// Monomorphic symbol so a cold edge can be placed in `.text.unlikely.*`.
#[unsafe(no_mangle)]
#[inline(never)]
#[cold]
pub fn flow_cold_path(value: u64) -> u64 {
    cold_path();
    value
}

/// Concrete branch-weight evidence for the stable likely hint.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn flow_likely(condition: bool, value: u64) -> u64 {
    if uarch_core::flow::likely(condition) {
        core::hint::black_box(value).wrapping_add(1)
    } else {
        core::hint::black_box(value.wrapping_mul(7))
    }
}

/// Concrete branch-weight evidence for the stable unlikely hint.
// Distinct arithmetic preserves independent controls after LLVM function merging.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn flow_unlikely(condition: bool, value: u64) -> u64 {
    if uarch_core::flow::unlikely(condition) {
        core::hint::black_box(value).wrapping_add(1)
    } else {
        core::hint::black_box(value.wrapping_mul(11))
    }
}
