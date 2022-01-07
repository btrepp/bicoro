//! This module contains functions that provide easier
//! to use workflows for the co-routine.
//! This can be implemented outside the crate, but are here for convenience.

use super::*;

/// Suspend this coroutine until an input arrives
///
/// The result can be bound. Eg. the below reads a int and converts it into a string
/// ```
/// use bicoro::*;
/// let co :Coroutine<i32,(),i32> = receive();
/// ```
pub fn receive<'a, I: 'a, O: 'a>() -> Coroutine<'a, I, O, I> {
    suspend(Box::new(result))
}

/// Map the inner type of the coroutine
///
/// This is sugar of bind and result
/// ```
/// use bicoro::*;
/// let co :Coroutine<i32,(),String> = map(receive(), |a| a.to_string());
/// ```
pub fn map<'a, I: 'a, O: 'a, A: 'a, B: 'a, F: 'a>(
    co: Coroutine<'a, I, O, A>,
    map: F,
) -> Coroutine<'a, I, O, B>
where
    F: FnOnce(A) -> B,
{
    bind(co, |a| result(map(a)))
}

/// Runs a subroutine and converts it to the hosted type
///
/// Converts a different subroutine to run inside this one.
/// Note you need to be able to convert from inputs and  outputs.
/// converting inputs may not be simple, so it can be it's own co-routine
/// ```
/// use bicoro::*;
/// // Reads a value, echos it, then returns 0u8
/// let child: Coroutine<i64,i64,u8> = receive().and_then(send).and_then(|()| result(0u8));
/// // all parent inputs are valid for the child here, so we just convert
/// let map_input = | i:i32 | result(i as i64);
/// // parent expects strings on it's output channel so we convert
/// let map_output = | o:i64 | o.to_string();
///
/// // run_child maps the child routine into the parenst 'world'
/// let parent : Coroutine<i32,String,u8> = run_child(map_input,map_output,child);
/// ```
pub fn run_child<
    'a,
    Input: 'a,
    Output: 'a,
    ChildInput: 'a,
    ChildOutput: 'a,
    MapInputFunction: 'a,
    MapOutputFunction: 'a,
    Result: 'a,
>(
    map_input: MapInputFunction,
    map_output: MapOutputFunction,
    child: Coroutine<'a, ChildInput, ChildOutput, Result>,
) -> Coroutine<'a, Input, Output, Result>
where
    MapInputFunction: Fn(Input) -> Coroutine<'a, Input, Output, ChildInput> + Copy,
    MapOutputFunction: Fn(ChildOutput) -> Output,
{
    match run_step(child) {
        StepResult::Done(r) => result(r),
        StepResult::Yield { output, next } => {
            let output = map_output(output);
            bind(send(output), move |()| {
                run_child(map_input, map_output, *next)
            })
        }
        StepResult::Next(n) => {
            receive()
                .and_then(move |i: Input| map_input(i))
                .and_then(move |i| {
                    let next = n(i);
                    run_child(map_input, map_output, next)
                })
        }
    }
}
