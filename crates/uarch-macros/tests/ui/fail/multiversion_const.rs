#![allow(dead_code, unused_imports)]
use uarch::Arch;
use uarch_macros::multiversion;

#[multiversion]
const fn bad<A: Arch>(arch: A) {}
fn main() {}
