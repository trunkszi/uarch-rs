use uarch_core::bits::Bitmap64;

pub(super) fn run(raw: u64) -> u64 {
    let mut map = Bitmap64::new(raw);
    u64::from(map.pop_lowest_one().unwrap_or(64)) ^ map.lowest_one_bit()
}
