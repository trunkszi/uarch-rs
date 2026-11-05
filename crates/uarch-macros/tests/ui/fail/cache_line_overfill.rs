#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, isolated, register_pass};
#[path = "support/layout.rs"]
mod support;
use support::__private;

#[cache_line(crate = "crate")]
struct Bad {
    value: [u8; 72],
}
fn main() {}
