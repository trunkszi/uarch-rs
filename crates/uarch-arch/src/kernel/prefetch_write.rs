#[inline(always)]
pub(crate) unsafe fn prefetch_write(ptr: *const u8) {
    #[cfg(all(not(miri), target_arch = "x86_64"))]
    // SAFETY: The caller's PrefetchWrite capability proves instruction support.
    unsafe {
        core::arch::asm!("prefetchw [{ptr}]", ptr = in(reg) ptr,
            options(nostack, preserves_flags, readonly));
    }
    #[cfg(all(not(miri), target_arch = "aarch64"))]
    // SAFETY: PRFM is part of the aarch64 baseline and does not dereference Rust memory.
    unsafe {
        core::arch::asm!("prfm pstl1keep, [{ptr}]", ptr = in(reg) ptr,
            options(nostack, preserves_flags, readonly));
    }
    #[cfg(any(miri, not(any(target_arch = "x86_64", target_arch = "aarch64"))))]
    let _ = ptr;
}
