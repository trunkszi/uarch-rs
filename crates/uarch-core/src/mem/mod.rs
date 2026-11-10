//! Slot names and power-of-two pools with a reserved sentinel slot.

mod error;
mod pool;
mod pool_index;
mod prefetch;

pub use error::PoolError;
pub use pool::Pool;
pub use pool_index::PoolIndex;
pub use prefetch::{L1, L2, L3, Locality, NonTemporal, prefetch_read};
