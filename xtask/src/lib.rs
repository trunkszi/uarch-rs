#[allow(unused_imports)]
use uarch::prelude::*;

pub mod audit;
mod cli;
pub mod codegen;
mod error;
mod hardware;
pub mod lint;
mod process;

pub use cli::run;
pub use error::{Error, Result};
