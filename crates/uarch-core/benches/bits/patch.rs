use super::Word;
use uarch_core::bits::{BitField, BitFlag, BitPatch};

pub(super) fn run(raw: u64) -> u64 {
    const PATCH: BitPatch<Word> = BitPatch::EMPTY
        .with_flag(BitFlag::new(0), true)
        .with_field(BitField::<Word, u8, 8, 11>::new(), 3);
    PATCH.apply(raw)
}
