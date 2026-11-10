#[cfg(target_arch = "x86_64")]
use crate::kernel::{bit_gather, masked_scan};
#[cfg(target_arch = "x86_64")]
implement_arch!(
    X86V3,
    true,
    32,
    bit_gather::gather_fast,
    bit_gather::scatter_fast,
    masked_scan::sum_avx2,
    masked_scan::find_avx2
);
#[cfg(not(target_arch = "x86_64"))]
implement_arch!(
    X86V3,
    true,
    32,
    gather_unavailable,
    scatter_unavailable,
    sum_unavailable,
    find_unavailable
);
#[cfg(not(target_arch = "x86_64"))]
unsafe fn gather_unavailable(_: u64, _: u64) -> u64 {
    unreachable!("tier unavailable on this target")
}
#[cfg(not(target_arch = "x86_64"))]
unsafe fn scatter_unavailable(_: u64, _: u64) -> u64 {
    unreachable!("tier unavailable on this target")
}
#[cfg(not(target_arch = "x86_64"))]
unsafe fn sum_unavailable(_: &[i64; 4], _: &[i64; 4], _: i64) -> i64 {
    unreachable!("tier unavailable on this target")
}
#[cfg(not(target_arch = "x86_64"))]
unsafe fn find_unavailable(_: &[i64; 4], _: i64) -> Option<u32> {
    unreachable!("tier unavailable on this target")
}
