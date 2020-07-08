use crate::prelude::*;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

thread_local! {
    pub static RAYON_COUNTER: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(usize::MAX));
}

pub struct ComposedSize<I> {
    pub base: I,
    pub reset_counter: usize,
}

impl<I: ParallelIterator> ParallelIterator for ComposedSize<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<CB> {
            callback: CB,
            reset_counter: usize,
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
                RAYON_COUNTER.with(|c| {
                    let counter = std::cmp::min(self.reset_counter, c.load(Ordering::Relaxed));
                    self.callback.call(ComposedSizeProducer {
                        base: producer,
                        reset_counter: self.reset_counter,
                        created_by: rayon::current_thread_index().unwrap(),
                        counter,
                    })
                })
            }
        }
        self.base.with_producer(Callback {
            callback,
            reset_counter: self.reset_counter,
        })
    }
}

struct ComposedSizeProducer<I> {
    base: I,
    reset_counter: usize,
    created_by: usize,
    counter: usize,
}

impl<I> Iterator for ComposedSizeProducer<I>
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
        RAYON_COUNTER.with(|c| {
            let old = c.swap(self.counter, Ordering::Relaxed);
            let result = self.base.fold(init, f);
            c.store(old, Ordering::Relaxed);
            result
        })
    }
}

impl<I> Divisible for ComposedSizeProducer<I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;

    fn divide(self) -> (Self, Self) {
        let current_thread = rayon::current_thread_index().unwrap_or(usize::MAX);
        let new_counter = if current_thread == self.created_by {
            self.counter.saturating_sub(1)
        } else {
            self.reset_counter - 1
        };
        let (left, right) = self.base.divide();
        (
            ComposedSizeProducer {
                base: left,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
            ComposedSizeProducer {
                base: right,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        let current_thread = rayon::current_thread_index().unwrap_or(usize::MAX);
        let new_counter = if current_thread == self.created_by {
            self.counter.saturating_sub(1)
        } else {
            self.reset_counter - 1
        };
        let (left, right) = self.base.divide_at(index);
        (
            ComposedSizeProducer {
                base: left,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
            ComposedSizeProducer {
                base: right,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
        )
    }

    fn should_be_divided(&self) -> bool {
        (self.counter != 0
            || self.created_by != rayon::current_thread_index().unwrap_or(usize::MAX))
            && self.base.should_be_divided()
    }
}

impl<I> Producer for ComposedSizeProducer<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}
