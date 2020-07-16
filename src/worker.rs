//! Adaptive reductions

use crate::{prelude::*, schedulers::AdaptiveScheduler};

pub struct OwningWorker<S, C, D, W, SD> {
    state: S,
    completed: C,
    divide: D,
    should_divide: SD,
    work: W,
}

impl<S, C, D, W, SD> ParallelIterator for OwningWorker<S, C, D, W, SD>
where
    W: Fn(S, usize) -> S + Sync,
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    type Item = S;
    type Controlled = False;
    type Enumerable = False;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.call(WorkerProducer {
            state: Some(self.state),
            completed: &self.completed,
            divide: &self.divide,
            should_divide: &self.should_divide,
            work: &self.work,
        })
    }
}

pub(crate) struct WorkerProducer<'l, S, C, D, W, SD> {
    state: Option<S>,
    completed: &'l C,
    divide: &'l D,
    should_divide: &'l SD,
    work: &'l W,
}

impl<'f, S, C, D, W, SD> Iterator for WorkerProducer<'f, S, C, D, W, SD>
where
    S: Send,
    W: Fn(S, usize) -> S + Sync,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    type Item = S;
    // One-shot iterator
    // If the iterator is finished, yield the state, else finish the work and yield the state
    // This always consumes the internal state
    fn next(&mut self) -> Option<Self::Item> {
        if self
            .state
            .as_ref()
            .map_or(false, |refstate| (self.completed)(refstate))
        {
            self.state.take()
        } else {
            self.state.take().map(|s| (self.work)(s, usize::MAX))
        }
    }
}

impl<'f, S, C, D, W, SD> Divisible for WorkerProducer<'f, S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(S, usize) -> S + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    type Controlled = False;
    fn should_be_divided(&self) -> bool {
        self.state
            .as_ref()
            .map_or(false, |refstate| (self.should_divide)(refstate))
    }
    fn divide(self) -> (Self, Self) {
        //Come on Rustc!
        let dividor = self.divide;
        let (lefts, rights) = self.state.map_or((None, None), |s| {
            let (l, r) = dividor(s);
            (Some(l), Some(r))
        });
        (
            WorkerProducer {
                state: lefts,
                completed: self.completed,
                divide: self.divide,
                should_divide: self.should_divide,
                work: self.work,
            },
            WorkerProducer {
                state: rights,
                completed: self.completed,
                divide: self.divide,
                should_divide: self.should_divide,
                work: self.work,
            },
        )
    }
    fn divide_at(self, _index: usize) -> (Self, Self) {
        panic!("Called divide_at on WorkerProducer but it is not controlled.")
    }
}

impl<'f, S, C, D, W, SD> Producer for WorkerProducer<'f, S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(S, usize) -> S + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
    fn preview(&self, _index: usize) -> Self::Item {
        panic!("you cannot preview a Worker")
    }
    fn scheduler<'s, P: 's, R: 's>(&self) -> Box<dyn Scheduler<P, R> + 's>
    where
        P: Producer,
        P::Item: Send,
        R: Reducer<P::Item>,
    {
        Box::new(AdaptiveScheduler)
    }
    fn completed(&self) -> bool {
        self.state
            .as_ref()
            .map_or(true, |refstate| (self.completed)(refstate))
    }
    fn partial_fold<B, F>(&mut self, init: B, _fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        // take current state
        // work on it for the limit
        // put it back in yourself
        // give a fake output (init) to the caller
        let current_state = self.state.take();
        let new_state = current_state.map(|s| (self.work)(s, limit));
        self.state = new_state;
        init
    }
}

pub fn work_with_output<'a, S, C, D, W, SD>(
    init: S,
    completed: C,
    divide: D,
    work: W,
    should_be_divided: SD,
) -> OwningWorker<S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(S, usize) -> S + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    OwningWorker {
        state: init,
        completed,
        divide,
        work,
        should_divide: should_be_divided,
    }
}
