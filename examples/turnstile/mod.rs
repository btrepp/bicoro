use std::fmt::Display;

use bicoro::*;
use do_notation::m;

// Encoding of https://en.wikipedia.org/wiki/Finite-state_machine#Example:_coin-operated_turnstile

/// FSM need a set of inputs
pub enum Input {
    Coin,
    Push,
}

/// And a set of outputs
pub enum Output {
    Unlocked,
    NoChange,
    Locked,
}

/// This is internal, so we can track what is happening
enum State {
    Locked,
    Unlocked,
}

/// Should be !, but not stable yet
pub enum Never {}

/// Creates a new turnstile, in the locked state
pub fn create<'a>() -> Coroutine<'a, Input, Output, Never> {

    // recursive function. that takes the current state, input and 
    // gives a co-routine.
    fn on_input<'a>(state: State, input: Input) -> Coroutine<'a, Input, Output, Never> {

        // coroutine section that sends the output, and sets the next state
        let next = match (state, input) {
            (State::Locked, Input::Coin) => m! {
                send(Output::Unlocked);
                result(State::Unlocked)
            },
            (State::Locked, Input::Push) => m! {
                send(Output::NoChange);
                result(State::Locked)
            },
            (State::Unlocked, Input::Coin) => m! {
                send(Output::NoChange);
                result(State::Unlocked)
            },
            (State::Unlocked, Input::Push) => m! {
                send(Output::Locked);
                result(State::Locked)
            },
        };

        // read an input, and call on_input again
        m! {
            state <- next;
            input <- receive();
            on_input(state, input)
        }
    }
    let initial_state = State::Locked;
    m! {
        input <- receive();
        on_input(initial_state, input)
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Output::Unlocked => f.write_str("Unlocked"),
            Output::Locked => f.write_str("Locked"),
            Output::NoChange => f.write_str("NoChange"),
        }
    }
}
