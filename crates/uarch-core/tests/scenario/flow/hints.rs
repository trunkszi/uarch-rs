//! Covered through the flow higher-level behavior suites; duplicate tests are omitted.

#[test]
fn probability_hints_preserve_boolean() {
    use uarch_core::flow::{likely, unlikely};
    const YES: bool = likely(true);
    const NO: bool = unlikely(false);
    const {
        assert!(YES);
    }
    const {
        assert!(!NO);
    }
    for condition in [false, true] {
        assert_eq!(likely(condition), condition);
        assert_eq!(unlikely(condition), condition);
    }
}
