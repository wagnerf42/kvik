//! Easiest parallel scheduler.
use crate::prelude::*;

pub(crate) fn schedule_join<P, T, R>(producer: P, reducer: &R) -> T
where
    P: Producer<Item = T>,
    T: Send,
    R: Reducer<T>,
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
