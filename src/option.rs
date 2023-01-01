//! This module contains functions that provide easier
//! to use workflows for the co-routine if R is Option<A>
//! This can be implemented outside the crate, but are here for convenience.
use super::*;

/// Process the Some value, or short-circuit
///
/// This is basically option semantics
/// Works with any C that can be turned into
/// option a
pub fn bind_some<'a, I, O, A, B, C, F>(
    co: Coroutine<'a, I, O, C>,
    f: F,
) -> Coroutine<'a, I, O, Option<B>>
where
    F: FnOnce(A) -> Coroutine<'a, I, O, Option<B>> + Send + 'a,
    C: Into<Option<A>>,
{
    bind(co, |r| match r.into() {
        Some(ok) => f(ok),
        None => result(None),
    })
}

/// Maps the inner value
///
/// Sugar over bind_some
pub fn map_some<'a, I, O, A, B, C, F>(
    co: Coroutine<'a, I, O, C>,
    f: F,
) -> Coroutine<'a, I, O, Option<B>>
where
    F: FnOnce(A) -> B + Send + 'a,
    C: Into<Option<A>>,
{
    bind_some(co, |a| result(Some(f(a))))
}
