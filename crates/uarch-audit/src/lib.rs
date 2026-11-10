//! Shared, synchronous validation of compiler artifacts and measurement evidence.
#[allow(unused_imports)]
use uarch::prelude::*;

mod error;
pub mod gem5_stats;
pub use error::{Error, Result};
pub mod allocator;
pub mod artifacts;
pub mod hardware;
pub mod manifest;
pub mod process;
