#![allow(dead_code, unused_imports)]
use uarch::Arch;
use uarch_macros::multiversion;

#[multiversion]
extern "C" fn bad<A: Arch>(arch: A) {}
fn main() {}
