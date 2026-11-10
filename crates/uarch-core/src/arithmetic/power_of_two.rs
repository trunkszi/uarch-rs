/// Compile-time power-of-two value, masks, division and alignment arithmetic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PowerOfTwo<const LOG2: u32>;

impl<const LOG2: u32> PowerOfTwo<LOG2> {
    /// Power of two as a native-width value.
    pub const VALUE: usize = {
        assert!(LOG2 < usize::BITS);
        1usize << LOG2
    };
    /// Power of two as a 64-bit value.
    pub const VALUE_U64: u64 = {
        assert!(LOG2 < u64::BITS);
        1u64 << LOG2
    };
    /// Number of low bits occupied by the remainder.
    pub const SHIFT: u32 = {
        assert!(LOG2 < u64::BITS);
        LOG2
    };
    /// Low LOG2 bits set in a 64-bit word.
    pub const MASK_U64: u64 = Self::VALUE_U64 - 1;
    /// Low LOG2 bits set in a native-width word.
    pub const MASK_USIZE: usize = Self::VALUE - 1;
    /// Wrap an index modulo the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn wrap_u64(value: u64) -> u64 {
        Self::rem_u64(value)
    }
    /// Divide by the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn div_u64(value: u64) -> u64 {
        {
            let _ = Self::MASK_U64;
            value >> Self::SHIFT
        }
    }
    /// Return the remainder modulo the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn rem_u64(value: u64) -> u64 {
        value & Self::MASK_U64
    }
    /// Quotient and remainder modulo the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn div_rem_u64(value: u64) -> (u64, u64) {
        (Self::div_u64(value), Self::rem_u64(value))
    }
    /// Whether the value is an exact multiple of the boundary.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn is_multiple_of_u64(value: u64) -> bool {
        Self::rem_u64(value) == 0
    }
    /// Round down to the nearest aligned boundary.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn align_down_u64(value: u64) -> u64 {
        value & !Self::MASK_U64
    }
    /// Bytes to the next aligned boundary, or zero when already aligned.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn alignment_offset_u64(value: u64) -> u64 {
        value.wrapping_neg() & Self::MASK_U64
    }
    /// Round up to the boundary, returning None on overflow.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn checked_align_up_u64(value: u64) -> Option<u64> {
        match value.checked_add(Self::MASK_U64) {
            Some(sum) => Some(Self::align_down_u64(sum)),
            None => None,
        }
    }
    /// Wrap an index modulo the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn wrap_usize(value: usize) -> usize {
        Self::rem_usize(value)
    }
    /// Divide by the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn div_usize(value: usize) -> usize {
        {
            let _ = Self::MASK_USIZE;
            value >> Self::SHIFT
        }
    }
    /// Return the remainder modulo the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn rem_usize(value: usize) -> usize {
        value & Self::MASK_USIZE
    }
    /// Quotient and remainder modulo the power-of-two value.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn div_rem_usize(value: usize) -> (usize, usize) {
        (Self::div_usize(value), Self::rem_usize(value))
    }
    /// Whether the value is an exact multiple of the boundary.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn is_multiple_of_usize(value: usize) -> bool {
        Self::rem_usize(value) == 0
    }
    /// Round down to the nearest aligned boundary.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn align_down_usize(value: usize) -> usize {
        value & !Self::MASK_USIZE
    }
    /// Bytes to the next aligned boundary, or zero when already aligned.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn alignment_offset_usize(value: usize) -> usize {
        value.wrapping_neg() & Self::MASK_USIZE
    }
    /// Round up to the boundary, returning None on overflow.
    ///
    /// # Panics
    /// Fails at compile time if LOG2 is not smaller than the operand width.
    #[inline(always)]
    pub const fn checked_align_up_usize(value: usize) -> Option<usize> {
        match value.checked_add(Self::MASK_USIZE) {
            Some(sum) => Some(Self::align_down_usize(sum)),
            None => None,
        }
    }
}

// SAFETY: The marker is zero-sized and contains no storage bytes or padding.
unsafe impl<const LOG2: u32> crate::layout::NoPadding for PowerOfTwo<LOG2> {}
// SAFETY: The zero-sized marker contributes zero scalar leaves to an aggregate.
unsafe impl<const LOG2: u32> crate::layout::RegisterLeaf for PowerOfTwo<LOG2> {
    const LEAVES: u8 = 0;
}
