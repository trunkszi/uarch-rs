#![allow(dead_code)]
#[path = "support/const_bridge.rs"]
mod support;
use support::{__private, Word};
fn main() {
    let _ = uarch_macros::patch!(crate = "crate", Word { FIELD: 16 });
}
