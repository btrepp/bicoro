use crate::{
    bind, intercept_input, map, map_input, map_output, receive, recieve_until, result, right,
    run_step, send, subroutine, suspend, tuple, Coroutine, StepResult,
};

/// A selection for which coroutine to route to
pub enum Select<A, B, C> {
    Left(A),
    Right(B),
    Both(C),
}

/// Represents the result of running the left and right coroutines
/// Returns whichever coroutine finished first
pub enum DispatchResult<'a, IA, IB, OA, OB, A, B> {
    Left {
        value: A,
        remaining: Coroutine<'a, IB, OB, B>,
    },
    Right {
        value: B,
        remaining: Coroutine<'a, IA, OA, A>,
    },
}

type DispatchRoutine<'a, IA, IB, IAB, OA, OB, A, B> = Coroutine<
    'a,
    Select<IA, IB, IAB>,
    UnicastSelect<OA, OB>,
    DispatchResult<'a, IA, IB, OA, OB, A, B>,
>;
/// Run two co-routines, sharing inputs depending on selector.
///
/// This can be thought of as running them almost in parralel.
/// Selector will route the input as needed to coroutines a and b
/// Values that can be sent to both, if the selector selects both
/// and the both value is cloneable and convertable
pub fn dispatch<'a, IA, IB, IAB, OA, OB, A, B>(
    first: Coroutine<'a, IA, OA, A>,
    second: Coroutine<'a, IB, OB, B>,
) -> DispatchRoutine<'a, IA, IB, IAB, OA, OB, A, B>
where
    IAB: Into<IA> + Into<IB> + Clone,
    OA: Send,
    OB: Send,
    A: Send,
    B: Send,
{
    let s1 = run_step(first);
    let s2 = run_step(second);

    match (s1, s2) {
        (StepResult::Done(value), StepResult::Done(b)) => {
            let ret = DispatchResult::Left {
                value,
                remaining: result(b),
            };
            result(ret)
        }
        (StepResult::Done(value), StepResult::Yield { output, next }) => {
            let remaining = *next;
            let race = DispatchResult::Left { value, remaining };
            right(send(UnicastSelect::Right(output)), result(race))
        }
        (StepResult::Done(value), StepResult::Next(next)) => {
            let remaining = suspend(next);
            result(DispatchResult::Left { value, remaining })
        }
        (StepResult::Yield { output, next }, StepResult::Done(value)) => {
            let remaining = *next;
            let race = DispatchResult::Right { value, remaining };
            right(send(UnicastSelect::Left(output)), result(race))
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
            let send = tuple(send(UnicastSelect::Left(a)), send(UnicastSelect::Right(b)));
            let next = dispatch(*na, *nb);
            right(send, next)
        }
        (StepResult::Yield { output, next: a }, StepResult::Next(b)) => {
            let send = send(UnicastSelect::Left(output));
            let next = dispatch(*a, suspend(b));
            right(send, next)
        }
        (StepResult::Next(a), StepResult::Done(value)) => {
            let remaining = suspend(a);
            let race = DispatchResult::Right { value, remaining };
            result(race)
        }
        (StepResult::Next(a), StepResult::Yield { output, next }) => {
            let send = send(UnicastSelect::Right(output));
            let a = suspend(a);
            let b = *next;
            let next = dispatch(a, b);
            right(send, next)
        }
        (StepResult::Next(a), StepResult::Next(b)) => {
            let on_input = |input: Select<IA, IB, IAB>| match input {
                Select::Left(ia) => {
                    let a = a(ia);
                    dispatch(a, suspend(b))
                }
                Select::Right(ib) => {
                    let b = b(ib);
                    dispatch(suspend(a), b)
                }
                Select::Both(iab) => {
                    let ia = iab.clone().into();
                    let ib = iab.into();
                    let a = a(ia);
                    let b = b(ib);
                    dispatch(a, b)
                }
            };
            bind(receive(), on_input)
        }
    }
}

pub type BroadcastRoutine<'a, I, OA, OB, A, B> =
    Coroutine<'a, I, UnicastSelect<OA, OB>, DispatchResult<'a, I, I, OA, OB, A, B>>;
/// Sends inputs to both coroutines, and will emit outputs together
///
/// This is a more generic form of dispatch
/// in which we want inputs send to both routines always
pub fn broadcast<'a, I, OA, OB, A, B>(
    first: Coroutine<'a, I, OA, A>,
    second: Coroutine<'a, I, OB, B>,
) -> BroadcastRoutine<'a, I, OA, OB, A, B>
where
    I: Clone,
    A: Send,
    B: Send,
    OA: Send,
    OB: Send,
{
    fn both_selector<I>(input: I) -> Select<I, I, I> {
        Select::Both(input)
    }
    map_input(dispatch(first, second), both_selector)
}

