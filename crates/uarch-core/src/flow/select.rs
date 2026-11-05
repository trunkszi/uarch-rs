//! Register `cmov` / `csel`. Flags are clobbered, so `preserves_flags` is omitted.
//! `pure` is omitted so the block cannot be deleted as a pure computation.

use super::Predicate;

mod sealed {
    pub trait Sealed {}
}

/// Integers that fit in one general-purpose register and support conditional move.
pub trait RegisterSelectable: Copy + sealed::Sealed {
    /// Move `if_true` into the result when `cond` is set.
    #[doc(hidden)]
    fn select_register(cond: Predicate, if_true: Self, if_false: Self) -> Self;
}

#[cfg(all(target_arch = "x86_64", not(miri)))]
#[inline(always)]
fn select_u64(cond: u8, if_true: u64, if_false: u64) -> u64 {
    let mut out = if_false;
    // SAFETY: `cond` is 0 or 1. `test` sets ZF from the low 32 bits and `cmovne`
    // overwrites `out` only when ZF is clear. Operands are registers, so there
    // is no memory access. Flags stay clobbered on purpose.
    unsafe {
        core::arch::asm!(
            "test {cond:e}, {cond:e}",
            "cmovne {out}, {if_true}",
            cond = in(reg) u32::from(cond),
            if_true = in(reg) if_true,
            out = inout(reg) out,
            options(nostack, nomem),
        );
    }
    out
}

#[cfg(all(target_arch = "x86_64", not(miri)))]
#[inline(always)]
fn select_u32(cond: u8, if_true: u32, if_false: u32) -> u32 {
    let mut out = if_false;
    // SAFETY: same contract as `select_u64`, using 32-bit register views so the
    // written result is exactly the `u32` payload.
    unsafe {
        core::arch::asm!(
            "test {cond:e}, {cond:e}",
            "cmovne {out:e}, {if_true:e}",
            cond = in(reg) u32::from(cond),
            if_true = in(reg) if_true,
            out = inout(reg) out,
            options(nostack, nomem),
        );
    }
    out
}

#[cfg(all(target_arch = "aarch64", not(miri)))]
#[inline(always)]
fn select_u64(cond: u8, if_true: u64, if_false: u64) -> u64 {
    let out: u64;
    // SAFETY: `cond` is 0 or 1 in a full register. `cmp` writes NZCV and `csel`
    // writes only `out`. No memory access. Flags stay clobbered on purpose.
    unsafe {
        core::arch::asm!(
            "cmp {cond}, #0",
            "csel {out}, {if_true}, {if_false}, ne",
            cond = in(reg) u64::from(cond),
            if_true = in(reg) if_true,
            if_false = in(reg) if_false,
            out = out(reg) out,
            options(nostack, nomem),
        );
    }
    out
}

#[cfg(all(target_arch = "aarch64", not(miri)))]
#[inline(always)]
fn select_u32(cond: u8, if_true: u32, if_false: u32) -> u32 {
    let out: u32;
    // SAFETY: same contract as the 64-bit `csel`, using `w` register views.
    unsafe {
        core::arch::asm!(
            "cmp {cond:w}, #0",
            "csel {out:w}, {if_true:w}, {if_false:w}, ne",
            cond = in(reg) u32::from(cond),
            if_true = in(reg) if_true,
            if_false = in(reg) if_false,
            out = out(reg) out,
            options(nostack, nomem),
        );
    }
    out
}

#[cfg(any(miri, not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
#[inline(always)]
fn select_u64(cond: u8, if_true: u64, if_false: u64) -> u64 {
    core::hint::select_unpredictable(cond != 0, if_true, if_false)
}

#[cfg(any(miri, not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
#[inline(always)]
fn select_u32(cond: u8, if_true: u32, if_false: u32) -> u32 {
    core::hint::select_unpredictable(cond != 0, if_true, if_false)
}

macro_rules! impl_select_wide {
    ($($ty:ty),+ $(,)?) => {$(
        impl sealed::Sealed for $ty {}
        impl RegisterSelectable for $ty {
            #[inline(always)]
            fn select_register(cond: Predicate, if_true: Self, if_false: Self) -> Self {
                select_u64(cond.bit(), if_true as u64, if_false as u64) as Self
            }
        }
    )+};
}

macro_rules! impl_select_narrow {
    ($($ty:ty),+ $(,)?) => {$(
        impl sealed::Sealed for $ty {}
        impl RegisterSelectable for $ty {
            #[inline(always)]
            fn select_register(cond: Predicate, if_true: Self, if_false: Self) -> Self {
                select_u32(cond.bit(), if_true as u32, if_false as u32) as Self
            }
        }
    )+};
}

impl_select_wide!(u64, i64);
impl_select_narrow!(u32, i32);

impl sealed::Sealed for usize {}
impl RegisterSelectable for usize {
    #[inline(always)]
    fn select_register(cond: Predicate, if_true: Self, if_false: Self) -> Self {
        #[cfg(target_pointer_width = "64")]
        {
            select_u64(cond.bit(), if_true as u64, if_false as u64) as usize
        }
        #[cfg(not(target_pointer_width = "64"))]
        {
            select_u32(cond.bit(), if_true as u32, if_false as u32) as usize
        }
    }
}

impl sealed::Sealed for isize {}
impl RegisterSelectable for isize {
    #[inline(always)]
    fn select_register(cond: Predicate, if_true: Self, if_false: Self) -> Self {
        #[cfg(target_pointer_width = "64")]
        {
            select_u64(cond.bit(), if_true as u64, if_false as u64) as isize
        }
        #[cfg(not(target_pointer_width = "64"))]
        {
            select_u32(cond.bit(), if_true as u32, if_false as u32) as isize
        }
    }
}
