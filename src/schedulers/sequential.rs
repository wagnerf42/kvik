//! sequential scheduler
use crate::prelude::*;

pub(crate) struct SequentialScheduler;

impl<P, R> Scheduler<P, R> for SequentialScheduler
where
    P: Producer,
    P::Item: Send,
    R: Reducer<P::Item>,
{
    fn schedule(&self, producer: P, reducer: &R) -> P::Item {
        reducer.fold(producer)
    }
}
