use uarch::prelude::*;

fn decide(predictable_ok: bool, data_bit: bool, value: u64) -> u64 {
    if !predictable_ok {
        cold_path();
        return 0;
    }
    predicate(data_bit).select(value, 0)
}

#[test]
fn hot_arm_keeps_value() {
    assert_eq!(decide(false, true, 9), 0);
    assert_eq!(decide(true, true, 9), 9);
    assert_eq!(decide(true, false, 9), 0);
}
