use uarch_core::bitword;

bitword! {
    pub struct BenchWord(u64) {
        [0] pub READY,
        [32..=63] pub COUNT: u32,
    }
}

pub(super) fn run(raw: u64) -> u64 {
    let mut word = BenchWord::new(raw).wrapping_add_top_field(BenchWord::COUNT, 1);
    word |= BenchWord::READY;
    word.raw()
}
