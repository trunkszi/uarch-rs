use uarch::cold_path;

fn reject_cold(ready: bool, value: u64) -> u64 {
    if !ready {
        cold_path();
        return 0;
    }
    value
}

#[test]
fn cold_path_skips_hot_arm() {
    assert_eq!(reject_cold(true, 4), 4);
    assert_eq!(reject_cold(false, 4), 0);
}
