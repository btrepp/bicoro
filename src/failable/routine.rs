use crate::Coroutine;

/// A coroutine that supports a fatal, terminating error
///
/// This is really just a wrapper over the co-routine,
/// where we were do some short-circuiting in bind
pub struct ResultCoroutine<'a, I, O, R, E> {
    co: Coroutine<'a, I, O, Result<R, E>>,
}

/// Creates a coroutine from a
///
/// See [result](function@result)
pub fn result<'a, I, O, A, E>(a: A) -> ResultCoroutine<'a, I, O, A, E> {
    let co = crate::result(Result::Ok(a));
    ResultCoroutine { co }
}

/// Fails the coroutine, further binds will short-circuit
///
/// This allows us to ergonomically fail. Like the result type
/// build into result
pub fn err<'a, I, O, A, E>(e: E) -> ResultCoroutine<'a, I, O, A, E> {
    let co = crate::result(Result::Err(e));
    ResultCoroutine { co }
}

/// Lifts coroutine of result into a result coroutine
///
/// This is useful if you have a function that returns results
/// and want to easily map and lift it into a result coroutine
pub fn lift<I, O, R, E>(co: Coroutine<I, O, Result<R, E>>) -> ResultCoroutine<I, O, R, E> {
    ResultCoroutine { co }
}

/// Extracts a co-routine with a result
///
/// This is the opposite of lift. Useful when you need
/// to get at the error for whatever reason
pub fn to_coroutine<I, O, R, E>(co: ResultCoroutine<I, O, R, E>) -> Coroutine<I, O, Result<R, E>> {
    co.co
}

/// Bind for the result coroutine
///
/// This functions just like and_then for coroutine
/// The one difference is that it has semantics like result,
/// if a failure has occured, that will propagate
pub fn bind<'a, I: 'a, O: 'a, A: 'a, B: 'a, E: 'a, F: 'a>(
    result: ResultCoroutine<'a, I, O, A, E>,
    binder: F,
) -> ResultCoroutine<'a, I, O, B, E>
where
    F: FnOnce(A) -> ResultCoroutine<'a, I, O, B, E>,
{
    let co = crate::bind(result.co, |a| match a {
        Result::Ok(a) => binder(a).co,
        Result::Err(e) => crate::result(Result::Err(e)),
    });
    ResultCoroutine { co }
}
