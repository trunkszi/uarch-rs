#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, isolated, register_pass};
#[path = "support/layout.rs"]
mod support;
use support::__private;

struct Inner {
    value: u64,
}
#[cache_line(crate = "crate")]
struct Bad {
    value: [Inner; 8],
}
fn main() {}
