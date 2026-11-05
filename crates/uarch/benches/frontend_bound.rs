use std::hint::black_box;

#[inline(never)]
fn outlined(raw: u64) -> u64 {
    raw.rotate_left(7).wrapping_add(3)
}
#[inline(always)]
fn inlined(raw: u64) -> u64 {
    raw.rotate_left(7).wrapping_add(3)
}
pub(super) fn bad(data: &[u64; 256]) -> u64 {
    data.iter()
        .fold(0u64, |sum, raw| sum.wrapping_add(outlined(black_box(*raw))))
}
pub(super) fn good(data: &[u64; 256]) -> u64 {
    data.iter()
        .fold(0u64, |sum, raw| sum.wrapping_add(inlined(black_box(*raw))))
}
