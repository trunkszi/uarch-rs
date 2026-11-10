//! `use uarch::prelude::*;`
//!
//! Brings the day-to-day layout, bit-word, predication, and attribute-macro
//! names into scope. Host detect stays behind `crate::detect` so a `no_std`
//! build does not pull the probe.

pub use crate::arch::{
    Aarch64Neon, Arch, PrefetchWrite, Scalar, Tier, X86V3, X86V3SlowPdep, X86V4,
};
pub use crate::arithmetic::{FastDivisorU32, FastDivisorU64, PowerOfTwo};
pub use crate::bits::{BitField, BitFlag, BitPatch, BitPattern, Bitmap64, FlagClear, FlagSet};
pub use crate::flow::{
    MaskWord, Predicate, RegisterSelectable, likely, predicate, reduce_lanes, unlikely,
};
pub use crate::layout::{CacheAligned, Isolated, NoPadding, RegisterLeaf, assert_layout};
pub use crate::mem::{L1, L2, L3, NonTemporal, Pool, PoolIndex, prefetch_read};
pub use crate::tiers::TierId;
pub use crate::{
    bitword, cache_line, cold_path, cold_sink, hot_inline, ilp_all, ilp_any, isolated,
    multiversion, patch, pattern, reduce_four_way, register_pass,
};

#[cfg(feature = "detect")]
pub use crate::detect::{DetectError, DetectPolicy, detect};

pub use crate::hash::{fnv1a64, fnv1a64_extend};
pub use crate::text::FixedAscii;
