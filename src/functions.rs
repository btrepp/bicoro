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
pub fn receive<'a, I, O>() -> Coroutine<'a, I, O, I> {
    suspend(Box::new(result))
}

/// Map the inner type of the coroutine
///
/// This is sugar of bind and result
/// ```
/// use bicoro::*;
/// let co :Coroutine<i32,(),String> = map(receive(), |a| a.to_string());
/// ```
pub fn map<'a, I, O, A, B, F>(co: Coroutine<'a, I, O, A>, map: F) -> Coroutine<'a, I, O, B>
where
    F: FnOnce(A) -> B + Send + 'a,
{
    bind(co, move |a| result(map(a)))
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
/// let parent : Coroutine<i32,String,u8> = subroutine(on_input,on_output,child);
/// ```
pub fn subroutine<'a, Input, Output, ChildInput, ChildOutput, OnInput, OnOutput, Result>(
    on_input: OnInput,
    on_output: OnOutput,
    child: Coroutine<'a, ChildInput, ChildOutput, Result>,
) -> Coroutine<'a, Input, Output, Result>
where
    OnInput: Fn() -> Coroutine<'a, Input, Output, ChildInput> + Send + 'a,
    OnOutput: Fn(ChildOutput) -> Coroutine<'a, Input, Output, ()> + Send + 'a,
    ChildOutput: Send,
    Result: Send,
{
    match run_step(child) {
        StepResult::Done(r) => result(r),
        StepResult::Yield { output, next } => {
            let output = on_output(output);
            bind(output, move |()| subroutine(on_input, on_output, *next))
        }
        StepResult::Next(n) => {
            let rout = on_input();
            let cont = move |i| {
                let next = n(i);
                subroutine(on_input, on_output, next)
            };
            bind(rout, cont)
        }
    }
}

/// Transforms the input of coroutine A into B
///
/// This requires a coroutine that can map B inputs
/// into a, as this is similar to running 'co'
/// in the context of the output
///
/// This is a specialization of subroutine
///
/// TLDR; change Input with the inform transform function
pub fn intercept_input<'a, Input, InputNested, Output, Transform, Result>(
    co: Coroutine<'a, InputNested, Output, Result>,
    transform: Transform,
) -> Coroutine<'a, Input, Output, Result>
where
    Transform: Fn(Input) -> Coroutine<'a, Input, Output, InputNested> + Send + Clone + 'a,
    Result: Send,
    Output: Send,
{
    let on_input =
        move || -> Coroutine<Input, Output, InputNested> { bind(receive(), transform.clone()) };
    subroutine(on_input, send, co)
}

/// Takes a coroutine that wants a, and 'lifts' or maps it into B
///
/// Note: This is somewhat backwards than you might expect map to be
/// but is the closest you can get.
pub fn map_input<'a, InputA, InputB, Output, Result, MapFn>(
    co: Coroutine<'a, InputB, Output, Result>,
    map_fn: MapFn,
) -> Coroutine<'a, InputA, Output, Result>
where
    MapFn: Fn(InputA) -> InputB + Send + Clone + 'a,
    Result: Send,
    Output: Send,
{
    intercept_input(co, move |a| result(map_fn(a)))
}

/// Transforms the output of coroutine A into B
///
/// This requires a coroutine that can map B outputs
/// into a, as this is similar to running 'co'
/// in the context of the output
///
/// This is a specialization of run_child
///
/// TLDR; change output with the output transform function
pub fn intercept_output<'a, Input, OutputA, OutputB, Transform, Result>(
    co: Coroutine<'a, Input, OutputA, Result>,
    transform: Transform,
) -> Coroutine<'a, Input, OutputB, Result>
where
    Transform: Fn(OutputA) -> Coroutine<'a, Input, OutputB, ()> + Send + 'a,
    Result: Send,
    OutputA: Send,
{
    subroutine(receive, transform, co)
}

/// Takes a coutine outputting A, and makes it output B
///
/// This is a specialization of intercept_output, when we
/// don't need to have any affects
pub fn map_output<'a, Input, OutputA, OutputB, Result, MapFn>(
    co: Coroutine<'a, Input, OutputA, Result>,
    map_fn: MapFn,
) -> Coroutine<'a, Input, OutputB, Result>
where
    MapFn: Fn(OutputA) -> OutputB + Send + 'a,
    OutputA: Send,
    Result: Send,
{
    intercept_output(co, move |o| send(map_fn(o)))
}

