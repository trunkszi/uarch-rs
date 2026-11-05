use super::Word;
use uarch_core::bits::{BitField, BitFlag, BitPattern};

pub(super) fn run(raw: u64) -> u64 {
    const PATTERN: BitPattern<Word> = BitPattern::ANY
        .and_flag(BitFlag::new(0))
        .and_eq(BitField::<Word, u8, 8, 11>::new(), 3);
    u64::from(PATTERN.matches(raw))
}
