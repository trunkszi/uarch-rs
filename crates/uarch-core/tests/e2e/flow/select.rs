use uarch_core::flow::predicate;

#[test]
fn select_then_mask() {
    let word = 0x00FF_u64;
    let keep = predicate(word > 10).select(word, 0);
    let cleared = predicate(false) * keep;
    let passed = predicate(true) * keep;
    assert_eq!(cleared, 0);
    assert_eq!(passed, word);
}
