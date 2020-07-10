use crate::adaptors::composed::ALLOW_PARALLELISM;
use crate::prelude::*;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Tries to limit parallel composition by switching off the ability to
/// divide in parallel after a certain level of composition and upper task
/// completion.
pub struct ComposedCounter<I> {
    pub base: I,
    pub counter: AtomicU64,
    pub threshold: usize,
}

impl<I: ParallelIterator> ParallelIterator for ComposedCounter<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let composed_counter_consumer = ComposedCounter {
            base: consumer,
            counter: self.counter,
            threshold: self.threshold,
        };
        self.base.drive(composed_counter_consumer)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<'a, CB> {
            callback: CB,
            counter_ref: &'a AtomicU64,
            threshold: usize,
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
                self.callback.call(ComposedCounterProducer {
                    base: producer,
                    initial_size,
                    work_count: self.counter_ref,
                    threshold: self.threshold,
                })
            }
        }
        self.base.with_producer(Callback {
            callback,
            counter_ref: &self.counter,
            threshold: self.threshold,
        })
    }
}

struct ComposedCounterProducer<'a, I> {
    base: I,
    initial_size: usize,
    work_count: &'a AtomicU64,
    threshold: usize,
}

impl<'a, I> ComposedCounterProducer<'a, I>
where
    Self: Iterator,
{
    fn at_limit(&self) -> bool {
        let my_size = self.size_hint().0;
        let work_count = self.work_count.load(Ordering::Relaxed) as usize;
        let total_work = self.initial_size - self.threshold;

        my_size + work_count >= total_work
    }
}

impl<'a, I> Iterator for ComposedCounterProducer<'a, I>
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

        ALLOW_PARALLELISM.with(|b| {
            let allowed = b.load(Ordering::Relaxed);
            if allowed {
                if next_work_size > 1 {
                    b.store(false, Ordering::Relaxed);
                }
            }

            self.work_count
                .fetch_add(next_work_size as u64, Ordering::Relaxed);
            let result = self.base.fold(init, f);

            b.store(allowed, Ordering::Relaxed);

            result
        })
    }
}

impl<'a, I> Divisible for ComposedCounterProducer<'a, I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;

    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            ComposedCounterProducer {
                base: left,
                initial_size: self.initial_size,
                work_count: self.work_count.clone(),
                threshold: self.threshold,
            },
            ComposedCounterProducer {
                base: right,
                initial_size: self.initial_size,
                work_count: self.work_count,
                threshold: self.threshold,
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            ComposedCounterProducer {
                base: left,
                initial_size: self.initial_size,
                work_count: self.work_count.clone(),
                threshold: self.threshold,
            },
            ComposedCounterProducer {
                base: right,
                initial_size: self.initial_size,
                work_count: self.work_count,
                threshold: self.threshold,
            },
        )
    }

    fn should_be_divided(&self) -> bool {
        let at_limit = self.at_limit() && self.size_hint().0 > 1;

        ALLOW_PARALLELISM.with(|b| {
            let base = self.base.should_be_divided();
            let allowed = b.load(Ordering::Relaxed);

            allowed && (at_limit || base)
        })
    }
}

impl<'a, I> Producer for ComposedCounterProducer<'a, I>
where
    I: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.sizes()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
    fn scheduler<'r, P, T, R>(&self) -> &'r dyn Fn(P, &'r R) -> T
    where
        P: Producer<Item = T>,
        T: Send,
        R: Reducer<T>,
    {
        self.base.scheduler()
    }
}

// consumer
impl<C: Clone> Clone for ComposedCounter<C> {
    fn clone(&self) -> Self {
        ComposedCounter {
            base: self.base.clone(),
            counter: AtomicU64::new(self.counter.load(Ordering::SeqCst)),
            threshold: self.threshold,
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for ComposedCounter<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let initial_size = producer.length();
        let composed_counter_producer = ComposedCounterProducer {
            base: producer,
            initial_size,
            work_count: &self.counter,
            threshold: self.threshold,
        };
        self.base.consume_producer(composed_counter_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
