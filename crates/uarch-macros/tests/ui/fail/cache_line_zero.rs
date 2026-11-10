#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, cold_sink, hot_inline, isolated, register_pass};

#[cache_line(lines = 0)]
struct Bad {
    value: [u8; 64],
}
fn main() {}
