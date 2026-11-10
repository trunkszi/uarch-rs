use uarch_macros::{cold_sink, hot_inline};

#[cold_sink]
fn cold(value: u64) -> u64 {
    value.wrapping_add(7)
}

#[hot_inline]
pub(super) fn run(value: u64) -> u64 {
    if value == u64::MAX {
        cold(value)
    } else {
        value.wrapping_add(1)
    }
}
