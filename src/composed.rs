use crate::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

pub static WORK_COUNT: AtomicU64 = AtomicU64::new(0);

thread_local! {
    pub static ALLOW_PARALLELISM: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

/// Tries to limit parallel composition by switching off the ability to
/// divide in parallel after a certain level of composition and upper task
/// completion.
pub struct Composed<I> {
    pub base: I,
}

impl<I: ParallelIterator> ParallelIterator for Composed<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<CB> {
            callback: CB,
        }
        impl<CB, T> ProducerCallback<T> for Callback<CB>
        where
            CB: ProducerCallback<T>,
        {
            type Output = CB::Output;

            fn call<P>(self, producer: P) -> Self::Output
            where
                P: Producer<Item = T>,
            {
                let initial_size = producer.length();
                self.callback.call(ComposedProducer {
                    base: producer,
                    initial_size,
                })
            }
        }
        self.base.with_producer(Callback { callback })
    }
}

struct ComposedProducer<I> {
    base: I,
    initial_size: usize,
}

impl<I> Iterator for ComposedProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }

    fn fold<B, F>(mut self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        println!("fold in ComposedProducer");
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
                initial_size: self.initial_size,
            },
            ComposedProducer {
                base: right,
                initial_size: self.initial_size,
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            ComposedProducer {
                base: left,
                initial_size: self.initial_size,
            },
            ComposedProducer {
                base: right,
                initial_size: self.initial_size,
            },
        )
    }

    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
}

impl<I> Producer for ComposedProducer<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}
