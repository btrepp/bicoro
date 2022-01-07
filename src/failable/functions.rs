use super::*;

/// Just like coroutine send, but sugared for result coroutines
///
/// see (function@send)
pub fn send<'a, I: 'a, O: 'a, E: 'a>(o: O) -> ResultCoroutine<'a, I, O, (), E> {
    lift(crate::map(crate::send(o), Result::Ok))
}

/// Just like coroutine receive, but sugared for result coroutines
///
/// see (function@receive)
pub fn receive<'a, I: 'a, O: 'a, E: 'a>() -> ResultCoroutine<'a, I, O, I, E> {
    lift(crate::map(crate::receive(), Result::Ok))
}

/// Changes the result value inside
/// 
/// Calls f inside the bind and maps returns the result
pub fn map<'a,I:'a,O:'a,A:'a,B:'a,E:'a,F:'a>(co: ResultCoroutine<'a,I,O,A,E>, f:F) -> ResultCoroutine<'a,I,O,B,E>
    where F: FnOnce(A) -> B {
    bind(co, |a| result(f(a)))
}

/// Converts the error type
/// 
/// Handy to switch error types if you have different
/// coroutines
pub fn map_err<'a,I:'a,O:'a,R:'a,E1:'a,E2:'a,F:'a>(co: ResultCoroutine<'a,I,O,R,E1>, f:F) -> ResultCoroutine<'a,I,O,R,E2>
    where F: FnOnce(E1) -> E2 {
    let co = to_coroutine(co);
    let mapper = | result| {
        match result {
            Result::Ok(e) => Result::Ok(e),
            Result::Err(e1) => Result::Err(f(e1)),
        }
    };
    lift(crate::map(co,mapper))    
}


/// Just like run step, but gives a result type inside
///
/// see (function@run_step)
pub fn run_step<'a, I: 'a, O: 'a, R: 'a, E: 'a>(
    co: ResultCoroutine<'a, I, O, R, E>,
) -> crate::StepResult<'a, I, O, Result<R, E>> {
    let co = to_coroutine(co);
    crate::run_step(co)
}

/// Run a child result coroutine in the parent context
///
/// This is like the coroutine run child
/// Differences are, the routine, its inputs, and its output
/// routines may fail. If any do, the whole routine fails
pub fn run_child<
    'a,
    Input: 'a,
    Output: 'a,
    Error: 'a,
    ChildInput: 'a,
    ChildOutput: 'a,
    OnInput: 'a,
    OnOutput: 'a,
    Return: 'a,
>(
    on_input: OnInput,
    on_output: OnOutput,
    child: ResultCoroutine<'a, ChildInput, ChildOutput, Return, Error>,
) -> ResultCoroutine<'a, Input, Output, Return, Error>
where
    OnInput: Fn() -> ResultCoroutine<'a, Input, Output, ChildInput, Error>,
    OnOutput: Fn(ChildOutput) -> ResultCoroutine<'a, Input, Output, (), Error>,
{
    match run_step(child) {
        crate::StepResult::Done(Result::Ok(r)) => result(r),
        crate::StepResult::Done(Result::Err(e)) => err(e),
        crate::StepResult::Yield { output, next } => {
            let output = on_output(output);
            bind(output, move |()| {
                run_child(on_input, on_output, lift(*next))
            })
        }
        crate::StepResult::Next(n) => on_input().and_then(move |i| {
            let next = n(i);
            run_child(on_input, on_output, lift(next))
        }),
    }
}