/// Sends inputs to both coroutines, and will emit outputs together
///
/// If one finishes first, the other will consume the inputs until it is finished.
/// Both routines must finish to return
/// Input must be cloneable as it will need to feed both
/// This is similar to broadcast, but continues running the 'last' routine
pub fn broadcast_until_finished<'a, I, O, A, B>(
    first: Coroutine<'a, I, O, A>,
    second: Coroutine<'a, I, O, B>,
) -> Coroutine<'a, I, O, (A, B)>
where
    I: Clone,
    A: Send,
    B: Send,
    O: Send,
{
    let rr = broadcast(first, second);
    let rr = map_output(rr, |input| match input {
        UnicastSelect::Left(a) => a,
        UnicastSelect::Right(a) => a,
    });
    let on_result = |res| match res {
        DispatchResult::Left { value, remaining } => map(remaining, |b| (value, b)),
        DispatchResult::Right { value, remaining } => map(remaining, |a| (a, value)),
    };
    bind(rr, on_result)
}

/// A more specific version of select, where messages are exclusive
///
/// This allows you to not have to deal with A or B being cloneable
pub enum UnicastSelect<A, B> {
    Left(A),
    Right(B),
}

/// Run two co-routines, sharing inputs depending on selector.
///
/// This can be thought of as running them almost in parralel.
/// Selector will route the input as needed to coroutines a and b
/// This variant must send inputs to either first or second
/// it does not share them. See dispatch if you need to share values
pub fn unicast<'a, IA, IB, OA, OB, A, B>(
    first: Coroutine<'a, IA, OA, A>,
    second: Coroutine<'a, IB, OB, B>,
) -> Coroutine<
    'a,
    UnicastSelect<IA, IB>,
    UnicastSelect<OA, OB>,
    DispatchResult<'a, IA, IB, OA, OB, A, B>,
>
where
    OA: Send,
    OB: Send,
    A: Send,
    B: Send,
{
    // Private never type. Used for some trickery in unicast to cover into
    // for a type never used
    #[derive(Clone)]
    enum Never {}

    // Allows us to provide an impl for Never to wrapped t
    // This is an exception, but is never constructed. So is safe
    struct Wrapped<T>(T);
    impl<T> From<Never> for Wrapped<T> {
        fn from(_: Never) -> Self {
            unreachable!("This should never be called")
        }
    }

    // Selector will never create Both, thus never and Into Impls are safe
    let selector_ = move |input| -> Select<Wrapped<IA>, Wrapped<IB>, Never> {
        match input {
            UnicastSelect::Left(l) => Select::Left(Wrapped(l)),
            UnicastSelect::Right(r) => Select::Right(Wrapped(r)),
        }
    };

    // We need to convert out of the 'wrapped' inputs. So we change normal inputs to wrapped ones
    let extract = |dr| match dr {
        DispatchResult::Left { value, remaining } => {
            let remaining = intercept_input(remaining, |input| result(Wrapped(input)));
            DispatchResult::Left { value, remaining }
        }
        DispatchResult::Right { value, remaining } => {
            let remaining = intercept_input(remaining, |input| result(Wrapped(input)));
            DispatchResult::Right { value, remaining }
        }
    };

    // Extract from the wrappers to pass to lower levels
    let first = intercept_input(first, |input: Wrapped<IA>| result(input.0));
    let second = intercept_input(second, |input: Wrapped<IB>| result(input.0));
    let both = map_input(dispatch(first, second), selector_);
    map(both, extract)
}

/// Unicast until both routines are completed
///
/// Unlike broadcast, there is an issue with completing unicast routines
/// If IA is finished, the rest of its inputs will be thrown away/ignored
/// Be careful, as this could cause the routines to never complete
pub fn unicast_until_finished<'a, IA, IB, O, A, B>(
    first: Coroutine<'a, IA, O, A>,
    second: Coroutine<'a, IB, O, B>,
) -> Coroutine<'a, UnicastSelect<IA, IB>, O, (A, B)>
where
    O: Send,
    A: Send,
    B: Send,
{
    let is_ib = move |input| match input {
        UnicastSelect::Left(_) => result(None),
        UnicastSelect::Right(b) => result(Some(b)),
    };
    let is_ia = move |input| match input {
        UnicastSelect::Left(a) => result(Some(a)),
        UnicastSelect::Right(_) => result(None),
    };

    // These ultimately throw away inputs that aren't for them
    let on_ib_input = move || recieve_until(is_ib);
    let on_ia_input = move || recieve_until(is_ia);

    // This will finish-of, the 'loser' coroutine. Thus getting the values tupled together
    let on_result = move |r| match r {
        DispatchResult::Left { value, remaining } => {
            let remaining = subroutine(on_ib_input, send, remaining);
            map(remaining, |b| (value, b))
        }
        DispatchResult::Right { value, remaining } => {
            let remaining = subroutine(on_ia_input, send, remaining);
            map(remaining, |a| (a, value))
        }
    };
    let ur = unicast(first, second);
    let ur = map_output(ur, |o| match o {
        UnicastSelect::Left(o) => o,
        UnicastSelect::Right(o) => o,
    });
    bind(ur, on_result)
}
