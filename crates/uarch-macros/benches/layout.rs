use std::hint::black_box;
use uarch_macros::isolated;

#[isolated(full)]
struct Block {
    words: [u64; 16],
}

pub(super) fn run(value: u64) -> u64 {
    let block = Block { words: [value; 16] };
    black_box(&block).words[15]
}
