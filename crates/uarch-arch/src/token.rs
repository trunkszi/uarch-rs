use crate::{TierId, kernel};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A capability for every CPU on which the holder can execute.
///
/// Implementations are sealed so safe code cannot invent CPU authority.
///
/// ```compile_fail
/// use uarch_arch::X86V3;
/// let _ = X86V3 { _seal: () };
/// ```
///
/// ```compile_fail
/// let _ = uarch_arch::X86V3::new_unchecked();
/// ```
pub trait Arch: Copy + Send + Sync + 'static + sealed::Sealed {
    const TIER: TierId;
    const NAME: &'static str;
    const FAST_PDEP: bool;
    const VECTOR_BYTES: usize;
    fn bit_gather(self, val: u64, mask: u64) -> u64;
    fn bit_scatter(self, val: u64, mask: u64) -> u64;
    /// Sums qualifying values modulo 2^64.
    fn masked_sum_lt_i64x4(self, keys: &[i64; 4], vals: &[i64; 4], limit: i64) -> i64;
    fn find_first_ge_i64x4(self, keys: &[i64; 4], target: i64) -> Option<u32>;
}

macro_rules! token {
    ($name:ident, $requirement:literal) => {
        #[doc = $requirement]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name {
            _seal: (),
        }
        // SAFETY: The private unit field is zero-sized and has no padding bytes.
        unsafe impl uarch_core::layout::NoPadding for $name {}
        // SAFETY: A sealed zero-sized capability contributes no scalar ABI leaves.
        unsafe impl uarch_core::layout::RegisterLeaf for $name {
            const LEAVES: u8 = 0;
        }
        impl sealed::Sealed for $name {}
        impl $name {
            /// Constructs a capability without detection.
            ///
            /// # Safety
            #[doc = $requirement]
            /// Every CPU that may execute this token, including receiving threads,
            /// must satisfy this requirement for the token's entire lifetime.
            #[inline(always)]
            pub const unsafe fn new_unchecked() -> Self {
                Self { _seal: () }
            }
        }
    };
}

token!(
    Scalar,
    "Requires only the compilation target's baseline ISA."
);
impl Scalar {
    /// Constructs the baseline capability, which needs no extra ISA or OS state.
    #[inline(always)]
    pub const fn new() -> Self {
        Self { _seal: () }
    }
}

impl Default for Scalar {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

token!(
    X86V3,
    "Requires x86_64, every X86_V3_FEATURES feature, and XCR0 SSE/AVX state."
);
token!(
    X86V3SlowPdep,
    "Requires x86_64, every X86_V3_FEATURES feature, and XCR0 SSE/AVX state."
);
token!(
    X86V4,
    "Requires x86_64, every X86_V4_FEATURES feature, and XCR0 SSE/AVX/opmask/ZMM state."
);
token!(
    Aarch64Neon,
    "Requires aarch64 with NEON enabled by the operating system."
);

/// Write intent prefetch capability. This hint is not a memory barrier and
/// does not guarantee cache ownership or elimination of read-for-ownership.
/// Use only for cache lines owned by the calling worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrefetchWrite {
    _seal: (),
}

// SAFETY: The private unit field is zero-sized and contains no storage bytes.
unsafe impl uarch_core::layout::NoPadding for PrefetchWrite {}
// SAFETY: The zero-sized capability contributes no scalar ABI leaves.
unsafe impl uarch_core::layout::RegisterLeaf for PrefetchWrite {
    const LEAVES: u8 = 0;
}

impl PrefetchWrite {
    /// # Safety
    /// Every possible execution CPU must support PREFETCHW (x86_64) or PRFM
    /// (aarch64). The token must not be constructed on any other architecture.
    #[inline(always)]
    pub const unsafe fn new_unchecked() -> Self {
        Self { _seal: () }
    }

    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    pub const fn new() -> Self {
        Self { _seal: () }
    }

    /// Issues a non-faulting microarchitectural hint without accessing Rust memory.
    #[inline(always)]
    #[allow(
        clippy::not_unsafe_ptr_arg_deref,
        reason = "The hint never dereferences Rust memory."
    )]
    pub fn prefetch_write(&self, ptr: *const u8) {
        // SAFETY: Construction of PrefetchWrite proves PREFETCHW (x86_64) or PRFM
        // (aarch64) support on all execution CPUs.
        unsafe { kernel::prefetch_write::prefetch_write(ptr) }
    }
}

/// Prefetch the cache line containing a valid reference with a locality marker.
#[inline(always)]
pub fn prefetch_read_ref<T, L: uarch_core::mem::Locality>(reference: &T) {
    uarch_core::mem::prefetch_read::<L, T>(reference as *const T);
}

/// Prefetch a privately owned line for writing; this does not modify Rust memory.
/// x86 PREFETCHW has no locality operand. AArch64 selects the requested PRFM hint.
#[inline(always)]
pub fn prefetch_write_ref<T, L: uarch_core::mem::Locality>(token: PrefetchWrite, reference: &T) {
    #[cfg(all(not(miri), target_arch = "aarch64"))]
    {
        let pointer = reference as *const T;
        // SAFETY: The reference is live and the token proves the instruction's
        // execution-domain contract. PRFM performs no Rust memory access.
        unsafe {
            match L::X86_HINT {
                0 => {
                    core::arch::asm!("prfm pstl1strm, [{pointer}]", pointer = in(reg) pointer, options(nostack, preserves_flags, readonly))
                }
                1 => {
                    core::arch::asm!("prfm pstl3keep, [{pointer}]", pointer = in(reg) pointer, options(nostack, preserves_flags, readonly))
                }
                2 => {
                    core::arch::asm!("prfm pstl2keep, [{pointer}]", pointer = in(reg) pointer, options(nostack, preserves_flags, readonly))
                }
                _ => {
                    core::arch::asm!("prfm pstl1keep, [{pointer}]", pointer = in(reg) pointer, options(nostack, preserves_flags, readonly))
                }
            }
        }
        let _ = token;
    }
    #[cfg(any(miri, not(target_arch = "aarch64")))]
    {
        token.prefetch_write((reference as *const T).cast());
    }
}

#[cfg(target_arch = "aarch64")]
impl Default for PrefetchWrite {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
