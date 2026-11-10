use uarch_macros::{cold_sink, hot_inline};

#[cold_sink]
fn cold(value: u64) -> u64 {
    value + 1
}

#[hot_inline]
fn hot(value: u64) -> u64 {
    cold(value)
}

fn main() {
    assert_eq!(hot(2), 3);
}
