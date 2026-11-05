#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, cold_sink, hot_inline, isolated, register_pass};

#[cache_line]
struct Bad {
    #[cfg_attr(all(), allow(dead_code))]
    value: [u8; 64],
}
fn main() {}
