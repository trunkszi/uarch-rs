#![allow(dead_code, unused_imports)]
use uarch::Arch;
use uarch_macros::multiversion;

#[multiversion]
#[target_feature(enable = "sse2")]
fn bad<A: Arch>(arch: A) {}
fn main() {}
