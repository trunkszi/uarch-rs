macro_rules! implement_arch {
    ($ty:ident, $fast:expr, $bytes:expr, $gather:path, $scatter:path, $sum:path, $find:path) => {
        impl crate::Arch for crate::$ty {
            const TIER: crate::TierId = crate::TierId::$ty;
            const NAME: &'static str = stringify!($ty);
            const FAST_PDEP: bool = $fast;
            const VECTOR_BYTES: usize = $bytes;
            #[inline(always)]
            fn bit_gather(self, val: u64, mask: u64) -> u64 {
                // SAFETY: The unforgeable token proves the kernel's ISA preconditions.
                unsafe { $gather(val, mask) }
            }
            #[inline(always)]
            fn bit_scatter(self, val: u64, mask: u64) -> u64 {
                // SAFETY: The unforgeable token proves the kernel's ISA preconditions.
                unsafe { $scatter(val, mask) }
            }
            #[inline(always)]
            fn masked_sum_lt_i64x4(self, keys: &[i64; 4], vals: &[i64; 4], limit: i64) -> i64 {
                // SAFETY: The unforgeable token proves the kernel's ISA preconditions.
                unsafe { $sum(keys, vals, limit) }
            }
            #[inline(always)]
            fn find_first_ge_i64x4(self, keys: &[i64; 4], target: i64) -> Option<u32> {
                // SAFETY: The unforgeable token proves the kernel's ISA preconditions.
                unsafe { $find(keys, target) }
            }
        }
    };
}
mod aarch64_neon;
mod scalar;
mod x86_v3;
mod x86_v3_slow_pdep;
mod x86_v4;
