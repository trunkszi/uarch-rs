#![no_std]

pub mod gate;
pub use gate::{AbiRule, GATES, Gate, Instruction, McaBound};

pub mod arch;
pub mod arithmetic;
pub mod bits;
pub mod flow;
pub mod layout;
pub mod mem;

pub use arithmetic::{fast_divisor_u64_pow2_rem, fast_divisor_u64_rem};
