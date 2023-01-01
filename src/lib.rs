#![doc = include_str!("../README.md")]

mod chain;
mod compat;
mod coroutine;
mod dispatch;
mod functions;
mod result;
mod routed;

pub use crate::compat::*;
pub use chain::*;
pub use coroutine::*;
pub use dispatch::*;
pub use functions::*;
pub use result::*;
pub use routed::*;

pub mod executor;
pub mod iterator;
