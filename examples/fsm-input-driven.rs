use std::io::Write;

use bicoro::*;
mod turnstile;
use turnstile::create;

// Our terminal just reads lines
pub struct Input(String);

// It can output to stdout, or stderr
pub enum Output {
    StdOut(String),
    StdErr(String),
    Flush,
}

/// Reads input until we get a valid turnstile input
/// Here we can handle if the inputs aren't correct
pub fn needs_input() -> Coroutine<'static, Input, Output, turnstile::Input> {
    // this is a recursive function. Any easy way to express a loop
    fn loop_(input: Input) -> Coroutine<'static, Input, Output, turnstile::Input> {
        match input.0.trim() {
            "push" => result(turnstile::Input::Push),
            "coin" => result(turnstile::Input::Coin),
            other => {
                let error = format!("Turnstile: 'I don't understand {}' \r\n", other);
                let prompt = "What do you try instead?".to_string();

                send(Output::StdErr(error))
                    .and_then(|()| send(Output::StdOut(prompt)))
                    .and_then(|()| send(Output::Flush))
                    .and_then(|()| receive())
                    .and_then(loop_) // we loop if it's invalid, we need to get that input for the turnstile!
            }
        }
    }
    // kicks off the loop
    let initial_prompt = format!("What do you do?: ");
    send(Output::StdOut(initial_prompt))
        .and_then(|()| send(Output::Flush))
        .and_then(|()| receive())
        .and_then(loop_)
}

/// Simply display the transition
pub fn on_output(o: turnstile::Output) -> Coroutine<'static, Input, Output, ()> {
    send(Output::StdOut(format!(
        "Turnstile responds with: '{}'\r\n",
        o.to_string()
    )))
}

pub fn main() {
    let turnstile = create();

    // we can run a child routine inside this routine, with the provided functiosn to convert
    // the inputs and outputs. Result remains the same.
    let mut composed = send(Output::StdOut(
        "You are stopped by a turnstile!\r\n".to_string(),
    ))
    .and_then(|()| send(Output::Flush))
    .and_then(|()| run_child(needs_input, on_output, turnstile));

    // This is the main loop. Notice it's really only concerned
    // with handling inputs and outputs of the termninal.
    loop {
        match run_step(composed) {
            StepResult::Done(_) => unreachable!(), // just for this case, as its a non-exiting coroutine
            StepResult::Yield { output, next } => {
                match output {
                    Output::StdOut(o) => print!("{}", o),
                    Output::StdErr(e) => print!("{}", e),
                    Output::Flush => std::io::stdout().flush().unwrap(),
                }
                composed = *next;
            }
            StepResult::Next(fun) => {
                // Prompts only appear when needed here
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf).unwrap();
                let input = Input(buf);
                composed = fun(input);
            }
        }
    }
}
