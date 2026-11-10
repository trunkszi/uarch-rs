// Independent coverage belongs to the applicable higher-level suites.

#[test]
fn tier_identity_is_not_authority() {
    use uarch_tiers::TierId;
    for tier in TierId::ALL {
        assert!(!tier.as_str().is_empty());
    }
    assert_eq!(TierId::Scalar.target_cpu(), None);
    assert_eq!(TierId::X86V3.target_cpu(), Some("x86-64-v3"));
    assert_eq!(
        TierId::X86V3SlowPdep.target_cpu(),
        TierId::X86V3.target_cpu()
    );
    assert_eq!(TierId::X86V4.target_cpu(), Some("x86-64-v4"));
    assert_eq!(TierId::Aarch64Neon.target_cpu(), Some("generic"));
    const COMPILED: TierId = TierId::compiled();
    assert!(TierId::ALL.contains(&COMPILED));
}

#[test]
fn compiled_feature_lists_match() {
    use std::collections::BTreeSet;
    use uarch_tiers::TierId;
    let source = include_str!("../src/lib.rs");
    let mut conditions = source
        .split_once("pub const fn compiled()")
        .unwrap_or_else(|| panic!("missing compiled function"))
        .1;
    for (tier, marker) in [
        (TierId::X86V4, "return Self::X86V4;"),
        (TierId::X86V3, "return Self::X86V3;"),
    ] {
        let (condition, remainder) = conditions
            .split_once(marker)
            .unwrap_or_else(|| panic!("missing tier branch"));
        let names: Vec<_> = condition
            .split("target_feature = \"")
            .skip(1)
            .map(|feature| {
                feature
                    .split_once('"')
                    .unwrap_or_else(|| panic!("unterminated feature"))
                    .0
            })
            .collect();
        let checked: BTreeSet<_> = names.iter().copied().collect();
        let listed: BTreeSet<_> = tier.feature_list().iter().copied().collect();
        assert_eq!(names.len(), checked.len());
        assert_eq!(tier.feature_list().len(), listed.len());
        assert_eq!(checked, listed);
        conditions = remainder;
    }
}
