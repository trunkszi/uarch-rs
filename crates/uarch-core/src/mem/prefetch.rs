//! Hardware prefetch hints. The pointer is not dereferenced.

mod sealed {
    pub trait Sealed {}
}

/// Cache level of a read prefetch. Only the crate's markers implement this.
pub trait Locality: sealed::Sealed {
    /// Intel `PREFETCH` hint immediate: non-temporal `0`, T2 `1`, T1 `2`, T0 `3`.
    const X86_HINT: u8;
}

/// Keep the line in L1 and outer levels (`prefetcht0` / `pldl1keep`).
#[derive(Debug, Clone, Copy, Default)]
pub struct L1;

/// Keep the line in L2 (`prefetcht1` / `pldl2keep`).
#[derive(Debug, Clone, Copy, Default)]
pub struct L2;

/// Keep the line in L3 (`prefetcht2` / `pldl3keep`).
#[derive(Debug, Clone, Copy, Default)]
pub struct L3;

/// Non-temporal load hint (`prefetchnta` / `pldl1strm`).
#[derive(Debug, Clone, Copy, Default)]
pub struct NonTemporal;

impl sealed::Sealed for L1 {}
impl Locality for L1 {
    const X86_HINT: u8 = 3;
}

impl sealed::Sealed for L2 {}
impl Locality for L2 {
    const X86_HINT: u8 = 2;
}

impl sealed::Sealed for L3 {}
impl Locality for L3 {
    const X86_HINT: u8 = 1;
}

impl sealed::Sealed for NonTemporal {}
impl Locality for NonTemporal {
    const X86_HINT: u8 = 0;
}

/// Prefetch `ptr` without forming a reference.
///
/// The instruction is a hint. It does not authorize a later dereference.
#[inline(always)]
pub fn prefetch_read<L: Locality, T>(ptr: *const T) {
    #[cfg(all(not(miri), target_arch = "x86_64"))]
    {
        // SAFETY: `PREFETCH*` does not architecturally load or store. `pure` is
        // absent so the hint is not deleted. Flags are unchanged.
        unsafe {
            match L::X86_HINT {
                0 => core::arch::asm!(
                    "prefetchnta byte ptr [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
                1 => core::arch::asm!(
                    "prefetcht2 byte ptr [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
                2 => core::arch::asm!(
                    "prefetcht1 byte ptr [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
                _ => core::arch::asm!(
                    "prefetcht0 byte ptr [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
            }
        }
    }
    #[cfg(all(not(miri), target_arch = "aarch64"))]
    {
        // SAFETY: `prfm` is a load hint and does not architecturally write.
        // `pure` is absent so the hint is not deleted. NZCV is unchanged.
        unsafe {
            match L::X86_HINT {
                0 => core::arch::asm!(
                    "prfm pldl1strm, [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
                1 => core::arch::asm!(
                    "prfm pldl3keep, [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
                2 => core::arch::asm!(
                    "prfm pldl2keep, [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
                _ => core::arch::asm!(
                    "prfm pldl1keep, [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly),
                ),
            }
        }
    }
    #[cfg(any(miri, not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
    {
        let _ = ptr;
    }
}
