#![allow(dead_code, unused_macros)]
#[path = "support/layout.rs"]
mod support;
use support::__private;
use uarch_macros::isolated;

macro_rules! assert {
    ($($token:tt)*) => {};
}

#[isolated(full, crate = "crate")]
struct Bad {
    first: u8,
    second: u64,
    reserved: [u8; 119],
}

fn main() {}
