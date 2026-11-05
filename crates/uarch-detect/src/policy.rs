//! Detection choices for execution-domain and instruction-cost evidence.
//!
//! # Deployment & Thread Pinning
//! Pin production worker threads to one CPU before [`crate::detect`] to avoid
//! needing read access to Linux `/dev/cpu/N/cpuid` across a multi-CPU mask.
//! Without that evidence, [`ExecutionDomainPolicy::CommonSubset`] permits only
//! Scalar; [`ExecutionDomainPolicy::RequireHomogeneous`] returns an error.
//! Otherwise V3-capable hypervisor guests (`CPUID.1:ECX[31]`) select
//! `X86V3SlowPdep` with default [`UnknownPdep::Conservative`]. Opting into
//! [`UnknownPdep::AssumeFast`] overrides unknown cost, while recognized slow
//! microcode remains on the software path. See [`crate::detect`] for deployment
//! permissions and the lifetime contract for receiving threads.

use crate::{CpuidSource, cpuid, quirks};
use uarch_tiers::TierId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WidePolicy {
    #[default]
    Max256,
    Allow512,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UnknownPdep {
    #[default]
    Conservative,
    AssumeFast,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExecutionDomainPolicy {
    /// Use the proved intersection; an unknown domain permits only Scalar.
    #[default]
    CommonSubset,
    /// Reject unknown or heterogeneous ISA domains when issuing capabilities.
    RequireHomogeneous,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DetectPolicy {
    pub wide: WidePolicy,
    pub unknown_pdep: UnknownPdep,
    pub domain: ExecutionDomainPolicy,
}

/// Pure classification; fake sources can never issue execution capabilities.
///
/// ```compile_fail
/// use uarch_detect::{classify, DetectPolicy, HardwareCpuid};
/// let _: uarch_arch::Tier = classify(&HardwareCpuid, DetectPolicy::default());
/// ```
pub fn classify<S: CpuidSource>(src: &S, policy: DetectPolicy) -> TierId {
    if !cpuid::v3_supported(src) {
        return TierId::Scalar;
    }
    let cost = quirks::pdep_cost(src);
    let slow = cost == quirks::PdepCost::Slow
        || (cost == quirks::PdepCost::Unknown && policy.unknown_pdep == UnknownPdep::Conservative);
    if slow {
        return TierId::X86V3SlowPdep;
    }
    if policy.wide == WidePolicy::Allow512 && !quirks::avoid_wide(src) && cpuid::v4_supported(src) {
        TierId::X86V4
    } else {
        TierId::X86V3
    }
}

/// Returns the common executable tier of a complete domain snapshot.
/// An empty, mixed-target or strictly heterogeneous snapshot classifies Scalar.
/// This result carries no hardware authority and has no safe token conversion.
pub fn classify_domain<S: CpuidSource>(sources: &[S], policy: DetectPolicy) -> TierId {
    let Some(first) = sources.first() else {
        return TierId::Scalar;
    };
    if policy.domain == ExecutionDomainPolicy::RequireHomogeneous
        && !sources.iter().all(|src| cpuid::same_isa(first, src))
    {
        return TierId::Scalar;
    }
    sources
        .iter()
        .skip(1)
        .fold(classify(first, policy), |common, src| {
            intersect(common, classify(src, policy))
        })
}

fn intersect(a: TierId, b: TierId) -> TierId {
    use TierId::*;
    match (a, b) {
        (Scalar, _) | (_, Scalar) => Scalar,
        (Aarch64Neon, Aarch64Neon) => Aarch64Neon,
        (Aarch64Neon, _) | (_, Aarch64Neon) => Scalar,
        (X86V3SlowPdep, _) | (_, X86V3SlowPdep) => X86V3SlowPdep,
        (X86V3, _) | (_, X86V3) => X86V3,
        (X86V4, X86V4) => X86V4,
    }
}
