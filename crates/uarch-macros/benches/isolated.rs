use std::hint::black_box;
use uarch_macros::isolated;

#[isolated]
struct Cursor {
    value: u64,
}

pub(super) fn run(value: u64) -> u64 {
    let cursor = Cursor { value };
    black_box(&cursor).value
}
