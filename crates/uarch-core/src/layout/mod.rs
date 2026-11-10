//! Layout proofs and cache isolation.
mod assertions;
mod isolated;
mod traits;

pub use assertions::{assert_copy, assert_no_padding, assert_register_leaf};
pub use isolated::Isolated;
pub use traits::{NoPadding, RegisterLeaf};

mod cache_aligned;
pub use crate::assert_layout;
pub use cache_aligned::CacheAligned;
