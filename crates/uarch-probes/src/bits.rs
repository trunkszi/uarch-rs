//! Stable symbols for bit-algebra instruction audits.
use uarch_core::{bits::BitPattern, bitword};

bitword! {
    pub struct ProbeWord(u64) {
        [0] pub READY,
        [1] pub ACTIVE,
        [8..=10] pub STATE: u8,
        [60..=63] pub HIGH: u8,
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn bitword_low_matches(raw: u64) -> bool {
    const PATTERN: BitPattern<ProbeWord> = BitPattern::ANY
        .and_flag(ProbeWord::READY)
        .and_not(ProbeWord::ACTIVE)
        .and_eq(ProbeWord::STATE, 3);
    ProbeWord::new(raw).matches(PATTERN)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn bitword_high_matches(raw: u64) -> bool {
    const PATTERN: BitPattern<ProbeWord> = BitPattern::ANY
        .and_flag(ProbeWord::READY)
        .and_eq(ProbeWord::HIGH, 3);
    ProbeWord::new(raw).matches(PATTERN)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn bitpatch_apply(raw: u64) -> u64 {
    const PATCH: uarch_core::bits::BitPatch<ProbeWord> = uarch_core::bits::BitPatch::EMPTY
        .with_flag(ProbeWord::READY, true)
        .with_field(ProbeWord::STATE, 5);
    ProbeWord::new(raw).patch(PATCH).raw()
}

/// Returns the remaining bits and the removed index (64 for an empty bitmap).
#[unsafe(no_mangle)]
#[inline(never)]
pub fn bitmap64_pop_lowest(raw: u64) -> (u64, u32) {
    let mut bitmap = uarch_core::bits::Bitmap64::new(raw);
    let index = bitmap.pop_lowest_one().unwrap_or(64);
    (bitmap.raw(), index)
}
