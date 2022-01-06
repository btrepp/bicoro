//! A sample executor that consumes an iterator
//!
//! It's not necessary to use this, as run_step is all you need if rolling your own
//! but it's a good reference, and is fairly generally useable

use crate::*;

pub enum IteratorExecutorResult<'a, Iter, Input, Output, Result> {
    /// The coroutine has finished
    Completed {
        /// The final result of the coroutine
        result: Result,
        /// What remains of the input
        remaining: Iter,
    },
    /// We ran out of inputs, returns a coroutine to continue when more inputs are
    /// available
    OutOfInputs(Coroutine<'a, Input, Output, Result>),
}

/// Consumes a coroutine and runs it with the iterated events
/// This may run to completion, or may consume all the inputs
/// The on_output function is called whenever a output is produced
/// Note: it's expected that shouldn't ever panic
/// ```
/// use bicoro::*;
/// use bicoro::executor::*;
///
/// // sample coroutine
/// let co : Coroutine<i32,i32,()> = receive().and_then(|i| send(i));
///
/// // inputs to send
/// let inputs = vec![1];
/// let mut outputs = vec![];
/// let on_output = |output:i32| outputs.push(output);
///
/// let exec = execute_from_iter(co,on_output,inputs.into_iter());
///
/// assert!(matches!(exec, IteratorExecutorResult::Completed{ result: (),..}));
/// assert_eq!(outputs, vec![1]);
/// ```
pub fn execute_from_iter<'a, Iter, Input: 'a, Output: 'a, OnOutput, Result: 'a>(
    mut routine: Coroutine<'a, Input, Output, Result>,
    mut on_output: OnOutput,
    mut events: Iter,
) -> IteratorExecutorResult<'a, Iter, Input, Output, Result>
where
    Iter: Iterator<Item = Input>,
    OnOutput: FnMut(Output),
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
                on_output(output);
                routine = *next;
            }
            StepResult::Next(next) => {
                if let Some(event) = events.next() {
                    routine = next(event);
                } else {
                    let next = suspend(next);
                    return IteratorExecutorResult::OutOfInputs(next);
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
        let mut output = vec![];
        let on_output = |o| output.push(o);

        let exec = execute_from_iter(test, on_output, inputs.into_iter());

        assert!(matches!(exec, IteratorExecutorResult::OutOfInputs(_)));
        assert_eq!(output, vec![]);
    }

    #[test]
    fn instantly_completes() {
        let test: Co<(), (), i32> = result(1);
        let inputs = vec![];
        let mut output = vec![];
        let on_output = |o| output.push(o);

        let exec = execute_from_iter(test, on_output, inputs.into_iter());

        assert!(matches!(
            exec,
            IteratorExecutorResult::Completed { result: 1, .. }
        ));
        assert_eq!(output, vec![]);
    }

    #[test]
    fn send_writes_to_vec() {
        let test: Co<(), i32, ()> = send(1);
        let inputs = vec![];
        let mut output = vec![];
        let on_output = |o| output.push(o);

        let exec = execute_from_iter(test, on_output, inputs.into_iter());

        assert!(matches!(
            exec,
            IteratorExecutorResult::Completed { result: (), .. }
        ));
        assert_eq!(output, vec![1]);
    }

    #[test]
    fn send_writes_multiple_to_vec() {
        let test: Co<(), i32, ()> = send(1).and_then(|()| send(2));
        let inputs = vec![];
        let mut output = vec![];
        let on_output = |o| output.push(o);

        let exec = execute_from_iter(test, on_output, inputs.into_iter());

        assert!(matches!(
            exec,
            IteratorExecutorResult::Completed { result: (), .. }
        ));
        assert_eq!(output, vec![1, 2]);
    }
}
