#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, cold_sink, hot_inline, isolated, register_pass};

#[hot_inline]
#[target_feature(enable = "sse2")]
fn bad() {}
fn main() {}
