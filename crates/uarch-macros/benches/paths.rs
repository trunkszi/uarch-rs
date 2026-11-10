use std::hint::black_box;
use uarch_macros::register_pass;

#[register_pass(crate = "::uarch")]
#[derive(Clone, Copy)]
struct Scalar {
    value: u64,
}

pub(super) fn run(value: u64) -> u64 {
    black_box(Scalar { value }).value
}
