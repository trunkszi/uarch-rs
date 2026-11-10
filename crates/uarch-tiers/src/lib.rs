#![no_std]

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TierId {
    Scalar = 0,
    X86V3 = 1,
    X86V3SlowPdep = 2,
    X86V4 = 3,
    Aarch64Neon = 4,
}

pub const X86_V3_FEATURES: &str =
    "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave";
pub const X86_V4_FEATURES: &str = "avx,avx2,bmi1,bmi2,cmpxchg16b,f16c,fma,lzcnt,movbe,popcnt,sse3,ssse3,sse4.1,sse4.2,xsave,avx512f,avx512bw,avx512cd,avx512dq,avx512vl";
pub const AARCH64_NEON_FEATURES: &str = "neon";

pub const X86_V3_FEATURE_LIST: &[&str] = &[
    "avx",
    "avx2",
    "bmi1",
    "bmi2",
    "cmpxchg16b",
    "f16c",
    "fma",
    "lzcnt",
    "movbe",
    "popcnt",
    "sse3",
    "ssse3",
    "sse4.1",
    "sse4.2",
    "xsave",
];
pub const X86_V4_FEATURE_LIST: &[&str] = &[
    "avx",
    "avx2",
    "bmi1",
    "bmi2",
    "cmpxchg16b",
    "f16c",
    "fma",
    "lzcnt",
    "movbe",
    "popcnt",
    "sse3",
    "ssse3",
    "sse4.1",
    "sse4.2",
    "xsave",
    "avx512f",
    "avx512bw",
    "avx512cd",
    "avx512dq",
    "avx512vl",
];
pub const AARCH64_NEON_FEATURE_LIST: &[&str] = &["neon"];

impl TierId {
    /// Stable textual identifier, independent of host execution authority.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::X86V3 => "x86_v3",
            Self::X86V3SlowPdep => "x86_v3_slow_pdep",
            Self::X86V4 => "x86_v4",
            Self::Aarch64Neon => "aarch64_neon",
        }
    }
    /// LLVM CPU name for this tier's target; it is not host detection.
    pub const fn target_cpu(self) -> Option<&'static str> {
        match self {
            Self::Scalar => None,
            Self::X86V3 | Self::X86V3SlowPdep => Some("x86-64-v3"),
            Self::X86V4 => Some("x86-64-v4"),
            Self::Aarch64Neon => Some("generic"),
        }
    }
    /// ISA features enabled when compiling this crate, not a runtime capability.
    pub const fn compiled() -> Self {
        if cfg!(all(
            target_arch = "x86_64",
            target_feature = "avx",
            target_feature = "avx2",
            target_feature = "bmi1",
            target_feature = "bmi2",
            target_feature = "cmpxchg16b",
            target_feature = "f16c",
            target_feature = "fma",
            target_feature = "lzcnt",
            target_feature = "movbe",
            target_feature = "popcnt",
            target_feature = "sse3",
            target_feature = "ssse3",
            target_feature = "sse4.1",
            target_feature = "sse4.2",
            target_feature = "xsave",
            target_feature = "avx512f",
            target_feature = "avx512bw",
            target_feature = "avx512cd",
            target_feature = "avx512dq",
            target_feature = "avx512vl"
        )) {
            return Self::X86V4;
        }
        if cfg!(all(
            target_arch = "x86_64",
            target_feature = "avx",
            target_feature = "avx2",
            target_feature = "bmi1",
            target_feature = "bmi2",
            target_feature = "cmpxchg16b",
            target_feature = "f16c",
            target_feature = "fma",
            target_feature = "lzcnt",
            target_feature = "movbe",
            target_feature = "popcnt",
            target_feature = "sse3",
            target_feature = "ssse3",
            target_feature = "sse4.1",
            target_feature = "sse4.2",
            target_feature = "xsave"
        )) {
            return Self::X86V3;
        }
        if cfg!(all(target_arch = "aarch64", target_feature = "neon")) {
            Self::Aarch64Neon
        } else {
            Self::Scalar
        }
    }
    pub const ALL: [Self; 5] = [
        Self::Scalar,
        Self::X86V3,
        Self::X86V3SlowPdep,
        Self::X86V4,
        Self::Aarch64Neon,
    ];

    pub const fn features(self) -> &'static str {
        match self {
            Self::Scalar => "",
            Self::X86V3 | Self::X86V3SlowPdep => X86_V3_FEATURES,
            Self::X86V4 => X86_V4_FEATURES,
            Self::Aarch64Neon => AARCH64_NEON_FEATURES,
        }
    }

    pub const fn feature_list(self) -> &'static [&'static str] {
        match self {
            Self::Scalar => &[],
            Self::X86V3 | Self::X86V3SlowPdep => X86_V3_FEATURE_LIST,
            Self::X86V4 => X86_V4_FEATURE_LIST,
            Self::Aarch64Neon => AARCH64_NEON_FEATURE_LIST,
        }
    }

    pub const fn target_arch(self) -> Option<&'static str> {
        match self {
            Self::Scalar => None,
            Self::X86V3 | Self::X86V3SlowPdep | Self::X86V4 => Some("x86_64"),
            Self::Aarch64Neon => Some("aarch64"),
        }
    }
}
