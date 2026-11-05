//! Typed, allocation-free bit queries and updates.
mod bitmap64;
mod error;
mod field;
mod flag;
mod ops;
mod patch;
mod pattern;
mod word_macro;

pub use crate::bitword;
pub use bitmap64::{Bitmap64, Ones};
pub use error::BitError;
pub use field::{BitField, BitValue};
pub use flag::{BitFlag, FlagClear, FlagSet};
pub use patch::BitPatch;
pub use pattern::BitPattern;
