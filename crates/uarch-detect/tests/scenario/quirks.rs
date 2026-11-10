use super::cpuid::{Fake, intel_v4};
use uarch_detect::{DetectPolicy, UnknownPdep, WidePolicy, classify};
use uarch_tiers::TierId;

fn identify(src: &mut Fake, vendor: &[u8; 12], signature: u32) {
    let Some(id) = src.entries.get_mut(&(0, 0)) else {
        panic!("fixture leaf missing")
    };
    id.ebx = u32::from_le_bytes([vendor[0], vendor[1], vendor[2], vendor[3]]);
    id.edx = u32::from_le_bytes([vendor[4], vendor[5], vendor[6], vendor[7]]);
    id.ecx = u32::from_le_bytes([vendor[8], vendor[9], vendor[10], vendor[11]]);
    let Some(id) = src.entries.get_mut(&(1, 0)) else {
        panic!("fixture leaf missing")
    };
    id.eax = signature;
}

#[test]
fn slow_microcode_stays_software() {
    let aggressive = DetectPolicy {
        wide: WidePolicy::Allow512,
        unknown_pdep: UnknownPdep::AssumeFast,
        ..DetectPolicy::default()
    };
    for (vendor, signature) in [
        (b"AuthenticAMD", 0x0080_0f10),
        (b"AuthenticAMD", 0x0060_0f01),
        (b"HygonGenuine", 0x0090_0f10),
    ] {
        let mut src = intel_v4();
        identify(&mut src, vendor, signature);
        assert_eq!(classify(&src, aggressive), TierId::X86V3SlowPdep);
    }
}

#[test]
fn downclock_model_rejects_wide() {
    let wide = DetectPolicy {
        wide: WidePolicy::Allow512,
        ..DetectPolicy::default()
    };
    let mut src = intel_v4();
    identify(&mut src, b"GenuineIntel", 0x0005_0650);
    assert_eq!(classify(&src, wide), TierId::X86V3);
    identify(&mut src, b"AuthenticAMD", 0x00a0_0f10);
    assert_eq!(classify(&src, wide), TierId::X86V4);
}

#[test]
fn unknown_cost_is_conservative() {
    let mut src = intel_v4();
    identify(&mut src, b"UnknownCPU!!", 0);
    assert_eq!(
        classify(&src, DetectPolicy::default()),
        TierId::X86V3SlowPdep
    );
    let override_policy = DetectPolicy {
        unknown_pdep: UnknownPdep::AssumeFast,
        ..DetectPolicy::default()
    };
    assert_eq!(classify(&src, override_policy), TierId::X86V3);
}

#[test]
fn later_intel_families_are_fast() {
    let mut src = intel_v4();
    for (signature, family) in [(0x0000_0f10, 0x0f), (0x0040_0f00, 0x13)] {
        identify(&mut src, b"GenuineIntel", signature);
        assert_eq!(uarch_detect::cpuid::family_model(signature).0, family);
        assert_eq!(
            uarch_detect::quirks::pdep_cost(&src),
            uarch_detect::quirks::PdepCost::Fast
        );
        assert_eq!(classify(&src, DetectPolicy::default()), TierId::X86V3);
    }
}
