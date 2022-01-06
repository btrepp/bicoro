# (Bi)directional (Coro)utines

This is a simple pure library to model a bidirectional coroutine in rust.
This allows a coroutine to be defined abstracted away from the actual IO implementations.

The main motivation is writing unit testable coroutines, that respond to inputs, and generate outputs.
This is a more generic form of Future.

## Example

```rust
use bicoro::*;
use bicoro::executor::*;
use ::do_notation::m;
 
// The coroutine in dot-notation
let co : Coroutine<i32,String,()> =
        m! {
            value_1 <- receive();
            value_2 <- receive();
            let sum = i32::wrapping_add(value_1,value_2);
            let output = sum.to_string();
            send(output);
            result(())
        };
 
// Arrange inputs and an store outputs in a vec
let inputs = vec![1,2];
let mut outputs = vec![];
let on_output = |output:String| outputs.push(output);

// run (and consume the inputs), mutate the output
let exec = execute_from_iter(co,on_output,inputs.into_iter());
 
// Verify
assert!(matches!(exec, IteratorExecutorResult::Completed{result: (),..}));
assert_eq!(outputs, vec!["3"]); 
```

## Executing

The coroutine just describes what to do, it won't actually do anything.
You need to run it through an executor.
One that works on an iterator is provided, but you may also want to create your own
to implement with different contexts.

## do-notatation

Implementation is compatible with <https://github.com/phaazon/do-notation>
allowing for a syntax 'similar in feel' to using .await on futures.

This helps a bit with the callback hell, but is not strictly necessary.