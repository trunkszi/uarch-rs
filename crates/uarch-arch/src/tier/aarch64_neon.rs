#[cfg(target_arch = "aarch64")]
use crate::kernel::{bit_gather, masked_scan};
#[cfg(target_arch = "aarch64")]
implement_arch!(
    Aarch64Neon,
    false,
    16,
    gather_neon,
    scatter_neon,
    masked_scan::sum_neon,
    masked_scan::find_neon
);
#[cfg(target_arch = "aarch64")]
unsafe fn gather_neon(val: u64, mask: u64) -> u64 {
    bit_gather::gather(val, mask)
}
#[cfg(target_arch = "aarch64")]
unsafe fn scatter_neon(val: u64, mask: u64) -> u64 {
    bit_gather::scatter(val, mask)
}
#[cfg(not(target_arch = "aarch64"))]
implement_arch!(
    Aarch64Neon,
    false,
    16,
    gather_unavailable,
    scatter_unavailable,
    sum_unavailable,
    find_unavailable
);
#[cfg(not(target_arch = "aarch64"))]
unsafe fn gather_unavailable(_: u64, _: u64) -> u64 {
    unreachable!("tier unavailable on this target")
}
#[cfg(not(target_arch = "aarch64"))]
unsafe fn scatter_unavailable(_: u64, _: u64) -> u64 {
    unreachable!("tier unavailable on this target")
}
#[cfg(not(target_arch = "aarch64"))]
unsafe fn sum_unavailable(_: &[i64; 4], _: &[i64; 4], _: i64) -> i64 {
    unreachable!("tier unavailable on this target")
}
#[cfg(not(target_arch = "aarch64"))]
unsafe fn find_unavailable(_: &[i64; 4], _: i64) -> Option<u32> {
    unreachable!("tier unavailable on this target")
}
