# (Bi)directional (Coro)utines

This is a simple pure library to model a bidirectional coroutine in rust.
This allows a coroutine to be defined abstracted away from the actual IO implementations.

The main motivation is writing unit testable coroutines, that respond to inputs, and generate outputs.
This is a more generic form of Future.

This is a form of co-operative concurrency. It is also similar to an actor model.

## Mental model

This libraries mental model is 'pull' based, similar in concept to rusts iterators, except
that we can also 'ask' for more input, which is not possible using iterators. This means we don't
have to 'allocate' or buffer to much, we only need to box the 'next thing to do' as a closure.

There is no borrows used, coroutines become consumed as they are passed around. This is to explictly
avoid strange cases of 'cloning' a coroutines state, and the complications that would have around how everything
is executed.

## Functional

There is one core data type, the coroutine. The rest of the functionality is provided by functions
that allow you to glue coroutines and closures together to write your problem.



## Example

```rust
use bicoro::*;
use bicoro::iterator::*;
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

// creates an interator of outputs, fed via the inputs
let mut it = as_iterator(co,inputs.into_iter());

// By ref so we don't consume the iterator (important if
// we want to get the remaining inputs or coroutine)
let outputs = it.by_ref().collect::<Vec<_>>();
// Verify output values
assert_eq!(outputs, vec!["3"]); 

// Consume the iterator, so that we can check results
let (result,mut remaining_inputs) = it.finish();
// We return the result from the coroutine.
assert!(matches!(result,Result::Ok(())));
// All the inputs from the iterator were consumed
assert!(matches!(remaining_inputs.unwrap().next(),None));
```

## Executing

The coroutine just describes what to do, it won't actually do anything.
You need to run it through an executor.
One that works on an iterator is provided, but you may also want to create your own
to implement with different contexts.

## Use cases

### Protocol descriptions

This can be used to describe protocols using the sans-io philosophy <https://sans-io.readthedocs.io>. 
A protocol can be thought of an actor that recieves input messages and responds with outputs, maintaing it's
own internal state. As the coroutines are nestable and composeable, we can create subroutines and functions to solve
particular 'sub states' of the protocol, e.g connecting then transitioning to the main part of the protocol.

An executor can then map to low level IO functions, e.g read/write bytes. Having these seperate means the protocol 
implementation can be thoroughly tested

### State Machines

We can model state machines (see examples). Particularly useful as we can compose and combine state machines like in protocols.
Types can even show that there is no 'final' state for the machine e.g `Coroutine<Input,Ouput,!>` implies we can always
feed information to the machine.

On state transitions, we can also emit multiple messages or none, so we are flexible on what the output is.

### Iterator with a terminating state

`Coroutine<!,Output,Result>` is much like a rust iterator, except that the final result doesn't need to be the same type.
This can be useful as you don't need to create an awkward enum, and only ever create the 'termination' state at the end.
It also more clearly communicates 'we are actually done now' and consumes the routine, so it can't be re-used accidentally.

### Folds

`Coroutine<Input,!,Result>` is similar to folding (reduce) a set of inputs. This can be written without even having an iterator.
E.g it could be in a loop where inputs are read as needed, which is handy for interactive components.

## do-notatation

Implementation is compatible with <https://github.com/phaazon/do-notation>
allowing for a syntax 'similar in feel' to using .await on futures.

This helps a bit with the callback hell, but is not strictly necessary.