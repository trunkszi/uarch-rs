use super::Word;
use uarch_core::bits::BitField;

pub(super) fn run(raw: u64) -> u64 {
    BitField::<Word, u8, 8, 11>::new().wrapping_add_field(raw, 1)
}
