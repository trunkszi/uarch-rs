#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, cold_sink, hot_inline, isolated, register_pass};

#[cold_sink]
#[inline(always)]
fn bad() {}
fn main() {}
