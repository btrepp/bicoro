///! Convert the coroutine to an iterator
///!
///! One of the issues here is that we
use crate::{
    executor::{run_until_output, IteratorExecutorResult},
    *,
};

pub struct CoroutineIterator<'a, It, I, O, R>
where
    It: Iterator<Item = I>,
{
    co: Option<Coroutine<'a, I, O, R>>,
    result: Option<R>,
    inputs: Option<It>,
}

impl<'a, It, I, O, R> Iterator for CoroutineIterator<'a, It, I, O, R>
where
    It: Iterator<Item = I>,
{
    type Item = O;

    fn next(&mut self) -> Option<Self::Item> {
        let mut co = None;
        let mut it = None;
        std::mem::swap(&mut self.co, &mut co);
        std::mem::swap(&mut self.inputs, &mut it);
        if co.is_some() && it.is_some() {
            let co = co.unwrap();
            let it = it.unwrap();
            match run_until_output(co, it) {
                IteratorExecutorResult::Completed { result, remaining } => {
                    std::mem::swap(&mut self.inputs, &mut Some(remaining));
                    self.result = Some(result);
                    None
                }
                IteratorExecutorResult::Output {
                    output,
                    co,
                    remaining,
                } => {
                    std::mem::swap(&mut self.inputs, &mut Some(remaining));
                    std::mem::swap(&mut self.co, &mut Some(co));
                    Some(output)
                }
                IteratorExecutorResult::Exhausted { co } => {
                    let co = suspend(co);
                    std::mem::swap(&mut self.co, &mut Some(co));
                    None
                }
            }
        } else {
            std::mem::swap(&mut self.co, &mut co);
            std::mem::swap(&mut self.inputs, &mut it);
            None
        }
    }
}

impl<'a, It, I, O, R> CoroutineIterator<'a, It, I, O, R>
where
    It: Iterator<Item = I>,
{
    pub fn finish(self) -> (Option<Coroutine<'a, I, O, R>>, It) {
        (self.co, self.inputs.unwrap())
    }
}

pub fn as_iterator<'a, I, O, R, It>(
    co: Coroutine<'a, I, O, R>,
    inputs: It,
) -> CoroutineIterator<'a, It, I, O, R>
where
    It: Iterator<Item = I>,
{
    CoroutineIterator {
        co: Some(co),
        result: None,
        inputs: Some(inputs),
    }
}
