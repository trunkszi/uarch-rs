#![allow(dead_code, unused_imports)]
#[path = "support/layout.rs"]
mod support;
use support::__private;
use uarch_macros::{cache_line, isolated, register_pass};

#[register_pass(crate = "crate", crate = "crate")]
struct Bad {
    value: u64,
}
fn main() {}
