#![allow(dead_code, unused_imports)]
use uarch::Arch;
use uarch_macros::multiversion;

#[multiversion]
fn bad<A: Arch>(&self, arch: A) {}
fn main() {}
