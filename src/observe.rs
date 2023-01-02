use crate::{bind, receive, result, run_step, Coroutine, StepResult};

pub enum ObserveResult<'a, I, O, R> {
    Output {
        output: O,
        next: Coroutine<'a, I, O, R>,
    },
    Finished {
        value: R,
    },
}

pub fn observe<I, O, R>(co: Coroutine<I, O, R>) -> Coroutine<I, O, ObserveResult<I, O, R>> {
    match run_step(co) {
        StepResult::Done(value) => result(ObserveResult::Finished { value }),
        StepResult::Yield { output, next } => result(ObserveResult::Output {
            output,
            next: *next,
        }),
        StepResult::Next(next) => {
            let on_input = |input| {
                let next = next(input);
                observe(next)
            };
            bind(receive(), on_input)
        }
    }
}
