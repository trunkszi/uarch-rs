#![allow(dead_code, unused_imports)]
use uarch_macros::{cold_sink, hot_inline};
#[hot_inline]
#[cold]
fn bad() {}
fn main() {}
