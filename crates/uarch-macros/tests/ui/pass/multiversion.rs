use uarch::{Arch, Scalar};
use uarch_macros::multiversion;

fn same_scope(value: u64) -> u64 {
    value + 1
}

#[multiversion]
fn evaluate<A: Arch>(arch: A, value: u64) -> u64 {
    assert_eq!(A::VECTOR_BYTES, <A as Arch>::VECTOR_BYTES);
    let typed: A = arch;
    same_scope(typed.bit_gather(value, u64::MAX)) + A::VECTOR_BYTES as u64
}

#[multiversion(crate = "::uarch")]
fn borrow<'a, A: Arch>(arch: A, (left, _): (&'a u64, u64)) -> &'a u64 {
    let _ = <A as Arch>::TIER;
    let _ = arch;
    left
}

mod data {
    pub type A = u64;
}

#[multiversion]
fn qualified<A: Arch>(arch: A, value: data::A) -> data::A {
    let _ = arch;
    value
}

fn main() {
    // SAFETY: Scalar requires no optional ISA support.
    let arch = unsafe { Scalar::new_unchecked() };
    assert_eq!(evaluate(arch, 6), 7);
    assert_eq!(qualified(arch, 9), 9);
    assert_eq!(*borrow(arch, (&42, 0)), 42);
}
