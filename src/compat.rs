//! Compatibility with do-notation
//!
//! As rust currently lacks the ability to define a generic 'bind'
//! operation, thus provides syntactic sugar.
//! It's not necessary to use the library, but does help with callback hell
//!```
//! use bicoro::*;
//! use bicoro::iterator::*;
//! use ::do_notation::m;
//!
//! // The coroutine in do-notation
//! let co : Coroutine<i32,String,()> =
//!        m! {
//!            value_1 <- receive();
//!            value_2 <- receive();
//!            let sum = i32::wrapping_add(value_1,value_2);
//!            let output = sum.to_string();
//!            send(output);
//!            result(())
//!        };
//!
//! // Execute
//! let inputs = vec![1,2];
//! let it = as_iterator(co,inputs.into_iter());
//! let values = it.collect::<Vec<_>>();
//!
//! // Verify
//! assert_eq!(values, vec!["3"]);
//!```
use crate::*;
use ::do_notation::Lift;

impl<'a, I, O, R> compat::Lift<R> for Coroutine<'a, I, O, R> {
    /// Creates coroutine from a value
    ///
    /// see [result](function@result)
    fn lift(a: R) -> Self {
        result(a)
    }
}

impl<'a, I, O, R> Coroutine<'a, I, O, R> {
    /// Chains coroutines
    ///
    /// see [bind](function@bind)  
    pub fn and_then<F: 'a, B>(self, f: F) -> Coroutine<'a, I, O, B>
    where
        F: FnOnce(R) -> Coroutine<'a, I, O, B>,
    {
        bind(self, f)
    }
}
