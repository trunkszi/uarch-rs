use std::hint::black_box;
use uarch_core::layout::Isolated;

pub(super) fn run(mut raw: u64) -> u64 {
    let mut isolated = Isolated::new(raw);
    for _ in 0..64 {
        *isolated.get_mut() = black_box(raw).wrapping_add(1);
        raw = *black_box(isolated.get());
    }
    isolated.into_inner()
}
