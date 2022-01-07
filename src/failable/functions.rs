use super::{lift, ResultCoroutine};

/// Just like coroutine send, but sugared for result coroutines
///
/// see (function@send)
pub fn send<'a, I: 'a, O: 'a, E: 'a>(o: O) -> ResultCoroutine<'a, I, O, (), E> {
    lift(crate::map(crate::send(o), Result::Ok))
}

/// Just like coroutine receive, but sugared for result coroutines
///
/// see (function@receive)
pub fn receive<'a, I: 'a, O: 'a, E: 'a>() -> ResultCoroutine<'a, I, O, I, E> {
    lift(crate::map(crate::receive(), Result::Ok))
}
