//! sequential scheduler
use crate::prelude::*;

pub(crate) fn schedule_sequential<P, T, R>(producer: P, reducer: &R) -> T
where
    P: Producer<Item = T>,
    T: Send,
    R: Reducer<T>,
{
    producer.fold(reducer.identity(), |a, b| reducer.reduce(a, b))
}
