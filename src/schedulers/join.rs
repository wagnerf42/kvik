//! Easiest parallel scheduler.
use crate::prelude::*;

pub(crate) struct JoinScheduler;

impl<P, R> Scheduler<P, R> for JoinScheduler
where
    P: Producer,
    P::Item: Send,
    R: Reducer<P::Item>,
{
    fn schedule(&self, producer: P, reducer: &R) -> P::Item {
        if producer.should_be_divided() {
            let (left, right) = producer.divide();
            let (left_r, right_r) = rayon::join(
                || self.schedule(left, reducer),
                || self.schedule(right, reducer),
            );
            reducer.reduce(left_r, right_r)
        } else {
            reducer.fold(producer)
        }
    }
}
