use super::Word;
use uarch_core::bits::BitFlag;

pub(super) fn run(raw: u64) -> u64 {
    let flags = BitFlag::<Word>::new(0) | BitFlag::new(1);
    raw | flags.mask()
}
