use uarch_core::flow::{Predicate, predicate};

#[test]
fn ops_stay_in_bit() {
    for flag in [false, true] {
        let from_bool = Predicate::from(flag);
        assert_eq!(Predicate::new(flag), predicate(flag));
        assert_eq!(from_bool, predicate(flag));
        assert_eq!(from_bool.as_bool(), flag);
        assert_eq!(
            from_bool.select_register(11u64, 22),
            if flag { 11 } else { 22 }
        );
    }
    assert_eq!(predicate(true) & predicate(false), predicate(false));
    assert_eq!(predicate(true) | predicate(false), predicate(true));
    assert_eq!(predicate(true) ^ predicate(true), predicate(false));
    assert_eq!(!predicate(true), predicate(false));
    assert_eq!(!predicate(false), predicate(true));
    assert_eq!(predicate(true) * 0xABu8, 0xAB);
    assert_eq!(predicate(false) * 0xABu64, 0);
    assert_eq!(predicate(true) * -5i32, -5);
    assert_eq!(predicate(false) * i8::MIN, 0);
    assert_eq!(predicate(true) * usize::MAX, usize::MAX);
}

#[test]
fn sign_min_stays_min() {
    assert_eq!(predicate(true).select_sign_i64(5), 5);
    assert_eq!(predicate(false).select_sign_i64(5), -5);
    assert_eq!(predicate(true).select_sign_i64(i64::MIN), i64::MIN);
    assert_eq!(predicate(false).select_sign_i64(i64::MIN), i64::MIN);
}

#[test]
fn predicate_is_one_leaf() {
    assert_eq!(<Predicate as uarch_core::layout::RegisterLeaf>::LEAVES, 1);
    assert_eq!(core::mem::size_of::<Predicate>(), 1);
}
