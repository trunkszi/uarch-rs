use core::sync::atomic::{AtomicU64, Ordering};
use uarch_macros::cache_line;

#[cache_line]
struct AtomicLine {
    value: AtomicU64,
    reserved: [u8; 56],
}

#[cache_line(lines = 2)]
struct TwoLines {
    value: [u64; 16],
}

#[cache_line(lines = 3)]
struct Lease { bytes: [u8; 192] }
#[cache_line(lines = 5)]
struct Cold { bytes: [u8; 320] }

fn requires_proof<T: uarch::__private::NoPadding>() {}

fn main() {
    requires_proof::<AtomicLine>();
    requires_proof::<TwoLines>();
    requires_proof::<Lease>();
    requires_proof::<Cold>();
    assert_eq!(core::mem::size_of::<Lease>(), 192);
    assert_eq!(core::mem::size_of::<Cold>(), 320);
    assert_eq!(core::mem::align_of::<Cold>(), 64);
    let line = AtomicLine {
        value: AtomicU64::new(7),
        reserved: [0; 56],
    };
    assert_eq!(line.value.load(Ordering::Relaxed), 7);
    assert_eq!(core::mem::size_of::<TwoLines>(), 128);
}
