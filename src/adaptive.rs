//! Adaptive reductions

use crate::prelude::*;

struct Worker<'f, S, C, D, W, SD> {
    state: S,
    completed: &'f C,
    divide: &'f D,
    should_divide: &'f SD,
    work: &'f W,
}

impl<'f, S, C, D, W, SD> Iterator for Worker<'f, S, C, D, W, SD>
where
    W: Fn(&mut S, usize) + Sync,
    C: Fn(&S) -> bool + Sync,
{
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
    fn fold<B, F>(mut self, init: B, _f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        (self.work)(&mut self.state, std::usize::MAX);
        init
    }
}

impl<'f, S, C, D, W, SD> Divisible for Worker<'f, S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(&mut S, usize) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    type Controlled = False;
    fn should_be_divided(&self) -> bool {
        (self.should_divide)(&self.state)
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = (self.divide)(self.state);
        (
            Worker {
                state: left,
                completed: self.completed,
                divide: self.divide,
                should_divide: self.should_divide,
                work: self.work,
            },
            Worker {
                state: right,
                completed: self.completed,
                should_divide: self.should_divide,
                divide: self.divide,
                work: self.work,
            },
        )
    }
    fn divide_at(self, _index: usize) -> (Self, Self) {
        panic!("should never be called")
    }
}

impl<'f, S, C, D, W, SD> Producer for Worker<'f, S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(&mut S, usize) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
    fn preview(&self, _index: usize) -> Self::Item {
        panic!("you cannot preview a Worker")
    }
    fn completed(&self) -> bool {
        (self.completed)(&self.state)
    }
    fn partial_fold<B, F>(&mut self, init: B, _fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        (self.work)(&mut self.state, limit);
        init
    }
}

pub fn work<S, C, D, W, SD>(init: S, completed: C, divide: D, work: W, should_be_divided: SD)
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(&mut S, usize) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    let worker = Worker {
        state: init,
        completed: &completed,
        divide: &divide,
        work: &work,
        should_divide: &should_be_divided,
    };
    let identity = || ();
    let op = |_, _| ();
    let reducer = crate::traits::ReduceConsumer {
        op: &op,
        identity: &identity,
    };
    crate::schedulers::adaptive_scheduler(&reducer, worker, ());
}
