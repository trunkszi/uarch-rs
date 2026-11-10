//! Invariant division, 32-bit remainder, and power-of-two wrapping.

mod fast_divisor_u32;
mod fast_divisor_u64;
mod power_of_two;

pub use fast_divisor_u32::FastDivisorU32;
pub use fast_divisor_u64::FastDivisorU64;
pub use power_of_two::PowerOfTwo;
