use crate::{
    bind, inject, receive, result, right, run_step, send, suspend, Coroutine, StepResult,
    UnicastSelect,
};

/// Represents the result of running the left and right coroutines
/// with the ability to share inputs to each other
pub enum CooperateResult<'a, IA, IB, OA, OB, A, B> {
    Left {
        value: A,
        remaining: Coroutine<'a, IB, OB, B>,
    },
    Right {
        value: B,
        remaining: Coroutine<'a, IA, OA, A>,
    },
}
pub fn cooperate<'a, I, O, IA, OA, IB, OB, MA, MB, A, B, S>(
    selector: S,
    map_first: MA,
    map_second: MB,
    first: Coroutine<'a, IA, OA, A>,
    second: Coroutine<'a, IB, OB, B>,
) -> Coroutine<'a, I, O, CooperateResult<'a, IA, IB, OA, OB, A, B>>
where
    S: Fn(I) -> UnicastSelect<IA, IB> + Send + 'a,
    MA: Fn(OA) -> UnicastSelect<IB, O> + Send + 'a,
    MB: Fn(OB) -> UnicastSelect<IA, O> + Send + 'a,
    OA: Send,
    OB: Send,
    B: Send,
    A: Send,
    O: Send,
{
    let sr1 = run_step(first);
    let sr2 = run_step(second);

    match (sr1, sr2) {
        (StepResult::Done(value), StepResult::Done(remaining)) => {
            let coop = CooperateResult::Left {
                value,
                remaining: result(remaining),
            };
            result(coop)
        }
        (StepResult::Done(value), StepResult::Yield { output, next }) => {
            let remaining = bind(send(output), |()| *next);
            let coop = CooperateResult::Left { value, remaining };
            result(coop)
        }
        (StepResult::Done(value), StepResult::Next(next)) => {
            let remaining = suspend(next);
            let coop = CooperateResult::Left { value, remaining };
            result(coop)
        }
        (StepResult::Yield { output, next }, StepResult::Done(value)) => {
            let remaining = bind(send(output), |()| *next);
            let coop = CooperateResult::Right { value, remaining };
            result(coop)
        }
        (
            StepResult::Yield {
                output: a,
                next: na,
            },
            StepResult::Yield {
                output: b,
                next: nb,
            },
        ) => {
            let a: UnicastSelect<IB, O> = map_first(a);
            let b: UnicastSelect<IA, O> = map_second(b);
            match (a, b) {
                (UnicastSelect::Left(ib), UnicastSelect::Left(ia)) => {
                    let first = inject(ia, *na);
                    let second = inject(ib, *nb);
                    cooperate(selector, map_first, map_second, first, second)
                }
                (UnicastSelect::Left(ib), UnicastSelect::Right(o)) => {
                    let first = *na;
                    let second = inject(ib, *nb);
                    let output = send(o);
                    let next = |()| cooperate(selector, map_first, map_second, first, second);
                    bind(output, next)
                }
                (UnicastSelect::Right(o), UnicastSelect::Left(ia)) => {
                    let first = inject(ia, *na);
                    let second = *nb;
                    let output = send(o);
                    let next = |()| cooperate(selector, map_first, map_second, first, second);
                    bind(output, next)
                }
                (UnicastSelect::Right(o1), UnicastSelect::Right(o2)) => {
                    let first = *na;
                    let second = *nb;
                    let output = right(send(o1), send(o2));
                    let next = |()| cooperate(selector, map_first, map_second, first, second);
                    bind(output, next)
                }
            }
        }
        (StepResult::Yield { output, next }, StepResult::Next(input)) => {
            let output: UnicastSelect<IB, O> = map_first(output);
            match output {
                UnicastSelect::Left(ib) => {
                    let first = *next;
                    let second = input(ib);
                    cooperate(selector, map_first, map_second, first, second)
                }
                UnicastSelect::Right(o) => {
                    let output = send(o);
                    let first = *next;
                    let second = suspend(input);
                    let next = |()| cooperate(selector, map_first, map_second, first, second);
                    bind(output, next)
                }
            }
        }
        (StepResult::Next(input), StepResult::Done(value)) => {
            let remaining = suspend(input);
            let coop = CooperateResult::Right { value, remaining };
            result(coop)
        }
        (StepResult::Next(input), StepResult::Yield { output, next }) => {
            let output: UnicastSelect<IA, O> = map_second(output);
            match output {
                UnicastSelect::Left(ia) => {
                    let first = input(ia);
                    let second = *next;
                    cooperate(selector, map_first, map_second, first, second)
                }
                UnicastSelect::Right(o) => {
                    let first = suspend(input);
                    let second = *next;
                    let next = |()| cooperate(selector, map_first, map_second, first, second);
                    let output = send(o);
                    bind(output, next)
                }
            }
        }
        (StepResult::Next(input_a), StepResult::Next(input_b)) => {
            let on_input = |input: I| match selector(input) {
                UnicastSelect::Left(ia) => {
                    let first = input_a(ia);
                    let second = suspend(input_b);
                    cooperate(selector, map_first, map_second, first, second)
                }
                UnicastSelect::Right(ib) => {
                    let first = suspend(input_a);
                    let second = input_b(ib);
                    cooperate(selector, map_first, map_second, first, second)
                }
            };
            bind(receive(), on_input)
        }
    }
}
