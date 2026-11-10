use core::num::NonZeroU64;
use std::hint::black_box;
use uarch::{arithmetic::FastDivisorU64, bitword, patch, pattern};

bitword! {
    pub struct Flags(u64) {
        [0] pub READY,
        [1] pub ACTIVE,
        [2] pub BUSY,
        [8..=15] pub CODE: u8,
    }
}

const DIVISOR: NonZeroU64 = match NonZeroU64::new(7) {
    Some(value) => value,
    None => panic!("nonzero constant divisor"),
};
const FAST: FastDivisorU64 = FastDivisorU64::new(DIVISOR);

struct Descriptors {
    divisor: u64,
    checks: [(u64, u64); 3],
    edits: [(u64, u64); 10],
}

const DESCRIPTORS: Descriptors = Descriptors {
    divisor: DIVISOR.get(),
    checks: [(1, 1), (2, 2), (4, 0)],
    edits: [
        (2, 2),
        (4, 0),
        (0x100, 0),
        (0x200, 0x200),
        (0x400, 0),
        (0x800, 0x800),
        (0x1000, 0x1000),
        (0x2000, 0),
        (0x4000, 0x4000),
        (0x8000, 0),
    ],
};

pub(super) fn bad(data: &[u64; 256]) -> u64 {
    let descriptors = black_box(&DESCRIPTORS);
    data.iter().fold(0u64, |sum, raw| {
        let raw = black_box(*raw);
        let quotient = raw / descriptors.divisor;
        let remainder = raw % descriptors.divisor;
        let matches = descriptors
            .checks
            .iter()
            .fold(true, |matched, &(mask, expected)| {
                matched & (raw & mask == expected)
            });
        let patched = descriptors
            .edits
            .iter()
            .fold(raw, |word, &(mask, value)| (word & !mask) | value);
        sum.wrapping_add(black_box(
            quotient ^ remainder ^ patched ^ u64::from(matches),
        ))
    })
}

pub(super) fn good(data: &[u64; 256]) -> u64 {
    let divisor = black_box(FAST);
    data.iter().fold(0u64, |sum, raw| {
        let raw = black_box(*raw);
        let (quotient, remainder) = divisor.div_rem(raw);
        let word = Flags::new(raw);
        let matches = word.matches(pattern!(Flags { READY & ACTIVE & !BUSY }));
        let patched = word
            .patch(patch!(Flags {
                ACTIVE: true,
                BUSY: false,
                CODE: 0x5a
            }))
            .raw();
        sum.wrapping_add(black_box(
            quotient ^ remainder ^ patched ^ u64::from(matches),
        ))
    })
}
