use uarch_tiers::{X86_V3_FEATURE_LIST, X86_V4_FEATURE_LIST};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuidRegisters {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

/// Observation seam only: implementing this trait cannot issue capabilities.
pub trait CpuidSource {
    fn cpuid(&self, leaf: u32, subleaf: u32) -> CpuidRegisters;
    /// `None` means the OS state is unavailable or XGETBV is unsafe to execute.
    fn xcr0(&self) -> Option<u64>;
}

#[derive(Debug, Clone, Copy)]
pub struct HardwareCpuid;

impl CpuidSource for HardwareCpuid {
    fn cpuid(&self, leaf: u32, subleaf: u32) -> CpuidRegisters {
        #[cfg(target_arch = "x86_64")]
        {
            #[allow(unused_unsafe)] // The intrinsic became safe after the supported MSRV.
            // SAFETY: CPUID is part of the x86_64 baseline; unsupported leaves
            // are interpreted only after checking their maximum-leaf values.
            let r = unsafe { core::arch::x86_64::__cpuid_count(leaf, subleaf) };
            CpuidRegisters {
                eax: r.eax,
                ebx: r.ebx,
                ecx: r.ecx,
                edx: r.edx,
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (leaf, subleaf);
            CpuidRegisters::default()
        }
    }
    fn xcr0(&self) -> Option<u64> {
        #[cfg(target_arch = "x86_64")]
        {
            let c = self.cpuid(1, 0).ecx;
            if c & (3 << 26) != 3 << 26 {
                return None;
            }
            // SAFETY: XSAVE and OSXSAVE are both set, permitting XGETBV(0).
            Some(unsafe { core::arch::x86_64::_xgetbv(0) })
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            None
        }
    }
}

pub(crate) fn available<S: CpuidSource>(src: &S, leaf: u32) -> bool {
    let max_leaf = if leaf >= 0x8000_0000 { 0x8000_0000 } else { 0 };
    src.cpuid(max_leaf, 0).eax >= leaf
}

pub(crate) fn feature<S: CpuidSource>(src: &S, name: &str) -> bool {
    let (leaf, register, bit) = match name {
        "sse3" => (1, 2, 0),
        "ssse3" => (1, 2, 9),
        "fma" => (1, 2, 12),
        "cmpxchg16b" => (1, 2, 13),
        "sse4.1" => (1, 2, 19),
        "sse4.2" => (1, 2, 20),
        "movbe" => (1, 2, 22),
        "popcnt" => (1, 2, 23),
        "xsave" => (1, 2, 26),
        "avx" => (1, 2, 28),
        "f16c" => (1, 2, 29),
        "bmi1" => (7, 1, 3),
        "avx2" => (7, 1, 5),
        "bmi2" => (7, 1, 8),
        "avx512f" => (7, 1, 16),
        "avx512dq" => (7, 1, 17),
        "avx512cd" => (7, 1, 28),
        "avx512bw" => (7, 1, 30),
        "avx512vl" => (7, 1, 31),
        "lzcnt" => (0x8000_0001, 2, 5),
        _ => return false,
    };
    if !available(src, leaf) {
        return false;
    }
    let r = src.cpuid(leaf, 0);
    let value = match register {
        1 => r.ebx,
        2 => r.ecx,
        _ => return false,
    };
    value & (1 << bit) != 0
}

fn os_state<S: CpuidSource>(src: &S, mask: u64) -> bool {
    if !available(src, 1) || src.cpuid(1, 0).ecx & (3 << 26) != 3 << 26 {
        return false;
    }
    src.xcr0().is_some_and(|x| x & mask == mask)
}

pub(crate) fn v3_supported<S: CpuidSource>(src: &S) -> bool {
    os_state(src, 0x6) && X86_V3_FEATURE_LIST.iter().all(|name| feature(src, name))
}

pub(crate) fn v4_supported<S: CpuidSource>(src: &S) -> bool {
    os_state(src, 0xe6) && X86_V4_FEATURE_LIST.iter().all(|name| feature(src, name))
}

pub(crate) fn prefetch_write_supported<S: CpuidSource>(src: &S) -> bool {
    available(src, 0x8000_0001) && src.cpuid(0x8000_0001, 0).ecx & (1 << 8) != 0
}

pub(crate) fn same_isa<S: CpuidSource>(a: &S, b: &S) -> bool {
    X86_V4_FEATURE_LIST
        .iter()
        .all(|name| feature(a, name) == feature(b, name))
        && (a.cpuid(1, 0).ecx & (3 << 26)) == (b.cpuid(1, 0).ecx & (3 << 26))
        && a.xcr0() == b.xcr0()
        && prefetch_write_supported(a) == prefetch_write_supported(b)
}

/// Decodes display family/model according to the Intel/AMD CPUID.1:EAX rules.
pub fn family_model(signature: u32) -> (u16, u16) {
    let base_family = (signature >> 8) & 0xf;
    let family = base_family
        + if base_family == 0xf {
            (signature >> 20) & 0xff
        } else {
            0
        };
    let model = ((signature >> 4) & 0xf)
        + if base_family == 6 || base_family == 0xf {
            (signature >> 12) & 0xf0
        } else {
            0
        };
    (family as u16, model as u16)
}

pub(crate) fn vendor<S: CpuidSource>(src: &S) -> [u8; 12] {
    let r = src.cpuid(0, 0);
    let mut bytes = [0; 12];
    bytes[0..4].copy_from_slice(&r.ebx.to_le_bytes());
    bytes[4..8].copy_from_slice(&r.edx.to_le_bytes());
    bytes[8..12].copy_from_slice(&r.ecx.to_le_bytes());
    bytes
}

#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    pub(crate) basic: CpuidRegisters,
    pub(crate) signature: CpuidRegisters,
    pub(crate) structured: CpuidRegisters,
    pub(crate) extended: CpuidRegisters,
    pub(crate) extended_features: CpuidRegisters,
    pub(crate) xcr0: Option<u64>,
}

#[cfg(target_arch = "x86_64")]
impl CpuidSource for Snapshot {
    fn cpuid(&self, leaf: u32, _subleaf: u32) -> CpuidRegisters {
        match leaf {
            0 => self.basic,
            1 => self.signature,
            7 => self.structured,
            0x8000_0000 => self.extended,
            0x8000_0001 => self.extended_features,
            _ => CpuidRegisters::default(),
        }
    }
    fn xcr0(&self) -> Option<u64> {
        self.xcr0
    }
}

#[cfg(target_arch = "x86_64")]
impl Snapshot {
    pub(crate) fn from_source(src: &impl CpuidSource) -> Self {
        Self {
            basic: src.cpuid(0, 0),
            signature: src.cpuid(1, 0),
            structured: src.cpuid(7, 0),
            extended: src.cpuid(0x8000_0000, 0),
            extended_features: src.cpuid(0x8000_0001, 0),
            xcr0: src.xcr0(),
        }
    }
}
