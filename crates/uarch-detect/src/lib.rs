//! Host observation and conservative capability issuance.
//!
//! Tokens describe the audited execution domain. External code must not expand
//! affinity, migrate receiving threads outside that domain, change ISA exposure,
//! or remove OS extended-register support while capabilities remain live.

pub mod cpuid;
pub mod error;
pub mod host;
pub mod hwcap;
pub mod policy;
pub mod quirks;

pub use cpuid::{CpuidRegisters, CpuidSource, HardwareCpuid};
pub use error::{AuditError, DetectError};
pub use host::{CacheLine, HostHardwareAudit, HostReport, Observed};
pub use policy::{
    DetectPolicy, ExecutionDomainPolicy, UnknownPdep, WidePolicy, classify, classify_domain,
};

use uarch_arch::{Aarch64Neon, PrefetchWrite, Scalar, Tier, X86V3, X86V3SlowPdep, X86V4};
use uarch_tiers::TierId;

/// Audits all allowed CPUs before issuing a transferable capability.
///
/// The caller must preserve the execution-domain environment contract described
/// in the crate documentation. No affinity or other system state is changed.
///
/// # Deployment & Thread Pinning
/// On Linux with a multi-CPU affinity mask, this detector needs read access to
/// every `/dev/cpu/N/cpuid` in that mask to prove per-core ISA support. Device
/// access is often restricted to root or privileged deployments; file permissions
/// still apply, including in environments granted `CAP_SYS_RAWIO`.
/// Without that evidence, [`ExecutionDomainPolicy::CommonSubset`] returns
/// `Ok(Tier::Scalar(Scalar::new()))`; [`ExecutionDomainPolicy::RequireHomogeneous`]
/// returns `Err(DetectError::UnverifiedExecutionDomain)`.
///
/// Production low-latency workers should pin their thread to one core with
/// `sched_setaffinity` or `taskset -c <cpu>` **before** calling this function.
/// A singleton mask enables direct CPUID + XGETBV verification without opening
/// `/dev/cpu/N/cpuid`. Receiving threads must remain in the audited domain.
///
/// With [`UnknownPdep::Conservative`] (the default), otherwise V3-capable
/// hypervisor guests (`CPUID.1:ECX[31] == 1`) select `X86V3SlowPdep`.
/// [`UnknownPdep::AssumeFast`] explicitly treats unknown PEXT cost as fast;
/// recognized slow-microcode CPUs still select the software path.
///
/// # Errors
/// Returns `UnverifiedExecutionDomain` when strict verification fails.
pub fn detect(policy: DetectPolicy) -> Result<Tier, DetectError> {
    #[cfg(target_arch = "x86_64")]
    let id = {
        match host::execution_domain() {
            Some(sources) => {
                let first = sources
                    .first()
                    .ok_or(DetectError::UnverifiedExecutionDomain)?;
                if policy.domain == ExecutionDomainPolicy::RequireHomogeneous
                    && !sources.iter().all(|src| cpuid::same_isa(first, src))
                {
                    return Err(DetectError::UnverifiedExecutionDomain);
                }
                classify_domain(&sources, policy)
            }
            None if policy.domain == ExecutionDomainPolicy::CommonSubset => TierId::Scalar,
            None => return Err(DetectError::UnverifiedExecutionDomain),
        }
    };
    #[cfg(target_arch = "aarch64")]
    let id = if hwcap::neon_supported() {
        TierId::Aarch64Neon
    } else if policy.domain == ExecutionDomainPolicy::RequireHomogeneous {
        return Err(DetectError::UnverifiedExecutionDomain);
    } else {
        TierId::Scalar
    };
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let id = {
        if policy.domain == ExecutionDomainPolicy::RequireHomogeneous {
            return Err(DetectError::UnverifiedExecutionDomain);
        }
        TierId::Scalar
    };

    // SAFETY: Only real, complete execution-domain evidence reaches a non-scalar
    // arm; Scalar uses the compilation target's universal baseline.
    Ok(unsafe {
        match id {
            TierId::Scalar => Tier::Scalar(Scalar::new()),
            TierId::X86V3 => Tier::X86V3(X86V3::new_unchecked()),
            TierId::X86V3SlowPdep => Tier::X86V3SlowPdep(X86V3SlowPdep::new_unchecked()),
            TierId::X86V4 => Tier::X86V4(X86V4::new_unchecked()),
            TierId::Aarch64Neon => Tier::Aarch64Neon(Aarch64Neon::new_unchecked()),
        }
    })
}

/// Issues a [`PrefetchWrite`] capability after checking the execution domain.
/// On x86_64, verifies PREFETCHW (`CPUID 0x8000_0001:ECX[8]`) on every execution CPU.
/// The same lifetime and affinity contract as [`detect`] applies.
///
/// # Deployment & Thread Pinning
/// On x86_64 Linux, a multi-CPU mask needs read access to each
/// `/dev/cpu/N/cpuid`, subject to the permissions described in [`detect`].
/// Pin the worker thread using `sched_setaffinity` or `taskset -c <cpu>` before
/// detection to use direct singleton CPU verification without these device files.
/// Missing domain evidence returns `Err(DetectError::UnverifiedExecutionDomain)`;
/// this function has no Scalar fallback. The audited domain must remain valid
/// for receiving threads too. PEXT cost policy affects [`detect`]'s tier choice,
/// not this independent write-prefetch capability.
///
/// # Errors
/// Returns `UnverifiedExecutionDomain` if any allowed CPU cannot be verified,
/// or `Unsupported` if at least one CPU lacks write-prefetch support.
pub fn detect_prefetch_write() -> Result<PrefetchWrite, DetectError> {
    #[cfg(target_arch = "x86_64")]
    {
        let sources = host::execution_domain().ok_or(DetectError::UnverifiedExecutionDomain)?;
        if sources.is_empty() {
            return Err(DetectError::UnverifiedExecutionDomain);
        }
        if !sources.iter().all(cpuid::prefetch_write_supported) {
            return Err(DetectError::Unsupported);
        }
        // SAFETY: Each CPU in the real execution-domain snapshot reports
        // PREFETCHW (CPUID 0x8000_0001:ECX[8]).
        Ok(unsafe { PrefetchWrite::new_unchecked() })
    }
    #[cfg(target_arch = "aarch64")]
    {
        Ok(PrefetchWrite::new())
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        Err(DetectError::Unsupported)
    }
}
