//! sequential scheduler
use crate::prelude::*;

pub(crate) fn schedule_sequential<P, R>(producer: P, reducer: &R) -> P::Item
where
    P: Producer,
    R: Reducer<P::Item>,
{
    producer.fold(reducer.identity(), |a, b| reducer.reduce(a, b))
}
