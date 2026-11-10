use uarch_core::flow::predicate;

#[test]
fn select_register_picks_side() {
    assert_eq!(predicate(true).select_register(7u64, 9u64), 7);
    assert_eq!(predicate(false).select_register(7u64, 9u64), 9);
    assert_eq!(predicate(true).select_register(-3i32, 4), -3);
    assert_eq!(predicate(false).select_register(-3i64, 4), 4);
    assert_eq!(predicate(true).select(1usize, 2), 1);
    assert_eq!(predicate(false).select_register(1isize, -8), -8);
    let cond = predicate(true);
    assert_eq!(cond.select_register(11u32, 22), cond.select(11u32, 22));
}
