//! Fixed power-of-two table. Physical slot 0 is the sentinel and stays initialized.

use super::prefetch::{Locality, prefetch_read};
use super::{PoolError, PoolIndex};

/// Borrowed table of `1 << LOG2` initialized slots.
///
/// Slot `0` is reserved. [`Pool::get`] refuses indexes outside the table.
/// [`Pool::get_masked`] folds the index with the capacity mask, which can land
/// on the sentinel.
pub struct Pool<'s, T, const LOG2: u32> {
    base: &'s mut [T],
}

impl<'s, T, const LOG2: u32> Pool<'s, T, LOG2> {
    /// Borrow `storage` when its length is exactly `1 << LOG2`.
    ///
    /// # Errors
    ///
    /// Returns [`PoolError::Capacity`] when `storage.len()` is not `1 << LOG2`.
    ///
    /// # Panics
    ///
    /// Fails at compile time when `LOG2` is outside `1..=31`.
    #[inline]
    pub fn new(storage: &'s mut [T]) -> Result<Self, PoolError> {
        const {
            assert!(LOG2 >= 1 && LOG2 <= 31);
        }
        let cap = 1usize << LOG2;
        let len = storage.len();
        if len != cap {
            return Err(PoolError::Capacity { len, cap });
        }
        Ok(Self { base: storage })
    }

    /// Slot `index` when it lies inside this table.
    #[inline(always)]
    pub fn get(&self, index: PoolIndex<T>) -> Option<&T> {
        let slot = index.slot() as usize;
        if slot < (1usize << LOG2) {
            // SAFETY: Pool::new proves base.len() == 1 << LOG2, and the branch
            // proves slot < 1 << LOG2. The shared borrow covers the returned reference.
            Some(unsafe { self.base.get_unchecked(slot) })
        } else {
            None
        }
    }

    /// Mutable slot `index` when it lies inside this table.
    #[inline(always)]
    pub fn get_mut(&mut self, index: PoolIndex<T>) -> Option<&mut T> {
        let slot = index.slot() as usize;
        if slot < (1usize << LOG2) {
            // SAFETY: Pool::new proves base.len() == 1 << LOG2, and the branch
            // proves slot < 1 << LOG2. The mutable borrow grants exclusive access.
            Some(unsafe { self.base.get_unchecked_mut(slot) })
        } else {
            None
        }
    }

    /// Slot `index.slot() & (cap - 1)`, including the sentinel when the mask is 0.
    #[inline(always)]
    pub fn get_masked(&self, index: PoolIndex<T>) -> &T {
        let slot = (index.slot() as usize) & ((1usize << LOG2) - 1);
        // SAFETY: `new` requires `base.len() == 1 << LOG2`, and `slot` is that
        // length minus one, masked. The shared borrow of `self` owns the table.
        unsafe { self.base.get_unchecked(slot) }
    }

    /// Mutable form of [`Self::get_masked`].
    #[inline(always)]
    pub fn get_masked_mut(&mut self, index: PoolIndex<T>) -> &mut T {
        let slot = (index.slot() as usize) & ((1usize << LOG2) - 1);
        // SAFETY: same index proof as `get_masked`. `&mut self` is the unique
        // borrow of the table for this call.
        unsafe { self.base.get_unchecked_mut(slot) }
    }

    /// Prefetch the masked slot. The address is formed with `wrapping_add`.
    #[inline(always)]
    pub fn prefetch_read<L: Locality>(&self, index: PoolIndex<T>) {
        let slot = (index.slot() as usize) & ((1usize << LOG2) - 1);
        let ptr = self.base.as_ptr().wrapping_add(slot);
        prefetch_read::<L, T>(ptr);
    }
}

impl<T: core::fmt::Debug, const LOG2: u32> core::fmt::Debug for Pool<'_, T, LOG2> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Pool")
            .field("log2", &LOG2)
            .field("base", &self.base)
            .finish()
    }
}
