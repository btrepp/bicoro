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
/// let parent : Coroutine<i32,String,u8> = run_child(on_input,on_output,child);
/// ```
pub fn run_child<'a, Input, Output, ChildInput, ChildOutput, OnInput, OnOutput, Result>(
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
            bind(output, move |()| run_child(on_input, on_output, *next))
        }
        StepResult::Next(n) => {
            let rout = on_input();
            let cont = move |i| {
                let next = n(i);
                run_child(on_input, on_output, next)
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
/// This is a specialization of run_child
///
/// TLDR; change Input with the inform transform function
pub fn transform_input<'a, Input, InputNested, Output, Transform, Result>(
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
    let on_output = |o: Output| send(o);
    run_child(on_input, on_output, co)
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
pub fn transform_output<'a, Input, OutputA, OutputB, Transform, Result>(
    co: Coroutine<'a, Input, OutputA, Result>,
    transform: Transform,
) -> Coroutine<'a, Input, OutputB, Result>
where
    Transform: Fn(OutputA) -> Coroutine<'a, Input, OutputB, OutputB> + Send + 'a,
    Result: Send,
    OutputA: Send,
{
    let on_input = || receive();
    let on_output = move |o: OutputA| bind(transform(o), send);
    run_child(on_input, on_output, co)
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
pub fn void<'a, I, O, A>(co: Coroutine<'a, I, O, A>) -> Coroutine<'a, I, O, ()> {
    map(co, |_| ())
}
