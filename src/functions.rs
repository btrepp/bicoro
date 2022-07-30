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
    F: FnOnce(A) -> B + Send + Sync,
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
    OnInput: Fn() -> Coroutine<'a, Input, Output, ChildInput> + Send + Sync,
    OnOutput: Fn(ChildOutput) -> Coroutine<'a, Input, Output, ()> + Send + Sync,
    ChildOutput: Send + Sync,
    Result: Send + Sync,
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

/// Transforms the input of coroutine A into B
///
/// This requires a coroutine that can map B inputs
/// into a, as this is similar to running 'co'
/// in the context of the output
///
/// This is a specialization of run_child
///
/// TLDR; change Input with the inform transform function
pub fn transform_input<'a, Input: 'a, InputNested: 'a, Output: 'a, Transform: 'a, Result: 'a>(
    co: Coroutine<'a, InputNested, Output, Result>,
    transform: Transform,
) -> Coroutine<'a, Input, Output, Result>
where
    Transform: Fn(Input) -> Coroutine<'a, Input, Output, InputNested> + Send + Sync + Clone,
    Output: Send + Sync,
    Result: Send + Sync,
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
pub fn transform_output<'a, Input: 'a, OutputA: 'a, OutputB: 'a, Transform: 'a, Result: 'a>(
    co: Coroutine<'a, Input, OutputA, Result>,
    transform: Transform,
) -> Coroutine<'a, Input, OutputB, Result>
where
    Transform: Fn(OutputA) -> Coroutine<'a, Input, OutputB, OutputB> + Send + Sync,
    Result: Send + Sync,
    OutputA: Send + Sync,
{
    let on_input = || receive();
    let on_output = move |o: OutputA| bind(transform(o), send);
    run_child(on_input, on_output, co)
}

/// Runs recieve until f returns some
/// 
/// This is ran inside it's own coroutine, so
/// you can call send inside it. 
pub fn recieve_until<'a, Input: 'a, Output: 'a, Result: 'a, F: 'a>(
    f: F,
) -> Coroutine<'a, Input, Output, Result>
where
    F: Fn(Input) -> Coroutine<'a, Input, Output, Option<Result>> + Send + Sync + Clone,
{    
    let input = bind(receive(), f.clone());
    bind(input,| opt| {
        match opt {
            Some(v) => result(v),
            None => recieve_until(f)
        }
    })
}

/// Runs two coroutines sequentially
/// 
/// This will run first until it completes, then second afterwards
/// until it completes
/// Returns both return results tupled together
pub fn chain<'a, I: 'a, O: 'a, R1: 'a, R2:'a>(
    first: Coroutine<'a, I, O, R1>,
    second: Coroutine<'a, I, O, R2>,
) -> Coroutine<'a, I, O, (R1,R2)>
where
    O: Send + Sync,
    R1: Send + Sync,
    R2: Send + Sync
{
    bind(first, |a| map(second,|b|(a,b)))
}

/// Runs a routine before the second routine
/// 
/// Do must return a unit type, when it completes
/// then routine will be ran see also chain
pub fn before<'a, I: 'a, O: 'a, R: 'a>(
    r#do: Coroutine<'a, I, O, ()>,
    routine: Coroutine<'a, I, O, R>,
) -> Coroutine<'a, I, O, R>
where
    O: Send + Sync,
    R: Send + Sync,
{
    map(chain(r#do, routine), |((),b)| b)
}

/// Runs a routine after this routine
/// 
/// Do must return a unit type, it will be ran after routine
/// is completed, see also chain
pub fn after<'a, I: 'a, O: 'a, R: 'a>(    
    routine: Coroutine<'a, I, O, R>,
    r#do: Coroutine<'a, I, O, ()>,
) -> Coroutine<'a, I, O, R>
where
    O: Send + Sync,
    R: Send + Sync,
{
    map(chain(routine, r#do), |(a,())| a)
}