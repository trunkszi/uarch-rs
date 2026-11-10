mod bitmap64;
mod error;
mod field;
mod flag;
mod ops;
mod patch;
mod pattern;
mod word_macro;

#[derive(Clone, Copy)]
#[repr(transparent)]
struct Word(u64);

pub(crate) fn run() -> u64 {
    let mut raw = std::hint::black_box(0x1234_5678_9abc_def0);
    for _ in 0..64 {
        raw = field::run(std::hint::black_box(raw));
        raw = flag::run(std::hint::black_box(raw));
        raw ^= bitmap64::run(std::hint::black_box(raw));
        raw ^= ops::run(std::hint::black_box(raw));
        raw ^= pattern::run(std::hint::black_box(raw));
        raw = patch::run(std::hint::black_box(raw));
        raw = word_macro::run(std::hint::black_box(raw));
    }
    std::hint::black_box(raw)
}
