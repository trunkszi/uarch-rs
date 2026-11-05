use crate::{Aarch64Neon, Arch, Scalar, X86V3, X86V3SlowPdep, X86V4};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Scalar(Scalar),
    X86V3(X86V3),
    X86V3SlowPdep(X86V3SlowPdep),
    X86V4(X86V4),
    Aarch64Neon(Aarch64Neon),
}

pub trait ArchVisitor {
    type Output;
    fn visit<A: Arch>(self, arch: A) -> Self::Output;
}

impl Tier {
    /// Identity of the held capability, without dropping its proof.
    #[inline(always)]
    pub const fn id(self) -> crate::TierId {
        match self {
            Self::Scalar(_) => crate::TierId::Scalar,
            Self::X86V3(_) => crate::TierId::X86V3,
            Self::X86V3SlowPdep(_) => crate::TierId::X86V3SlowPdep,
            Self::X86V4(_) => crate::TierId::X86V4,
            Self::Aarch64Neon(_) => crate::TierId::Aarch64Neon,
        }
    }
    #[inline(always)]
    pub fn dispatch<V: ArchVisitor>(self, visitor: V) -> V::Output {
        match self {
            Self::Scalar(a) => visitor.visit(a),
            Self::X86V3(a) => visitor.visit(a),
            Self::X86V3SlowPdep(a) => visitor.visit(a),
            Self::X86V4(a) => visitor.visit(a),
            Self::Aarch64Neon(a) => visitor.visit(a),
        }
    }
}

/// Changes the static type of a capability after a tier dispatch.
///
/// # Panics
/// Panics if the source and destination tiers differ.
///
/// # Safety
/// The caller must prove `From::TIER == To::TIER` and that all execution CPUs
/// support `To`. All implementations are sealed, inhabited, zero-sized tokens.
#[inline(always)]
pub const unsafe fn retag_unchecked<From: Arch, To: Arch>(_from: From) -> To {
    assert!(From::TIER as u8 == To::TIER as u8);
    const { assert!(core::mem::size_of::<To>() == 0) };
    // SAFETY: All sealed Arch implementors contain only a private unit field.
    unsafe { core::mem::MaybeUninit::<To>::zeroed().assume_init() }
}
