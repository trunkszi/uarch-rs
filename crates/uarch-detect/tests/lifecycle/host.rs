use uarch_arch::{Arch, ArchVisitor, Scalar, Tier, TierId, X86V3, X86V3SlowPdep};
#[cfg(target_arch = "x86_64")]
use uarch_detect::ExecutionDomainPolicy;
use uarch_detect::{
    DetectError, DetectPolicy, HostHardwareAudit, Observed, WidePolicy, detect,
    detect_prefetch_write,
};

struct Check;
impl ArchVisitor for Check {
    type Output = TierId;
    fn visit<A: Arch>(self, arch: A) -> TierId {
        for value in [0, 1, u64::MAX, 0xdead_beef_feed_face] {
            for mask in [0, 1 << 63, 0x5555_5555_5555_5555, u64::MAX] {
                assert_eq!(
                    arch.bit_scatter(arch.bit_gather(value, mask), mask),
                    value & mask
                );
                let scalar = Scalar::new();
                assert_eq!(arch.bit_gather(value, mask), scalar.bit_gather(value, mask));
                assert_eq!(
                    arch.bit_scatter(value, mask),
                    scalar.bit_scatter(value, mask)
                );
            }
        }
        let keys = [i64::MIN, -1, 0, i64::MAX];
        assert_eq!(
            arch.masked_sum_lt_i64x4(&keys, &[i64::MAX, 1, 9, 37], 0),
            i64::MIN
        );
        assert_eq!(arch.find_first_ge_i64x4(&keys, i64::MAX), Some(3));
        assert_eq!(arch.find_first_ge_i64x4(&[7, 2, 9, 1], 8), Some(2));
        assert_eq!(arch.find_first_ge_i64x4(&[7, 2, 9, 1], 10), None);
        for lane in 0..4 {
            let mut keys = [i64::MIN; 4];
            keys[lane] = i64::MAX;
            assert_eq!(arch.find_first_ge_i64x4(&keys, 0), Some(lane as u32));
        }
        A::TIER
    }
}

#[test]
fn real_capabilities_execute_kernels() -> Result<(), DetectError> {
    let policy = DetectPolicy {
        wide: WidePolicy::Allow512,
        ..DetectPolicy::default()
    };
    let tier = detect(policy)?;
    let id = tier.dispatch(Check);
    match tier {
        Tier::X86V3(_) | Tier::X86V3SlowPdep(_) | Tier::X86V4(_) => {
            // SAFETY: Real detect proved the full V3 ISA and OS state on every
            // execution CPU. Both cost variants require exactly that same ISA.
            let (fast, slow) = unsafe { (X86V3::new_unchecked(), X86V3SlowPdep::new_unchecked()) };
            Check.visit(fast);
            Check.visit(slow);
        }
        Tier::Scalar(_) | Tier::Aarch64Neon(_) => {}
    }
    #[cfg(target_arch = "x86_64")]
    if let Observed::Known(cpus) = HostHardwareAudit::probe().affinity_cpus
        && cpus.len() == 1
    {
        #[cfg(target_arch = "x86_64")]
        assert_eq!(
            id,
            uarch_detect::classify(&uarch_detect::HardwareCpuid, policy)
        );
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = id;
    Ok(())
}

#[test]
fn audit_preserves_thread_affinity() {
    #[cfg(target_os = "linux")]
    let before = std::fs::read_to_string("/proc/thread-self/status").ok();
    let report = HostHardwareAudit::probe();
    #[cfg(target_arch = "x86_64")]
    let strict = DetectPolicy {
        domain: ExecutionDomainPolicy::RequireHomogeneous,
        ..DetectPolicy::default()
    };
    #[cfg(target_arch = "x86_64")]
    match report.isa_homogeneous {
        Observed::Known(true) => assert!(detect(strict).is_ok()),
        Observed::Known(false) | Observed::Unknown | Observed::Unsupported => {
            assert!(matches!(
                detect(strict),
                Err(DetectError::UnverifiedExecutionDomain)
            ));
        }
    }
    if let Observed::Known(lines) = report.cache_lines {
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| line.level > 0 && line.bytes > 0));
    }
    if let Ok(write_hint) = detect_prefetch_write() {
        let owned = [0u8; 128];
        write_hint.prefetch_write(owned.as_ptr());
    }
    #[cfg(target_os = "linux")]
    {
        let after = std::fs::read_to_string("/proc/thread-self/status").ok();
        let mask = |text: &Option<String>| -> Option<String> {
            text.as_deref()?
                .lines()
                .find(|line| line.starts_with("Cpus_allowed_list:"))
                .map(str::to_owned)
        };
        assert_eq!(mask(&before), mask(&after));
    }
}
