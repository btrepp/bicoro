impl<'a, I: 'a, O: 'a, R: 'a> Coroutine<'a, I, O, R> {
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
