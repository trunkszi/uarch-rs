use uarch_macros::{patch, pattern};

uarch::bitword! { pub struct Word(u64) { [0] pub READY, [1] pub BUSY, [8..=11] pub STATE:u8, } }

pub(super) fn run(value: u64) -> u64 {
    let pattern = pattern!(Word {READY & !BUSY & STATE == 3});
    let patch = patch!(Word {
        READY: true,
        BUSY: false,
        STATE: 3
    });
    Word::new(value).patch(patch).raw() ^ pattern.expected()
}
