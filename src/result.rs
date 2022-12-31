//! This module contains functions that provide easier
//! to use workflows for the co-routine if R is Result
//! This can be implemented outside the crate, but are here for convenience.
use super::*;

/// Process the Ok value, or short-circuit
///
/// This is basically result semantics
pub fn bind_ok<'a, I, O, A, B, E, F>(
    co: Coroutine<'a, I, O, Result<A, E>>,
    f: F,
) -> Coroutine<'a, I, O, Result<B, E>>
where
    F: FnOnce(A) -> Coroutine<'a, I, O, Result<B, E>> + Send + 'a,
{
    bind(co, |r| match r {
        Ok(ok) => f(ok),
        Err(err) => result(Err(err)),
    })
}

/// Process the err value, or continue with ok
///
/// Allows you to use coroutines to tranform and routine
/// that is carrying error forward.
/// This is particularly useful to emit outputs in response
/// to that error
pub fn bind_err<'a, I, O, A, E1, E2, F>(
    co: Coroutine<'a, I, O, Result<A, E1>>,
    f: F,
) -> Coroutine<'a, I, O, Result<A, E2>>
where
    F: FnOnce(E1) -> Coroutine<'a, I, O, Result<A, E2>> + Send + 'a,
{
    bind(co, |r| match r {
        Ok(ok) => result(Ok(ok)),
        Err(err) => f(err),
    })
}

/// Convert E1 into E2
///
/// Sugar over bind
pub fn map_err<'a, I, O, A, E1, E2, F>(
    co: Coroutine<'a, I, O, Result<A, E1>>,
    f: F,
) -> Coroutine<'a, I, O, Result<A, E2>>
where
    F: FnOnce(E1) -> E2 + Send + 'a,
{
    bind_err(co, |e| result(Err(f(e))))
}

/// Run a child subroutine, with routines that map inputs and outputs
///
/// This is special over the core-lib version, as we work in results
/// a err in input procesisng, or output processing, will shortcircuit the
/// whole computation into the err value.
///
/// It is unexpected that an output routine should fail, but provided
/// for convenience
pub fn subroutine_result<'a, I, O, SI, SO, FI, FO, R, E>(
    on_input: FI,
    on_output: FO,
    subroutine: Coroutine<'a, SI, SO, Result<R, E>>,
) -> Coroutine<'a, I, O, Result<R, E>>
where
    FI: Fn() -> Coroutine<'a, I, O, Result<SI, E>> + Send + 'a,
    FO: Fn(SO) -> Coroutine<'a, I, O, Option<E>> + Send + 'a,
    SO: Send,
    R: Send,
    E: Send,
{
    match run_step(subroutine) {
        StepResult::Done(r) => result(r),
        StepResult::Yield { output, next } => {
            let output = on_output(output);
            let resolve = move |error: Option<E>| match error {
                None => subroutine_result(on_input, on_output, *next),
                Some(e) => result(Err(e)),
            };
            bind(output, resolve)
        }
        StepResult::Next(n) => {
            let rout = on_input();
            let cont = move |value: Result<SI, E>| match value {
                Ok(value) => {
                    let next = n(value);
                    subroutine_result(on_input, on_output, next)
                }
                Err(err) => result(Err(err)),
            };
            bind(rout, cont)
        }
    }
}
