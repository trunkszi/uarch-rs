#![allow(dead_code, unused_imports)]
use uarch_macros::{cache_line, cold_sink, hot_inline, isolated, register_pass};

fn bad() {
    let _ = uarch_macros::pattern!(Unknown { READY & });
}
fn main() {}
