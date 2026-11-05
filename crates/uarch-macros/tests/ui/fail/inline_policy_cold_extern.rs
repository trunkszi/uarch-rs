#![allow(dead_code, unused_imports)]
use uarch_macros::{cold_sink, hot_inline};
#[cold_sink]
extern "C" fn bad() {}
fn main() {}
