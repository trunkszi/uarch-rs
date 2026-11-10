#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, isolated, register_pass};
#[path = "support/layout.rs"]
mod support;
use support::__private;

#[isolated(crate = "crate")]
struct Bad {
    value: u64,
}
const _: () = __private::assert_no_padding::<Bad>();
fn main() {}
