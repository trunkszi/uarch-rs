use crate::{
    Arch, Scalar, TierId,
    kernel::{bit_gather, masked_scan},
};

impl Arch for Scalar {
    const TIER: TierId = TierId::Scalar;
    const NAME: &'static str = "Scalar";
    const FAST_PDEP: bool = false;
    const VECTOR_BYTES: usize = 0;
    #[inline(always)]
    fn bit_gather(self, val: u64, mask: u64) -> u64 {
        bit_gather::gather(val, mask)
    }
    #[inline(always)]
    fn bit_scatter(self, val: u64, mask: u64) -> u64 {
        bit_gather::scatter(val, mask)
    }
    #[inline(always)]
    fn masked_sum_lt_i64x4(self, keys: &[i64; 4], vals: &[i64; 4], limit: i64) -> i64 {
        masked_scan::sum(keys, vals, limit)
    }
    #[inline(always)]
    fn find_first_ge_i64x4(self, keys: &[i64; 4], target: i64) -> Option<u32> {
        masked_scan::find(keys, target)
    }
}
