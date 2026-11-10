//! Branchless predicates, register select, and independent-lane reductions.

mod hints;
mod ilp;
mod predicate;
mod reduce;
mod select;

pub use hints::{cold_path, likely, unlikely};
pub use ilp::reduce_lanes;
pub use predicate::{MaskWord, Predicate, predicate};
pub use select::RegisterSelectable;

#[doc(inline)]
pub use crate::{ilp_all, ilp_any, reduce_four_way};
