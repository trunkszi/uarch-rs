use uarch_arch::{Arch, Scalar};

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn uarch_arch_scalar_gather(value: u64, mask: u64) -> u64 {
    Scalar::new().bit_gather(value, mask)
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V3 ISA and OS state.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave"
)]
pub unsafe extern "C" fn x86_v3_gather(value: u64, mask: u64) -> u64 {
    // SAFETY: The caller proves the complete V3 execution-domain contract.
    unsafe { uarch_arch::X86V3::new_unchecked() }.bit_gather(value, mask)
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V3 ISA and OS state.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave"
)]
pub unsafe extern "C" fn x86_v3_slow_pdep_gather(value: u64, mask: u64) -> u64 {
    // SAFETY: The caller proves the complete V3 execution-domain contract.
    unsafe { uarch_arch::X86V3SlowPdep::new_unchecked() }.bit_gather(value, mask)
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V3 ISA and OS state.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave"
)]
pub unsafe extern "C" fn x86_v3_slow_pdep_scatter(value: u64, mask: u64) -> u64 {
    // SAFETY: The caller proves the complete V3 execution-domain contract.
    unsafe { uarch_arch::X86V3SlowPdep::new_unchecked() }.bit_scatter(value, mask)
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V3 ISA and OS state.
/// `keys` and `vals` must each point to four initialized, readable i64 values.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave"
)]
pub unsafe extern "C" fn uarch_arch_masked_sum(
    keys: *const [i64; 4],
    vals: *const [i64; 4],
    limit: i64,
) -> i64 {
    // SAFETY: The caller proves V3 support and validity of both readable arrays.
    unsafe { uarch_arch::X86V3::new_unchecked().masked_sum_lt_i64x4(&*keys, &*vals, limit) }
}

/// # Safety
/// Every execution CPU must support write-prefetch (PREFETCHW on x86_64,
/// PRFM on aarch64). This probe is unavailable on other targets.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn uarch_arch_prefetch_write(ptr: *const u8) {
    // SAFETY: The caller proves write-prefetch support for the execution domain.
    unsafe { uarch_arch::PrefetchWrite::new_unchecked() }.prefetch_write(ptr);
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V3 ISA and OS state.
/// `keys` must point to four initialized, readable i64 values.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave"
)]
pub unsafe extern "C" fn uarch_arch_first_ge(keys: *const [i64; 4], target: i64) -> u32 {
    // SAFETY: The caller proves V3 support and the array's readable validity.
    unsafe { uarch_arch::X86V3::new_unchecked().find_first_ge_i64x4(&*keys, target) }.unwrap_or(4)
}
/// # Safety
/// All possible execution CPUs must support the complete x86 V3 ISA and OS state.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave"
)]
pub unsafe extern "C" fn x86_v3_scatter(value: u64, mask: u64) -> u64 {
    // SAFETY: The caller proves the complete V3 execution-domain contract.
    unsafe { uarch_arch::X86V3::new_unchecked() }.bit_scatter(value, mask)
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V4 ISA and OS state.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave,avx512f,avx512bw,avx512cd,avx512dq,avx512vl"
)]
pub unsafe extern "C" fn x86_v4_gather(value: u64, mask: u64) -> u64 {
    // SAFETY: The caller proves the complete V4 execution-domain contract.
    unsafe { uarch_arch::X86V4::new_unchecked() }.bit_gather(value, mask)
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V4 ISA and OS state.
/// `keys` and `vals` must each point to four initialized, readable i64 values.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave,avx512f,avx512bw,avx512cd,avx512dq,avx512vl"
)]
pub unsafe extern "C" fn x86_v4_masked_sum(
    keys: *const [i64; 4],
    vals: *const [i64; 4],
    limit: i64,
) -> i64 {
    // SAFETY: The caller proves V4 support and validity of both readable arrays.
    unsafe { uarch_arch::X86V4::new_unchecked().masked_sum_lt_i64x4(&*keys, &*vals, limit) }
}

/// # Safety
/// All possible execution CPUs must support the complete x86 V4 ISA and OS state.
/// `keys` must point to four initialized, readable i64 values.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(
    enable = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave,avx512f,avx512bw,avx512cd,avx512dq,avx512vl"
)]
pub unsafe extern "C" fn x86_v4_first_ge(keys: *const [i64; 4], target: i64) -> u32 {
    // SAFETY: The caller proves V4 support and the array's readable validity.
    unsafe { uarch_arch::X86V4::new_unchecked().find_first_ge_i64x4(&*keys, target) }.unwrap_or(4)
}
