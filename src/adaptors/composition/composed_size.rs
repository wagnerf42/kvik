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
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let composed_size_consumer = ComposedSize {
            base: consumer,
            reset_counter: self.reset_counter,
        };
        self.base.drive(composed_size_consumer)
    }

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

impl<I> DoubleEndedIterator for ComposedSizeProducer<I>
where
    I: DoubleEndedIterator,
{
    //TODO: rfold on all of these
    fn next_back(&mut self) -> Option<Self::Item> {
        self.base.next_back()
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
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.sizes()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
    fn scheduler<'s, P: 's, R: 's>(&self) -> Box<dyn Scheduler<P, R> + 's>
    where
        P: Producer,
        P::Item: Send,
        R: Reducer<P::Item>,
    {
        self.base.scheduler()
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        self.base.partial_fold(init, fold_op, limit)
    }
}

// consumer

impl<C: Clone> Clone for ComposedSize<C> {
    fn clone(&self) -> Self {
        ComposedSize {
            base: self.base.clone(),
            reset_counter: self.reset_counter,
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for ComposedSize<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let counter =
            RAYON_COUNTER.with(|c| std::cmp::min(self.reset_counter, c.load(Ordering::Relaxed)));
        let composed_sized_producer = ComposedSizeProducer {
            base: producer,
            reset_counter: self.reset_counter,
            created_by: rayon::current_thread_index().unwrap_or(std::usize::MAX),
            counter,
        };
        self.base.consume_producer(composed_sized_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
