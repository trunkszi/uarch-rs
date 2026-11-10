mod assertions;
mod isolated;
mod traits;

pub(crate) fn run() -> u64 {
    isolated::run(std::hint::black_box(1))
}

mod cache_aligned;
