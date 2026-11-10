use uarch_arch::{Aarch64Neon, Arch, PrefetchWrite, Scalar, X86V3, X86V3SlowPdep, X86V4};

fn transferable<T: Copy + Send + Sync + 'static>() {}
fn capability<A: Arch>() {
    transferable::<A>();
}

#[test]
fn capabilities_are_transferable() {
    capability::<Scalar>();
    capability::<X86V3>();
    capability::<X86V3SlowPdep>();
    capability::<X86V4>();
    capability::<Aarch64Neon>();
    transferable::<PrefetchWrite>();
}

#[test]
fn capability_traits_preserve_identity() {
    use uarch_arch::{
        Aarch64Neon, PrefetchWrite, Scalar, Tier, TierId, X86V3, X86V3SlowPdep, X86V4,
    };
    fn traits<T: Copy + Clone + Eq + core::hash::Hash + core::fmt::Debug>() {}
    traits::<Scalar>();
    traits::<X86V3>();
    traits::<X86V3SlowPdep>();
    traits::<X86V4>();
    traits::<Aarch64Neon>();
    traits::<PrefetchWrite>();
    traits::<Tier>();
    const ID: TierId = Tier::Scalar(Scalar::new()).id();
    assert_eq!(ID, TierId::Scalar);
}

#[test]
fn reference_prefetch_keeps_value() {
    use uarch_arch::prefetch_read_ref;
    use uarch_core::mem::{L1, L2, L3, NonTemporal};
    let value = 0x1234_5678u64;
    prefetch_read_ref::<_, L1>(&value);
    prefetch_read_ref::<_, L2>(&value);
    prefetch_read_ref::<_, L3>(&value);
    prefetch_read_ref::<_, NonTemporal>(&value);
    let token = if cfg!(miri) {
        // SAFETY: The Miri fallback issues no ISA instruction and accesses no memory.
        Some(unsafe { PrefetchWrite::new_unchecked() })
    } else {
        uarch_detect::detect_prefetch_write().ok()
    };
    if let Some(token) = token {
        uarch_arch::prefetch_write_ref::<_, L1>(token, &value);
    }
    assert_eq!(value, 0x1234_5678);
}

#[cfg(target_arch = "aarch64")]
#[test]
fn baseline_prefetch_has_default() {
    assert_eq!(
        uarch_arch::PrefetchWrite::default(),
        uarch_arch::PrefetchWrite::new()
    );
}

#[test]
fn capabilities_have_zero_register_leaves() {
    fn marker<T: uarch_core::layout::RegisterLeaf>() {
        assert_eq!(T::LEAVES, 0);
        assert_eq!(core::mem::size_of::<T>(), 0);
    }
    marker::<Scalar>();
    marker::<X86V3>();
    marker::<X86V3SlowPdep>();
    marker::<X86V4>();
    marker::<Aarch64Neon>();
    marker::<PrefetchWrite>();
}
