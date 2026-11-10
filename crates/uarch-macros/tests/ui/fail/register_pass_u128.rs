#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, isolated, register_pass};
#[path = "support/layout.rs"]
mod support;
use support::__private;

#[register_pass(crate = "crate")]
#[derive(Clone, Copy)]
struct Bad {
    value: u128,
}
fn main() {}
