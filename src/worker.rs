//! Adaptive reductions

use crate::{prelude::*, schedulers::AdaptiveScheduler};

pub struct OwningWorker<S, C, W> {
    pub(crate) state: S,
    pub(crate) completed: C, // TODO: it is not so good we don't know the size
    pub(crate) work: W,
}

impl<S, C, W> ParallelIterator for OwningWorker<S, C, W>
where
    S: Divisible + Send,
    C: Fn(&S) -> bool + Sync,
    W: Fn(&mut S, usize) + Sync,
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
            work: &self.work,
        })
    }
}

pub(crate) struct WorkerProducer<'l, S, C, W> {
    state: Option<S>,
    completed: &'l C,
    work: &'l W,
}

impl<'f, S, C, W> Iterator for WorkerProducer<'f, S, C, W>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    W: Fn(&mut S, usize) + Sync,
{
    type Item = S;
    // One-shot iterator
    // If the iterator is finished, yield the state, else finish the work and yield the state
    // This always consumes the internal state
    fn next(&mut self) -> Option<Self::Item> {
        if self.state.as_ref().map_or(false, self.completed) {
            self.state.take()
        } else {
            self.state.take().map(|mut s| {
                (self.work)(&mut s, usize::MAX);
                s
            })
        }
    }
}

impl<'f, S, C, W> Divisible for WorkerProducer<'f, S, C, W>
where
    S: Divisible + Send,
    C: Fn(&S) -> bool + Sync,
    W: Fn(&mut S, usize) + Sync,
{
    type Controlled = S::Controlled;
    fn should_be_divided(&self) -> bool {
        self.state.as_ref().map_or(false, S::should_be_divided)
    }
    fn divide(self) -> (Self, Self) {
        let (lefts, rights) = self.state.map_or((None, None), |s| {
            let (l, r) = s.divide();
            (Some(l), Some(r))
        });
        (
            WorkerProducer {
                state: lefts,
                completed: self.completed,
                work: self.work,
            },
            WorkerProducer {
                state: rights,
                completed: self.completed,
                work: self.work,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (lefts, rights) = self.state.map_or((None, None), |s| {
            let (l, r) = s.divide_at(index);
            (Some(l), Some(r))
        });
        (
            WorkerProducer {
                state: lefts,
                completed: self.completed,
                work: self.work,
            },
            WorkerProducer {
                state: rights,
                completed: self.completed,
                work: self.work,
            },
        )
    }
}

impl<'f, S, C, W> Producer for WorkerProducer<'f, S, C, W>
where
    S: Divisible + Send,
    C: Fn(&S) -> bool + Sync,
    W: Fn(&mut S, usize) + Sync,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        if let Some(state) = &self.state {
            if (self.completed)(state) {
                (0, Some(0))
            } else {
                (1, None)
            }
        } else {
            (0, Some(0))
        }
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
    fn partial_fold<B, F>(&mut self, init: B, _fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        let work = self.work;
        self.state.as_mut().map(|s| work(s, limit));
        init
    }
}
