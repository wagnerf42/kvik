use crate::prelude::*;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

thread_local! {
    pub static INHIBITOR: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

/// Tries to limit parallel composition by switching off the ability to
/// divide in parallel after a certain level of composition and upper task
/// completion.
pub struct Composed<I> {
    pub base: I,
    pub inhib_upper: Arc<AtomicBool>,
    pub inhib: AtomicBool,
}

impl<I: ParallelIterator> ParallelIterator for Composed<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        self.base.with_producer(callback)
    }
}

//TODO: add initial size
struct ComposedProducer<I> {
    base: I,
    inhib: Arc<AtomicBool>,
}

impl<I> Iterator for ComposedProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        //TODO: Update thread_local inhibitor
        self.base.fold(init, f)
    }
}

impl<I> Divisible for ComposedProducer<I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;

    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            ComposedProducer {
                base: left,
                inhib: self.inhib.clone(),
            },
            ComposedProducer {
                base: right,
                inhib: self.inhib.clone(),
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            ComposedProducer {
                base: left,
                inhib: self.inhib.clone(),
            },
            ComposedProducer {
                base: right,
                inhib: self.inhib.clone(),
            },
        )
    }

    fn should_be_divided(&self) -> bool {
        self.inhib.load(Ordering::Relaxed) && self.base.should_be_divided()
    }
}
