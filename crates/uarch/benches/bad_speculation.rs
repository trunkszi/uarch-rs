use std::hint::black_box;
use uarch::{arithmetic::PowerOfTwo, flow::reduce_lanes};

const LOOKUP: [u64; 16] = [3, 19, 7, 43, 31, 11, 53, 17, 23, 61, 37, 5, 47, 13, 41, 29];
// Zero padding makes rejected indexes safe to load without a conditional edge.
const PADDED: [u64; 32] = {
    let mut table = [0; 32];
    let mut index = 0;
    while index < LOOKUP.len() {
        table[index] = LOOKUP[index];
        index += 1;
    }
    table
};
pub(super) const WORK_ITEMS: usize = 16 * 256;

#[inline(never)]
pub(super) fn bad(data: &[u64; 256]) -> u64 {
    let table = black_box(&LOOKUP);
    let mut sum = 0u64;
    for shift in 0..16 {
        for raw in data {
            let raw = black_box(*raw);
            let index = ((raw >> shift) & 31) as usize;
            // The out-of-range load must stay behind its data-dependent guard.
            let value = if index < table.len() { table[index] } else { 0 };
            sum = sum.wrapping_add(black_box(value));
        }
    }
    sum
}

#[inline(never)]
pub(super) fn good(data: &[u64; 256]) -> u64 {
    let table = black_box(&PADDED);
    let mut sum = 0u64;
    for shift in 0..16 {
        let subtotal = reduce_lanes::<4, _, _>(
            data,
            0u64,
            0u64,
            |acc, raw| {
                let raw = black_box(*raw);
                let index = PowerOfTwo::<5>::wrap_usize((raw >> shift) as usize);
                acc.wrapping_add(black_box(table[index]))
            },
            u64::wrapping_add,
        );
        sum = sum.wrapping_add(subtotal);
    }
    sum
}
