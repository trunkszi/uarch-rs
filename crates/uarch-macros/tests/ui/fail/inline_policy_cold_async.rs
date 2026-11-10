#![allow(dead_code, unused_imports)]
use uarch_macros::{cold_sink, hot_inline};
#[cold_sink]
async fn bad() {}
fn main() {}
