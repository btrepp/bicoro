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
/// As the child subroutine may have different inputs and outputs
/// you need to specify how to convert between these worlds.
/// This is done by providing functions that yield coroutines that can be
/// ran in the 'parent' context. One that is ran whenever the child needs an input
/// and one that is ran when the child produces an output
///
/// These can simply proxy through to the parents context, or
/// perform more processing. Both still retain inputs and outputs
/// so they may call a parents recieve multiple times
/// ```
/// use bicoro::*;
///
/// /* Types in this example are written explicitly to highlight what they are
///    It should be possible to use more compact forms
/// */
///
/// // Reads a value, echos it, then returns 0u8
/// let child: Coroutine<i64,i64,u8> = receive().and_then(send).and_then(|()| result(0u8));
///
/// // an example where we require two inputs from the parent context, to one on the child
/// // in this case we add the two i32's together to form an i64
/// let on_input = | | -> Coroutine<i32,_,i64> {
///     bind(receive(),|a| {
///         map(receive(),move |b| {
///             (a as i64+b as i64)
///         })
///     })
/// };
///
/// // parent expects strings on it's output channel so we convert
/// // note: type gaps used here as it must line up with the parent context types
/// // also means it's possible to read from the parent context when producing output
/// let on_output = | o:i64 | -> Coroutine<_,String,()> {
///     send(o.to_string())
/// };
///
/// // run_child maps the child routine into the parenst 'world'
/// // we can see the type of this is the parent coroutine
/// let parent : Coroutine<i32,String,u8> = run_child(on_input,on_output,child);
/// ```
pub fn run_child<
    'a,
    Input: 'a,
    Output: 'a,
    ChildInput: 'a,
    ChildOutput: 'a,
    OnInput: 'a,
    OnOutput: 'a,
    Result: 'a,
>(
    on_input: OnInput,
    on_output: OnOutput,
    child: Coroutine<'a, ChildInput, ChildOutput, Result>,
) -> Coroutine<'a, Input, Output, Result>
where
    OnInput: Fn() -> Coroutine<'a, Input, Output, ChildInput>,
    OnOutput: Fn(ChildOutput) -> Coroutine<'a, Input, Output, ()>,
{
    match run_step(child) {
        StepResult::Done(r) => result(r),
        StepResult::Yield { output, next } => {
            let output = on_output(output);
            bind(output, move |()| run_child(on_input, on_output, *next))
        }
        StepResult::Next(n) => on_input().and_then(move |i| {
            let next = n(i);
            run_child(on_input, on_output, next)
        }),
    }
}