/// Runs recieve until f returns some
///
/// This is ran inside it's own coroutine, so
/// you can call send inside it.
pub fn recieve_until<'a, Input, Output, Result, F>(f: F) -> Coroutine<'a, Input, Output, Result>
where
    F: Fn(Input) -> Coroutine<'a, Input, Output, Option<Result>> + Send + Clone + 'a,
{
    let input = bind(receive(), f.clone());
    bind(input, |opt| match opt {
        Some(v) => result(v),
        None => recieve_until(f),
    })
}

/// Use to either consume this input or re-emit as an output
///
/// This is useful when we want to intercept or transform
/// messages, but I may not be convertible to IA
/// In this case we emit the transform IB back out, hoping
/// that another routine may deal with it
pub fn receive_or_skip<'a, I, O, R, F, IA, IB>(
    co: Coroutine<'a, IA, O, R>,
    sel: F,
) -> Coroutine<'a, I, UnicastSelect<IB, O>, R>
where
    F: Fn(I) -> UnicastSelect<IA, IB> + Send + 'a,
    O: Send,
    R: Send,
{
    match run_step(co) {
        StepResult::Done(r) => result(r),
        StepResult::Yield { output, next } => {
            let output = send(UnicastSelect::Right(output));
            let next = |()| receive_or_skip(*next, sel);
            bind(output, next)
        }
        StepResult::Next(next) => {
            let on_input = |input: I| match sel(input) {
                UnicastSelect::Left(a) => {
                    let next = next(a);
                    receive_or_skip(next, sel)
                }
                UnicastSelect::Right(b) => {
                    let output = send(UnicastSelect::Left(b));
                    let next = suspend(next);
                    let next = |()| receive_or_skip(next, sel);
                    bind(output, next)
                }
            };
            bind(receive(), on_input)
        }
    }
}

/// Runs two coroutines sequentially
///
/// This will run first until it completes, then second afterwards
/// until it completes
/// Returns both return results tupled together
pub fn tuple<'a, I, O, R1, R2>(
    first: Coroutine<'a, I, O, R1>,
    second: Coroutine<'a, I, O, R2>,
) -> Coroutine<'a, I, O, (R1, R2)>
where
    R1: Send,
    R2: Send,
    O: Send,
{
    bind(first, move |a| map(second, move |b| (a, b)))
}

/// Runs a routine before the second routine
///
/// Result of the first routine is ignored, second is returned
/// if you need both results, use tuple
pub fn right<'a, I, O, A, B>(
    left: Coroutine<'a, I, O, A>,
    right: Coroutine<'a, I, O, B>,
) -> Coroutine<'a, I, O, B>
where
    O: Send,
    A: Send,
    B: Send,
{
    map(tuple(left, right), |(_, b)| b)
}

/// Runs a routine before the second routine
///
/// Result of the first routine is returned, second is ignored
/// if you need both results, use tuple
pub fn left<'a, I, O, A, B>(
    left: Coroutine<'a, I, O, A>,
    right: Coroutine<'a, I, O, B>,
) -> Coroutine<'a, I, O, A>
where
    O: Send,
    A: Send,
    B: Send,
{
    map(tuple(left, right), |(a, _)| a)
}

/// Converts the return result to the unit type
///
/// Useful when you don't care what the coroutine occurs
/// mainly after its effects
pub fn void<I, O, A>(co: Coroutine<I, O, A>) -> Coroutine<I, O, ()> {
    map(co, |_| ())
}

/// Use this input for the next input
///
/// Allows us to provide a single input to the coroutine
/// This will be provided the next time the co asks for an input
/// the input 'request' wont propagate.
/// If co is finished, the input is ignored
pub fn inject<I, O, R>(input: I, co: Coroutine<I, O, R>) -> Coroutine<I, O, R>
where
    R: Send,
    O: Send,
{
    match run_step(co) {
        StepResult::Done(r) => result(r),
        StepResult::Yield { output, next } => {
            let out = send(output);
            let next = inject(input, *next);
            right(out, next)
        }
        StepResult::Next(next) => next(input),
    }
}

/// Get the next output of the coroutine
///
/// This captures the next output, allowing it to be inspected instead of
/// emitted. Returns the observed value and the remaining coroutine.
/// If the coroutine has ended, the observed value is none.
pub fn observe<I, O, R>(
    co: Coroutine<I, O, R>,
) -> Coroutine<I, O, (Option<O>, Coroutine<I, O, R>)> {
    match run_step(co) {
        StepResult::Done(r) => result((None, result(r))),
        StepResult::Yield { output, next } => result((Some(output), *next)),
        StepResult::Next(next) => {
            let input = |input| {
                let co = next(input);
                observe(co)
            };
            suspend(input)
        }
    }
}
