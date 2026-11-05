use uarch_detect::{CpuidRegisters, CpuidSource, DetectPolicy, classify};
use uarch_tiers::TierId;

#[derive(Clone, Default)]
pub(super) struct Fake {
    pub(super) entries: std::collections::BTreeMap<(u32, u32), CpuidRegisters>,
    pub(super) xcr0: Option<u64>,
}
impl CpuidSource for Fake {
    fn cpuid(&self, leaf: u32, subleaf: u32) -> CpuidRegisters {
        self.entries
            .get(&(leaf, subleaf))
            .copied()
            .unwrap_or_default()
    }
    fn xcr0(&self) -> Option<u64> {
        self.xcr0
    }
}
pub(super) fn intel_v4() -> Fake {
    let mut src = Fake {
        xcr0: Some(0xe6),
        ..Fake::default()
    };
    for (leaf, value) in [
        (
            0,
            CpuidRegisters {
                eax: 7,
                ebx: u32::from_le_bytes(*b"Genu"),
                edx: u32::from_le_bytes(*b"ineI"),
                ecx: u32::from_le_bytes(*b"ntel"),
            },
        ),
        (
            1,
            CpuidRegisters {
                eax: 0x0009_06e0,
                ecx: (1 << 0)
                    | (1 << 9)
                    | (1 << 12)
                    | (1 << 13)
                    | (1 << 19)
                    | (1 << 20)
                    | (1 << 22)
                    | (1 << 23)
                    | (1 << 26)
                    | (1 << 27)
                    | (1 << 28)
                    | (1 << 29),
                ..CpuidRegisters::default()
            },
        ),
        (
            7,
            CpuidRegisters {
                ebx: (1 << 3)
                    | (1 << 5)
                    | (1 << 8)
                    | (1 << 16)
                    | (1 << 17)
                    | (1 << 28)
                    | (1 << 30)
                    | (1 << 31),
                ..CpuidRegisters::default()
            },
        ),
        (
            0x8000_0000,
            CpuidRegisters {
                eax: 0x8000_0001,
                ..CpuidRegisters::default()
            },
        ),
        (
            0x8000_0001,
            CpuidRegisters {
                ecx: 1 << 5,
                ..CpuidRegisters::default()
            },
        ),
    ] {
        src.entries.insert((leaf, 0), value);
    }
    src
}

#[test]
fn every_v3_feature_required() {
    assert!(!uarch_tiers::X86_V3_FEATURE_LIST.is_empty());
    assert_eq!(
        classify(&intel_v4(), DetectPolicy::default()),
        TierId::X86V3
    );
    for (leaf, register, bit) in [
        (1, 2, 0),
        (1, 2, 9),
        (1, 2, 12),
        (1, 2, 13),
        (1, 2, 19),
        (1, 2, 20),
        (1, 2, 22),
        (1, 2, 23),
        (1, 2, 26),
        (1, 2, 27),
        (1, 2, 28),
        (1, 2, 29),
        (7, 1, 3),
        (7, 1, 5),
        (7, 1, 8),
        (0x8000_0001, 2, 5),
    ] {
        let mut src = intel_v4();
        let Some(entry) = src.entries.get_mut(&(leaf, 0)) else {
            panic!("fixture leaf missing")
        };
        match register {
            1 => entry.ebx &= !(1 << bit),
            2 => entry.ecx &= !(1 << bit),
            _ => panic!("invalid register"),
        }
        assert_eq!(
            classify(&src, DetectPolicy::default()),
            TierId::Scalar,
            "missing leaf {leaf:x} bit {bit}"
        );
    }
}

#[test]
fn os_state_gates_vector_execution() {
    for state in [None, Some(0), Some(2), Some(4)] {
        let mut src = intel_v4();
        src.xcr0 = state;
        assert_eq!(classify(&src, DetectPolicy::default()), TierId::Scalar);
    }
    let wide = DetectPolicy {
        wide: uarch_detect::WidePolicy::Allow512,
        ..DetectPolicy::default()
    };
    for state in [6, 0x26, 0x66, 0xa6] {
        let mut src = intel_v4();
        src.xcr0 = Some(state);
        assert_eq!(classify(&src, wide), TierId::X86V3);
    }
}

#[test]
fn every_v4_feature_required() {
    assert!(!uarch_tiers::X86_V4_FEATURE_LIST.is_empty());
    let wide = DetectPolicy {
        wide: uarch_detect::WidePolicy::Allow512,
        ..DetectPolicy::default()
    };
    assert_eq!(classify(&intel_v4(), wide), TierId::X86V4);
    for bit in [16, 17, 28, 30, 31] {
        let mut src = intel_v4();
        let Some(entry) = src.entries.get_mut(&(7, 0)) else {
            panic!("fixture leaf missing")
        };
        entry.ebx &= !(1 << bit);
        assert_eq!(classify(&src, wide), TierId::X86V3);
    }
}

#[test]
fn maximum_leaf_limits_are_respected() {
    for (leaf, max) in [(0, 6), (0x8000_0000, 0x8000_0000)] {
        let mut src = intel_v4();
        let Some(entry) = src.entries.get_mut(&(leaf, 0)) else {
            panic!("fixture leaf missing")
        };
        entry.eax = max;
        assert_eq!(classify(&src, DetectPolicy::default()), TierId::Scalar);
    }
}

#[test]
fn family_model_uses_base_family() {
    use uarch_detect::cpuid::family_model;
    assert_eq!(family_model(0x0005_0650), (6, 0x55));
    assert_eq!(family_model(0x0080_0f10), (0x17, 1));
    assert_eq!(family_model(0x0090_0f10), (0x18, 1));
    assert_eq!(family_model(0x00f5_0540), (5, 4));
}
