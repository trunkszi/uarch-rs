use crate::{CpuidSource, cpuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Cost classification for the BMI2 PDEP/PEXT instruction pair.
pub enum PdepCost {
    Fast,
    Slow,
    Unknown,
}

/// Classify PDEP/PEXT cost without issuing an execution capability.
pub fn pdep_cost<S: CpuidSource>(src: &S) -> PdepCost {
    let (family, _) = cpuid::family_model(src.cpuid(1, 0).eax);
    let vendor = cpuid::vendor(src);
    if (vendor == *b"AuthenticAMD" && (family == 0x15 || family == 0x17))
        || (vendor == *b"HygonGenuine" && family == 0x18)
    {
        return PdepCost::Slow;
    }
    if src.cpuid(1, 0).ecx & (1 << 31) != 0 {
        return PdepCost::Unknown;
    }
    if (vendor == *b"GenuineIntel" && family >= 6) || (vendor == *b"AuthenticAMD" && family >= 0x19)
    {
        PdepCost::Fast
    } else {
        PdepCost::Unknown
    }
}

pub fn avoid_wide<S: CpuidSource>(src: &S) -> bool {
    cpuid::vendor(src) == *b"GenuineIntel" && cpuid::family_model(src.cpuid(1, 0).eax) == (6, 0x55)
}
