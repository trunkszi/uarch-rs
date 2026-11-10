#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
//! Facade for the `uarch` microarchitecture toolkit.
//!
//! Default features enable `std` and host `detect`.
//! `--no-default-features` keeps the crate `no_std` and still exposes layout,
//! bits, flow, arithmetic, pools, and arch tokens.
//!
//! Agents that emit hot code follow [`guide`].
//!
//! ```
//! use uarch::prelude::*;
//!
//! fn accept(ready: bool, value: u64) -> u64 {
//!     if !ready {
//!         cold_path();
//!         return 0;
//!     }
//!     value
//! }
//!
//! let kept = accept(true, 9);
//! assert_eq!(kept, 9);
//! assert_eq!(predicate(false).select(1u64, 2), 2);
//! ```

#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

pub mod guide;
pub mod prelude;

pub use uarch_arch as arch;
pub use uarch_arch::{
    Aarch64Neon, Arch, ArchVisitor, PrefetchWrite, Scalar, Tier, TierId, X86V3, X86V3SlowPdep,
    X86V4,
};
pub use uarch_core::{arithmetic, bits, flow, hash, layout, mem, text};
pub use uarch_tiers as tiers;

pub use uarch_core::flow::{cold_path, predicate, reduce_lanes};
pub use uarch_core::{bitword, ilp_all, ilp_any, reduce_four_way};
pub use uarch_macros::{
    cache_line, cold_sink, hot_inline, isolated, multiversion, patch, pattern, register_pass,
};

#[cfg(feature = "detect")]
pub use uarch_detect as detect;
