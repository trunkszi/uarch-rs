//! Sealed execution capabilities and statically dispatched hardware kernels.
#![no_std]

mod dispatch;
mod kernel;
mod tier;
mod token;

pub use dispatch::{ArchVisitor, Tier, retag_unchecked};
pub use token::{Aarch64Neon, Arch, PrefetchWrite, Scalar, X86V3, X86V3SlowPdep, X86V4};
pub use uarch_tiers::TierId;

#[doc(hidden)]
pub mod __private {
    pub use crate::{
        Aarch64Neon, Arch, Scalar, TierId, X86V3, X86V3SlowPdep, X86V4, retag_unchecked,
    };
    pub use uarch_core::__private::*;
}

pub use token::{prefetch_read_ref, prefetch_write_ref};
