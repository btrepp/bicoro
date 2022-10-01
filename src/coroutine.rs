/// A structure describing a co-routine supporting sends (inputs),
/// yields (outputs), and a final termination (result)
///
/// This requires something else to execute it.
/// a send or a yield will 'pause' the coroutine until the executor provides
/// or consumes the output.
///
/// This structure is useful as you can logically describe a workflow,
/// but leave the 'plumbing' of IO to later.
///
/// A simple case of input an output would be using enums.
/// So that you can send and recieve different messages
///
/// This is represented as a monad <https://en.wikipedia.org/wiki/Monad_(functional_programming)>
pub struct Coroutine<'a, Input, Output, Result> {
    resume: CoroutineState<'a, Input, Output, Result>,
}

// I believe this is safe, as the closures are send and fnonce.
// the coroutine can only be changed by move, so multiple aliases
// shouldn't be a problem. Would love to have feedback on this
unsafe impl<'a, I, O, R> Sync for Coroutine<'a, I, O, R> {}

/// The internal state of the machine
enum CoroutineState<'a, Input: 'a, Output: 'a, Result: 'a> {
    /// The coroutine is paused waiting for some-input
    Await(Box<dyn FnOnce(Input) -> Coroutine<'a, Input, Output, Result> + Send + 'a>),
    /// The coroutine is paused, waiting for a output to be consumed
    Yield(Output, Box<Coroutine<'a, Input, Output, Result>>),
    /// The coroutine is completed
    Done(Result),
}

/// Return/unit. Creates a result of the supplied value
///
/// This lifts the value into the coroutine 'world'
/// ```
/// use bicoro::*;
/// let co :Coroutine<(),(),i32> = result(1);
/// ```
pub fn result<'a, I, O, R>(r: R) -> Coroutine<'a, I, O, R> {
    let resume = CoroutineState::Done(r);
    Coroutine { resume }
}

/// Suspend this coroutine until an input arrives with a function
///
/// The function f, will be called on this input
/// see also: recieve()
/// ```
/// use bicoro::*;
/// let co :Coroutine<i32,(),String> = suspend(Box::new(|input:i32| result(input.to_string())));
/// ```
pub fn suspend<'a, I, O, R, F>(f: F) -> Coroutine<'a, I, O, R>
where
    F: FnOnce(I) -> Coroutine<'a, I, O, R> + Send + 'a,
{
    let closure = Box::new(f);
    let resume = CoroutineState::Await(closure);
    Coroutine { resume }
}

/// Yields a value to the executor
///
/// This pauses until the executor uses it
/// ```
/// use bicoro::*;
/// let co :Coroutine<(),&str,()> = send("hello");
/// ```
pub fn send<'a, I, O>(o: O) -> Coroutine<'a, I, O, ()> {
    let resume = CoroutineState::Yield(o, Box::new(result(())));
    Coroutine { resume }
}

/// Chain outputs together.
///
/// This allows the results from one item
/// to flow into the next one. This can call any arbitrary co-routine
/// The next routine needs the same inputs and outputs, but can change it's result
/// This is equivalent to and_then for the Future type.
/// ```
/// use bicoro::*;
/// // reads two input values and adds them.
/// let co:Coroutine<i32,(),i32> = bind(receive(),|a:i32| bind(receive(), move |b:i32| result(a+b)));
/// ```
pub fn bind<'a, I, O, A, B, F>(m: Coroutine<'a, I, O, A>, f: F) -> Coroutine<'a, I, O, B>
where
    F: FnOnce(A) -> Coroutine<'a, I, O, B> + Send + 'a,
{
    match m.resume {
        CoroutineState::Done(ra) => f(ra),
        CoroutineState::Yield(output, ra) => {
            let state = bind(*ra, f);
            let resume = CoroutineState::Yield(output, Box::new(state));
            Coroutine { resume }
        }
        CoroutineState::Await(mf) => {
            let state = |input: I| -> Coroutine<I, O, B> {
                let next = mf(input);
                bind(next, f)
            };
            suspend(state)
        }
    }
}

/// A step wise evalution of the coroutine
///
/// this allows you to 'iterate' through until you need to provide input
/// or to observe yields.
///
/// In the cause of input, a function is returned, it's expected
/// the executor will call this with the input.
///
/// In the cause of output, a tuple of the output and the remaining coroutine
/// is returned.
///
/// In the coroutine is finished, it will be in the done case, so the return
/// value can be extracted

pub enum StepResult<'a, Input, Output, Result> {
    /// The final value
    Done(Result),
    /// We have output to give to the executor
    Yield {
        /// The current output being provided to the executor
        output: Output,
        /// The remaining coroutine to process
        next: Box<Coroutine<'a, Input, Output, Result>>,
    },
    /// The coroutine is suspended, awaiting input
    Next(Box<dyn FnOnce(Input) -> Coroutine<'a, Input, Output, Result> + Send + 'a>),
}

/// Runs a single step in the coroutine.
///
/// This returns the step result. Used to interpret/run the coroutine
/// ```
/// use bicoro::*;
/// let co: Coroutine<i32,(),i32> = receive();
/// let sr = run_step(co);
/// assert!(matches!(sr, StepResult::Next(_)));
/// ```
pub fn run_step<'a, I, O, R>(routine: Coroutine<'a, I, O, R>) -> StepResult<'a, I, O, R> {
    match routine.resume {
        CoroutineState::Done(result) => StepResult::Done(result),
        CoroutineState::Await(run) => StepResult::Next(run),
        CoroutineState::Yield(output, next) => StepResult::Yield { output, next },
    }
}
