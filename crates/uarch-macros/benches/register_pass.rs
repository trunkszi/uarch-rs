use std::hint::black_box;
use uarch_macros::register_pass;

#[register_pass]
#[derive(Clone, Copy)]
struct Pair {
    left: u64,
    right: u64,
}

pub(super) fn run(value: u64) -> u64 {
    let pair = black_box(Pair {
        left: value,
        right: value + 1,
    });
    pair.left.wrapping_add(pair.right)
}
