//! Easiest parallel scheduler.
use crate::prelude::*;

pub(crate) fn schedule_join<P, R>(producer: P, reducer: &R) -> P::Item
where
    P: Producer,
    P::Item: Send,
    R: Reducer<P::Item>,
{
    if producer.should_be_divided() {
        let (left, right) = producer.divide();
        let (left_r, right_r) = rayon::join(
            || schedule_join(left, reducer),
            || schedule_join(right, reducer),
        );
        reducer.reduce(left_r, right_r)
    } else {
        producer.fold(reducer.identity(), |a, b| reducer.reduce(a, b))
    }
}
