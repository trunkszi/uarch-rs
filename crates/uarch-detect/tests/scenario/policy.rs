use super::cpuid::{Fake, intel_v4};
use uarch_detect::{DetectPolicy, ExecutionDomainPolicy, WidePolicy, classify_domain};
use uarch_tiers::TierId;

#[test]
fn heterogeneous_domains_intersect_safely() {
    let wide = DetectPolicy {
        wide: WidePolicy::Allow512,
        ..DetectPolicy::default()
    };
    let mut narrow = intel_v4();
    narrow.xcr0 = Some(6);
    assert_eq!(
        classify_domain(&[intel_v4(), narrow.clone()], wide),
        TierId::X86V3
    );
    assert_eq!(
        classify_domain(&[narrow.clone(), intel_v4()], wide),
        TierId::X86V3
    );
    let strict = DetectPolicy {
        domain: ExecutionDomainPolicy::RequireHomogeneous,
        ..wide
    };
    assert_eq!(
        classify_domain(&[intel_v4(), narrow], strict),
        TierId::Scalar
    );
    assert_eq!(
        classify_domain(&[intel_v4(), Fake::default()], wide),
        TierId::Scalar
    );
    assert_eq!(classify_domain::<Fake>(&[], wide), TierId::Scalar);
    assert_eq!(
        classify_domain(&[intel_v4(), intel_v4()], strict),
        TierId::X86V4
    );
}

#[test]
fn domain_keeps_slowest_pdep_cost() {
    let mut virtual_cpu = intel_v4();
    let Some(entry) = virtual_cpu.entries.get_mut(&(1, 0)) else {
        panic!("fixture leaf missing")
    };
    entry.ecx |= 1 << 31;
    let wide = DetectPolicy {
        wide: WidePolicy::Allow512,
        ..DetectPolicy::default()
    };
    assert_eq!(
        classify_domain(&[intel_v4(), virtual_cpu.clone()], wide),
        TierId::X86V3SlowPdep
    );
    assert_eq!(
        classify_domain(&[virtual_cpu, intel_v4()], wide),
        TierId::X86V3SlowPdep
    );
}
