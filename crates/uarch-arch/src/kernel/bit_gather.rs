#[inline(always)]
pub(crate) fn gather(val: u64, mut mask: u64) -> u64 {
    let mut result = 0;
    let mut output = 1u64;
    while mask != 0 {
        let bit = mask & mask.wrapping_neg();
        result |= u64::from(val & bit != 0).wrapping_neg() & output;
        mask &= mask - 1;
        output = output.wrapping_shl(1);
    }
    result
}

#[inline(always)]
pub(crate) fn scatter(val: u64, mut mask: u64) -> u64 {
    let mut result = 0;
    let mut input = val;
    while mask != 0 {
        let bit = mask & mask.wrapping_neg();
        result |= (input & 1).wrapping_neg() & bit;
        input >>= 1;
        mask &= mask - 1;
    }
    result
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2")]
#[inline]
pub(crate) unsafe fn gather_fast(val: u64, mask: u64) -> u64 {
    core::arch::x86_64::_pext_u64(val, mask)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2")]
#[inline]
pub(crate) unsafe fn scatter_fast(val: u64, mask: u64) -> u64 {
    core::arch::x86_64::_pdep_u64(val, mask)
}

// Inline assembly prevents LLVM from replacing the slow-cost tier's loops
// with microcoded PEXT/PDEP when the surrounding clone enables BMI2.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi1")]
#[inline]
pub(crate) unsafe fn gather_slow(val: u64, mask: u64) -> u64 {
    #[cfg(miri)]
    {
        gather(val, mask)
    }
    #[cfg(not(miri))]
    {
        let mut mask = mask;
        let mut result = 0;
        let mut output = 1u64;
        while mask != 0 {
            let index: u64;
            let next: u64;
            // SAFETY: The enclosing BMI1 function and nonzero mask permit both instructions.
            unsafe {
                core::arch::asm!(
                    "tzcnt {index}, {mask}", "blsr {next}, {mask}",
                    index = out(reg) index, next = out(reg) next, mask = in(reg) mask,
                    options(pure, nomem, nostack),
                );
            }
            result |= ((val >> index) & 1).wrapping_neg() & output;
            output = output.wrapping_shl(1);
            mask = next;
        }
        result
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi1")]
#[inline]
pub(crate) unsafe fn scatter_slow(val: u64, mask: u64) -> u64 {
    #[cfg(miri)]
    {
        scatter(val, mask)
    }
    #[cfg(not(miri))]
    {
        let mut mask = mask;
        let mut val = val;
        let mut result = 0;
        while mask != 0 {
            let index: u64;
            let next: u64;
            // SAFETY: The enclosing BMI1 function and nonzero mask permit both instructions.
            unsafe {
                core::arch::asm!(
                    "tzcnt {index}, {mask}", "blsr {next}, {mask}",
                    index = out(reg) index, next = out(reg) next, mask = in(reg) mask,
                    options(pure, nomem, nostack),
                );
            }
            result |= (val & 1) << index;
            val >>= 1;
            mask = next;
        }
        result
    }
}
