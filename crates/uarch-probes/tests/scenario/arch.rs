#[test]
fn target_feature_literals_match_tiers_ssot() {
    let source = include_str!("../../src/arch.rs");
    let mut counts = [0; 2];
    for attribute in source.split("#[target_feature(").skip(1) {
        let Some((body, suffix)) = attribute.split_once(")]") else {
            panic!("unterminated target_feature attribute");
        };
        let Some((_, enable)) = body.split_once("enable") else {
            panic!("missing enable argument");
        };
        let Some(literal) = enable.trim_start().strip_prefix('=') else {
            panic!("missing enable value");
        };
        let Some(literal) = literal.trim_start().strip_prefix('"') else {
            panic!("enable must be a string literal");
        };
        let Some((literal, _)) = literal.split_once('"') else {
            panic!("unterminated enable literal");
        };
        let Some((_, function)) = suffix.split_once("fn ") else {
            panic!("missing attributed function");
        };
        let v4 = function.starts_with("x86_v4_");
        let expected = if v4 {
            uarch_tiers::X86_V4_FEATURES
        } else {
            uarch_tiers::X86_V3_FEATURES
        };
        assert_eq!(literal, expected);
        counts[usize::from(v4)] += 1;
    }
    assert_eq!(counts, [6, 3]);
    assert_eq!(
        uarch_tiers::X86_V3_FEATURE_LIST.join(","),
        uarch_tiers::X86_V3_FEATURES
    );
    assert_eq!(
        uarch_tiers::X86_V4_FEATURE_LIST.join(","),
        uarch_tiers::X86_V4_FEATURES
    );
}
