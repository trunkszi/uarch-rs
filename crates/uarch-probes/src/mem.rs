//! Audit symbol for masked read prefetch.

use uarch_core::mem::{L1, prefetch_read};

// Monomorphic symbol. The pointer is not dereferenced.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn pool_prefetch_l1(ptr: *const u8) {
    prefetch_read::<L1, u8>(ptr);
}

/// The pool is prepared by the caller, outside the audited hot path.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn pool_get_checked(
    pool: &uarch_core::mem::Pool<'_, u64, 8>,
    index: uarch_core::mem::PoolIndex<u64>,
) -> Option<u64> {
    pool.get(index).copied()
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn pool_get_masked(
    pool: &uarch_core::mem::Pool<'_, u64, 8>,
    index: uarch_core::mem::PoolIndex<u64>,
) -> u64 {
    *pool.get_masked(index)
}
