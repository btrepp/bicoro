//! A sample executor that consumes an iterator
//!
//! It's not necessary to use this, as run_step is all you need if rolling your own
//! but it's a good reference, and is fairly generally useable

use std::fmt::Debug;

use crate::*;

pub enum IteratorExecutorResult<'a, It, Input, Output, Result> {
    /// The coroutine has finished
    Completed {
        /// The final result of the coroutine
        result: Result,
        remaining: It,
    },
    Output {
        /// Value emitted
        output: Output,
        co: Coroutine<'a, Input, Output, Result>,
        remaining: It,
    },
    /// We ran out of inputs, returns a coroutine to continue when more inputs are
    /// available
    Exhausted {
        co: Box<dyn FnOnce(Input) -> Coroutine<'a, Input, Output, Result> + Send + 'a>,
    },
}

impl<'a, It, Input, Output, Result> core::fmt::Debug
    for IteratorExecutorResult<'a, It, Input, Output, Result>
where
    Result: Debug,
    It: Debug,
    Output: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed { result, remaining } => f
                .debug_struct("Completed")
                .field("result", result)
                .field("remaining", remaining)
                .finish(),
            Self::Output {
                output,
                co: _,
                remaining,
            } => f
                .debug_struct("Output")
                .field("output", output)
                .field("remaining", remaining)
                .finish(),
            Self::Exhausted { co: _ } => f.debug_struct("Exhausted").finish(),
        }
    }
}

/// Consumes a coroutine and runs it with the iterated events
/// This may run to completion, or may consume all the inputs
/// Returns whenever an output is produced, and returns the remaining
/// iterator and coroutine so it can be called again
/// ```
/// use bicoro::*;
/// use bicoro::executor::*;
///
/// // sample coroutine
/// let co : Coroutine<i32,i32,()> = receive().and_then(|i| send(i));
///
/// // inputs to send
/// let inputs = vec![1];
///
/// let exec = run_until_output(co,inputs.into_iter());
///
/// assert!(matches!(exec, IteratorExecutorResult::Output{ output: 1,..}));
/// ```
pub fn run_until_output<Iter, Input, Output, Result>(
    mut routine: Coroutine<Input, Output, Result>,
    mut events: Iter,
) -> IteratorExecutorResult<Iter, Input, Output, Result>
where
    Iter: Iterator<Item = Input>,
{
    loop {
        match run_step(routine) {
            StepResult::Done(result) => {
                return IteratorExecutorResult::Completed {
                    result,
                    remaining: events,
                }
            }
            StepResult::Yield { output, next } => {
                return IteratorExecutorResult::Output {
                    output,
                    remaining: events,
                    co: *next,
                };
            }
            StepResult::Next(next) => {
                if let Some(event) = events.next() {
                    routine = next(event);
                } else {
                    return IteratorExecutorResult::Exhausted { co: next };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Co<I, O, R> = Coroutine<'static, I, O, R>;
    #[test]
    fn not_enough_input_data() {
        let test: Co<i32, (), i32> = receive();
        let inputs = vec![];

        let exec = run_until_output(test, inputs.into_iter());

        assert!(matches!(exec, IteratorExecutorResult::Exhausted { .. }));
    }

    #[test]
    fn instantly_completes() {
        let test: Co<(), (), i32> = result(1);
        let inputs = vec![];

        let exec = run_until_output(test, inputs.into_iter());

        assert!(matches!(
            exec,
            IteratorExecutorResult::Completed { result: 1, .. }
        ));
    }

    #[test]
    fn send_writes_to_vec() {
        let test: Co<(), i32, ()> = send(1);
        let inputs = vec![];

        let exec = run_until_output(test, inputs.into_iter());

        assert!(matches!(
            exec,
            IteratorExecutorResult::Output { output: 1, .. }
        ));
    }

    #[test]
    fn send_writes_multiple_to_vec() {
        let test: Co<(), i32, ()> = send(1).and_then(|()| send(2));
        let inputs = vec![];

        let exec = run_until_output(test, inputs.into_iter());

        assert!(matches!(
            exec,
            IteratorExecutorResult::Output { output: 1, .. }
        ));
    }
}
