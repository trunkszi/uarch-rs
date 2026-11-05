#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, isolated, register_pass};
#[path = "support/layout.rs"]
mod support;
use support::__private;

#[register_pass(c_abi, crate = "crate")]
#[derive(Clone, Copy)]
struct Bad {
    a: u64,
    b: u8,
}
fn main() {}
