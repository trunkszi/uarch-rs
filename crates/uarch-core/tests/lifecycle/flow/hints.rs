use uarch_core::flow::{cold_path, predicate};

#[test]
fn cold_hint_keeps_arm() {
    let value = 41u64;
    let out = if value == 0 {
        cold_path();
        0
    } else {
        predicate(true).select_register(value, 0)
    };
    assert_eq!(out, 41);
}
