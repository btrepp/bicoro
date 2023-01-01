#![doc = include_str!("../README.md")]

mod chain;
mod compat;
mod cooperate;
mod coroutine;
mod dispatch;
mod functions;
mod option;
mod result;
mod routed;
pub use crate::compat::*;
pub use chain::*;
pub use cooperate::*;
pub use coroutine::*;
pub use dispatch::*;
pub use functions::*;
pub use option::*;
pub use result::*;
pub use routed::*;
pub mod executor;
pub mod iterator;
