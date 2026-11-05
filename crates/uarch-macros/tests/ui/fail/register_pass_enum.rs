#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, cold_sink, hot_inline, isolated, register_pass};

#[register_pass]
enum Bad {
    A,
}
fn main() {}
