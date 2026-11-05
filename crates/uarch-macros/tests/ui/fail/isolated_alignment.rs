#![allow(dead_code, unused_imports)]
#[path = "support/layout.rs"]
mod support;
use support::__private;
use uarch_macros::{cache_line, isolated, register_pass};

#[repr(C, align(256))]
struct Aligned {
    bytes: [u8; 256],
}
#[isolated(crate = "crate")]
struct Bad {
    value: Aligned,
}
fn main() {}
