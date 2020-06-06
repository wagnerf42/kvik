use crate::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

thread_local! {
    pub static ALLOW_PARALLELISM: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

/// Tries to limit parallel composition by switching off the ability to
/// divide in parallel after a certain level of composition and upper task
/// completion.
pub struct Composed<I> {
    pub base: I,
    pub counter: AtomicU64,
}

impl<I: ParallelIterator> ParallelIterator for Composed<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<'a, CB> {
            callback: CB,
            counter_ref: &'a AtomicU64,
        }
        impl<'a, CB, T> ProducerCallback<T> for Callback<'a, CB>
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
                    work_count: self.counter_ref,
                })
            }
        }
        self.base.with_producer(Callback {
            callback,
            counter_ref: &self.counter,
        })
    }
}

struct ComposedProducer<'a, I> {
    base: I,
    initial_size: usize,
    work_count: &'a AtomicU64,
}

impl<'a, I> Iterator for ComposedProducer<'a, I>
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

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let next_work_size = self.base.size_hint().0;
        let previous_total_work = self.work_count.load(Ordering::Relaxed);

        if ((next_work_size as f64 + previous_total_work as f64) / self.initial_size as f64) < 0.9 {
            ALLOW_PARALLELISM.with(|b| b.store(false, Ordering::Relaxed));
        } else {
            ALLOW_PARALLELISM.with(|b| b.store(true, Ordering::Relaxed));
        }

        let result = self.base.fold(init, f);

        ALLOW_PARALLELISM.with(|b| b.fetch_and(true, Ordering::Relaxed));

        self.work_count
            .fetch_add(next_work_size as u64, Ordering::Relaxed);

        result
    }
}

impl<'a, I> Divisible for ComposedProducer<'a, I>
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
                work_count: self.work_count.clone(),
            },
            ComposedProducer {
                base: right,
                initial_size: self.initial_size,
                work_count: self.work_count,
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            ComposedProducer {
                base: left,
                initial_size: self.initial_size,
                work_count: self.work_count.clone(),
            },
            ComposedProducer {
                base: right,
                initial_size: self.initial_size,
                work_count: self.work_count,
            },
        )
    }

    fn should_be_divided(&self) -> bool {
        ALLOW_PARALLELISM.with(|b| b.load(Ordering::Relaxed) && self.base.should_be_divided())
    }
}

impl<'a, I> Producer for ComposedProducer<'a, I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}
