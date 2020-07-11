use crate::prelude::*;

pub trait Scheduler<P, R>
where
    P: Producer,
    P::Item: Send,
    R: Reducer<P::Item>,
{
    fn schedule(&self, producer: P, reducer: &R) -> P::Item;
}

mod adaptive;
mod depjoin;
mod join;
mod sequential;
pub(crate) use adaptive::{adaptive_scheduler, AdaptiveScheduler};
pub(crate) use depjoin::DepJoinScheduler;
pub(crate) use join::JoinScheduler;
pub(crate) use sequential::SequentialScheduler;
