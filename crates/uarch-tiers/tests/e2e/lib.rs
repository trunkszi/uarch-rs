use uarch_tiers::{TierId, X86_V3_FEATURES, X86_V4_FEATURES};

#[test]
fn feature_sources_agree() {
    for tier in TierId::ALL {
        assert_eq!(
            tier.features()
                .split(',')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            tier.feature_list()
        );
    }
    assert_eq!(TierId::X86V3.features(), TierId::X86V3SlowPdep.features());
    assert!(X86_V4_FEATURES.starts_with(X86_V3_FEATURES));
    assert_eq!(TierId::Aarch64Neon.target_arch(), Some("aarch64"));
    assert_eq!(TierId::Scalar.target_arch(), None);
}
