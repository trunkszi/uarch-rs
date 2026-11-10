/// Linux exposes process-wide HWCAP as the system-safe common capability set.
/// Unknown environments are conservatively treated as lacking NEON proof.
pub fn neon_supported() -> bool {
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        unsafe extern "C" {
            fn getauxval(kind: core::ffi::c_ulong) -> core::ffi::c_ulong;
        }
        // SAFETY: AT_HWCAP (16) is a read-only libc query with no pointer inputs.
        unsafe { getauxval(16) & (1 << 1) != 0 }
    }
    #[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
    {
        false
    }
}
