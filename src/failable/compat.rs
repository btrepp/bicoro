//! Compatibility with do-notation
//!
use super::ResultCoroutine;

impl<'a, I: 'a, O: 'a, A: 'a, E: 'a> ResultCoroutine<'a, I, O, A, E> {
    pub fn and_then<B: 'a, F: 'a>(self, f: F) -> ResultCoroutine<'a, I, O, B, E>
    where
        F: FnOnce(A) -> ResultCoroutine<'a, I, O, B, E>,
    {
        super::bind(self, f)
    }
}

impl<'a, I: 'a, O: 'a, A: 'a, E: 'a> do_notation::Lift<A> for ResultCoroutine<'a, I, O, A, E> {
    fn lift(a: A) -> Self {
        super::result(a)
    }
}
