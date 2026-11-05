#![allow(dead_code, unused_imports)]
#[path = "support/layout.rs"]
mod support;
use support::__private;
use uarch_macros::{cache_line, isolated, register_pass};

#[repr(C, align(128))]
struct Aligned {
    bytes: [u8; 128],
}
// SAFETY: The byte array occupies all 128 bytes of the aligned struct.
unsafe impl __private::NoPadding for Aligned {}
#[cache_line(lines = 2, crate = "crate")]
struct Bad {
    value: Aligned,
}
fn main() {}
