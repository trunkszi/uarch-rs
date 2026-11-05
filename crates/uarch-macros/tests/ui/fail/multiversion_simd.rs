#![allow(dead_code, unused_imports)]
use uarch::Arch;
use uarch_macros::multiversion;

#[multiversion]
fn bad<A: Arch>(arch: A, value: core::arch::x86_64::__m128i) {}
fn main() {}
