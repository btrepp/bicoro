#![doc = include_str!("../README.md")]

mod chain;
mod compat;
mod coroutine;
mod dispatch;
mod functions;

pub use crate::compat::*;
pub use chain::*;
pub use coroutine::*;
pub use dispatch::*;
pub use functions::*;

pub mod executor;
pub mod iterator;
