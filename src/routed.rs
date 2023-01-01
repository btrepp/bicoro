use crate::{
    bind, inject, receive, result, right, run_step, send, suspend, Coroutine, StepResult,
    UnicastSelect,
};

pub enum RoutedResult<'a, IA, IB, O, RA, RB> {
    Left {
        value: RA,
        remain: Coroutine<'a, IB, UnicastSelect<IA, O>, RB>,
    },
    Right {
        value: RB,
        remain: Coroutine<'a, IA, UnicastSelect<IB, O>, RA>,
    },
}

/// Chain and dispatch combined.
///
/// This allows first and second corotines to 'talk to'
/// each other. It also allows inputs to be routed to either if needed.
/// first and second won't care whether inputs are fed from above
/// or from each other. Outputs would be in a common language,
/// e.g they must both emit the same outputs
pub fn routed<'a, IA, IB, O, RA, RB>(
    first: Coroutine<'a, IA, UnicastSelect<IB, O>, RA>,
    second: Coroutine<'a, IB, UnicastSelect<IA, O>, RB>,
) -> Coroutine<'a, UnicastSelect<IA, IB>, O, RoutedResult<'a, IA, IB, O, RA, RB>>
where
    IA: Send,
    IB: Send,
    O: Send,
    RA: Send,
    RB: Send,
{
    let sr1 = run_step(first);
    let sr2 = run_step(second);

    match (sr1, sr2) {
        (StepResult::Done(value), StepResult::Done(b)) => {
            let remain = result(b);
            let coop = RoutedResult::Left { value, remain };
            result(coop)
        }
        (StepResult::Done(value), StepResult::Yield { output, next }) => match output {
            UnicastSelect::Left(i) => {
                let remain = bind(send(UnicastSelect::Left(i)), |()| *next);
                let coop = RoutedResult::Left { value, remain };
                result(coop)
            }
            UnicastSelect::Right(o) => {
                let remain = *next;
                let output = send(o);
                let coop = RoutedResult::Left { value, remain };
                right(output, result(coop))
            }
        },
        (StepResult::Done(value), StepResult::Next(input)) => {
            let remain = suspend(input);
            let coop = RoutedResult::Left { value, remain };
            result(coop)
        }
        (StepResult::Yield { output, next }, StepResult::Done(value)) => match output {
            UnicastSelect::Left(i) => {
                let remain = bind(send(UnicastSelect::Left(i)), |()| *next);
                let coop = RoutedResult::Right { value, remain };
                result(coop)
            }
            UnicastSelect::Right(o) => {
                let output = send(o);
                let remain = *next;
                let coop = RoutedResult::Right { value, remain };
                bind(output, |()| result(coop))
            }
        },
        (
            StepResult::Yield {
                output: oa,
                next: na,
            },
            StepResult::Yield {
                output: ob,
                next: nb,
            },
        ) => match (oa, ob) {
            (UnicastSelect::Left(c), UnicastSelect::Left(i)) => {
                let first = inject(i, *na);
                let second = inject(c, *nb);
                routed(first, second)
            }
            (UnicastSelect::Left(c), UnicastSelect::Right(o)) => {
                let output = send(o);
                let first = *na;
                let second = inject(c, *nb);
                let next = |()| routed(first, second);
                bind(output, next)
            }
            (UnicastSelect::Right(o), UnicastSelect::Left(i)) => {
                let output = send(o);
                let first = inject(i, *na);
                let second = *nb;
                let next = |()| routed(first, second);
                bind(output, next)
            }
            (UnicastSelect::Right(oa), UnicastSelect::Right(ob)) => {
                let output = right(send(oa), send(ob));
                let first = *na;
                let second = *nb;
                let next = |()| routed(first, second);
                bind(output, next)
            }
        },
        (StepResult::Yield { output, next }, StepResult::Next(input)) => match output {
            UnicastSelect::Left(c) => {
                let first = *next;
                let second = input(c);
                routed(first, second)
            }
            UnicastSelect::Right(o) => {
                let output = send(o);
                let first = *next;
                let second = suspend(input);
                let next = |()| routed(first, second);
                bind(output, next)
            }
        },
        (StepResult::Next(input), StepResult::Done(value)) => {
            let remain = suspend(input);
            let coop = RoutedResult::Right { value, remain };
            result(coop)
        }
        (StepResult::Next(input), StepResult::Yield { output, next }) => match output {
            UnicastSelect::Left(i) => {
                let first = input(i);
                let second = *next;
                routed(first, second)
            }
            UnicastSelect::Right(o) => {
                let output = send(o);

                let on_input = |i| match i {
                    UnicastSelect::Left(ia) => {
                        let first = input(ia);
                        let second = *next;
                        routed(first, second)
                    }
                    UnicastSelect::Right(ib) => {
                        let first = suspend(input);
                        let second = inject(ib, *next);
                        routed(first, second)
                    }
                };
                let next = |()| bind(receive(), on_input);
                bind(output, next)
            }
        },
        (StepResult::Next(input_a), StepResult::Next(input_b)) => {
            let on_input = |i| match i {
                UnicastSelect::Left(ia) => {
                    let first = input_a(ia);
                    let second = suspend(input_b);
                    routed(first, second)
                }
                UnicastSelect::Right(ib) => {
                    let first = suspend(input_a);
                    let second = input_b(ib);
                    routed(first, second)
                }
            };
            bind(receive(), on_input)
        }
    }
}
