use core::cell::Cell;
use uarch_core::flow::{reduce_four_way, reduce_lanes};

#[test]
fn reduce_init_once() {
    let xs = [1u64, 2, 3, 4, 5];
    let steps = Cell::new(0u32);
    let got = reduce_lanes::<4, _, _>(
        &xs,
        10,
        0,
        |acc, item| {
            steps.set(steps.get() + 1);
            acc + *item
        },
        |left, right| left + right,
    );
    assert_eq!(steps.get(), 5);
    assert_eq!(got, 25);

    let mixed = reduce_four_way!(&xs, 0xFFu64, 0, |acc, item| acc ^ *item, |a, b| a ^ b);
    assert_eq!(mixed, 0xFF ^ 1 ^ 2 ^ 3 ^ 4 ^ 5);
}

#[test]
fn reduce_empty_is_init() {
    let got = reduce_lanes::<4, u64, u64>(&[], 7, 0, |acc, _| acc, |a, b| a.wrapping_add(b));
    assert_eq!(got, 7);
    let wide = reduce_lanes::<8, u32, u32>(&[1, 2, 3], 9, 0, |acc, item| acc + *item, |a, b| a + b);
    assert_eq!(wide, 15);
    let pair = reduce_lanes::<2, u32, u32>(&[4, 5, 6], 1, 0, |acc, item| acc + *item, |a, b| a + b);
    assert_eq!(pair, 16);
}
