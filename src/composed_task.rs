use crate::composed::ALLOW_PARALLELISM;
use crate::prelude::*;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub struct ComposedTask<I> {
    pub base: I,
    pub counter: AtomicU64,
}

impl<I: ParallelIterator> ParallelIterator for ComposedTask<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<'a, CB> {
            callback: CB,
            counter: &'a AtomicU64,
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
                self.callback.call(ComposedTaskProducer {
                    base: producer,
                    counter: self.counter,
                })
            }
        }

        self.base.with_producer(Callback {
            callback,
            counter: &self.counter,
        })
    }
}

struct ComposedTaskProducer<'a, I> {
    base: I,
    counter: &'a AtomicU64,
}

impl<'a, I> Iterator for ComposedTaskProducer<'a, I>
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
        ALLOW_PARALLELISM.with(|b| {
            let allowed = b.load(Ordering::Relaxed);
            if allowed {
                if self.size_hint().0 != 1 {
                    b.store(false, Ordering::Relaxed);
                }
            }

            self.counter.fetch_sub(1, Ordering::Relaxed);

            let result = self.base.fold(init, f);

            b.store(allowed, Ordering::Relaxed);

            result
        })
    }
}

impl<'a, I> Divisible for ComposedTaskProducer<'a, I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;

    fn divide(self) -> (Self, Self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
        let (left, right) = self.base.divide();

        (
            ComposedTaskProducer {
                base: left,
                counter: self.counter,
            },
            ComposedTaskProducer {
                base: right,
                counter: self.counter,
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
        let (left, right) = self.base.divide_at(index);

        (
            ComposedTaskProducer {
                base: left,
                counter: self.counter,
            },
            ComposedTaskProducer {
                base: right,
                counter: self.counter,
            },
        )
    }

    fn should_be_divided(&self) -> bool {
        ALLOW_PARALLELISM.with(|b| {
            let allowed = b.load(Ordering::Relaxed);
            let last_task = self.counter.load(Ordering::Relaxed) == 1 && self.size_hint().0 > 1;

            allowed && (self.base.should_be_divided() || last_task)
        })
    }
}

impl<'a, I> Producer for ComposedTaskProducer<'a, I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}
