use bicoro::{executor::execute_from_iter, Coroutine};
mod turnstile;
use turnstile::{Input,Output,create, Never};

/// This is a simple example of how to run a set of inputs into
/// the coroutine, see the turnstile module for the definition
pub fn main() {
    let turnstile: Coroutine<Input,Output,Never> = create();

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
        Input::Push  // no change

    ];
    let on_output = |o| println!("Turnstile Says:{}", o);

    let _ = execute_from_iter(turnstile, on_output, inputs.into_iter());
}

