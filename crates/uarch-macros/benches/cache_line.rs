use std::hint::black_box;
use uarch_macros::cache_line;

#[cache_line]
struct Line {
    words: [u64; 8],
}

pub(super) fn run(value: u64) -> u64 {
    let line = Line { words: [value; 8] };
    black_box(&line).words[0]
}
