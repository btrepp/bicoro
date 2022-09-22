use bicoro::{iterator::as_iterator, Coroutine};
mod turnstile;
use turnstile::{create, Input, Never, Output};

/// This is a simple example of how to run a set of inputs into
/// the coroutine, see the turnstile module for the definition
pub fn main() {
    let turnstile: Coroutine<Input, Output, Never> = create();

    // simulate a set of inputs
    let inputs = vec![
        Input::Push, // no change
        Input::Push, // no change
        Input::Push, // no change
        Input::Coin, // unlocked
        Input::Push, // locked
        Input::Coin, // unlocked
        Input::Coin, // no change (wasting money at this point)
        Input::Coin, // no change
        Input::Coin, // no change
        Input::Push, // locked
        Input::Push, // no change
    ];

    let it = as_iterator(turnstile, inputs.into_iter());

    for event in it {
        println!("Turnstile Says:{}", event);
    }
}
