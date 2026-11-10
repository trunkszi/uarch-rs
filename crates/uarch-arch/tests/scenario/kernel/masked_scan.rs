use uarch_arch::{Arch, Scalar};

#[test]
fn signed_bounds_wrap_correctly() {
    let arch = Scalar::new();
    let keys = [i64::MIN, -1, 0, i64::MAX];
    let vals = [i64::MAX, 1, 9, 37];
    assert_eq!(arch.masked_sum_lt_i64x4(&keys, &vals, 0), i64::MIN);
    assert_eq!(arch.masked_sum_lt_i64x4(&keys, &vals, i64::MIN), 0);
    assert_eq!(
        arch.masked_sum_lt_i64x4(&keys, &vals, i64::MAX),
        i64::MIN + 9
    );
    assert_eq!(arch.find_first_ge_i64x4(&keys, i64::MIN), Some(0));
    assert_eq!(arch.find_first_ge_i64x4(&keys, 0), Some(2));
    assert_eq!(arch.find_first_ge_i64x4(&keys, i64::MAX), Some(3));
    assert_eq!(arch.find_first_ge_i64x4(&[7, 2, 9, 1], 8), Some(2));
    assert_eq!(arch.find_first_ge_i64x4(&[7, 2, 9, 1], 10), None);
}
