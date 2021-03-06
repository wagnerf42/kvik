// use crate::adaptive::AdaptiveProducer;
use crate::prelude::*;
use std::sync::atomic::{AtomicIsize, Ordering};

pub struct Cap<'l, I> {
    pub(crate) base: I,
    pub(crate) limit: &'l AtomicIsize,
}

impl<'l, I> ParallelIterator for Cap<'l, I>
where
    I: ParallelIterator,
{
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let cap_consumer = Cap {
            base: consumer,
            limit: self.limit,
        };
        self.base.drive(cap_consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.base.with_producer(Callback {
            callback,
            limit: self.limit,
        });
        struct Callback<'l, CB> {
            callback: CB,
            limit: &'l AtomicIsize,
        }

        impl<'l, T, CB> ProducerCallback<T> for Callback<'l, CB>
        where
            CB: ProducerCallback<T>,
        {
            type Output = CB::Output;
            fn call<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = T>,
            {
                self.limit.fetch_sub(1, Ordering::SeqCst);
                let producer = CapProducer {
                    base: Some(base),
                    limit: self.limit,
                    real_drop: true,
                };
                self.callback.call(producer)
            }
        }
    }
}

struct CapProducer<'l, I> {
    base: Option<I>,
    limit: &'l AtomicIsize,
    real_drop: bool, // if false we don't change counter when dropped
}

impl<'l, I> Iterator for CapProducer<'l, I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.as_ref().map(|b| b.size_hint()).unwrap()
    }
    fn next(&mut self) -> Option<Self::Item> {
        self.base.as_mut().and_then(|b| b.next())
    }
    fn fold<B, F>(mut self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.base.take().unwrap().fold(init, f)
    }
    #[cfg(feature = "nightly")]
    fn try_fold<B, F, R>(&mut self, init: B, f: F) -> R
    where
        F: FnMut(B, Self::Item) -> R,
        R: std::ops::Try<Ok = B>,
    {
        self.base.as_mut().unwrap().try_fold(init, f)
    }
}

impl<'l, I> DoubleEndedIterator for CapProducer<'l, I>
where
    I: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.base.as_mut().and_then(|b| b.next_back())
    }
}

impl<'l, I> Divisible for CapProducer<'l, I>
where
    I: Producer,
{
    type Controlled = I::Controlled;
    //TODO: there is an atomicity issue here.
    //it should be fine on the condition that
    //should_be_divided is not called twice for a division.
    //are we sure this is always true ?
    //more serious: we say yes but we are not divided
    fn should_be_divided(&self) -> bool {
        self.base
            .as_ref()
            .map(|b| b.should_be_divided())
            .unwrap_or(false)
            && {
                let l = self.limit.fetch_sub(1, Ordering::SeqCst);
                if l >= 0 {
                    true
                } else {
                    self.limit.fetch_add(1, Ordering::SeqCst);
                    false
                }
            }
    }
    fn divide(mut self) -> (Self, Self) {
        let (left, right) = self.base.take().unwrap().divide();
        self.real_drop = false;
        (
            CapProducer {
                base: Some(left),
                limit: self.limit,
                real_drop: true,
            },
            CapProducer {
                base: Some(right),
                limit: self.limit,
                real_drop: true,
            },
        )
    }
    fn divide_at(mut self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.take().unwrap().divide_at(index);
        self.real_drop = false;
        (
            CapProducer {
                base: Some(left),
                limit: self.limit,
                real_drop: true,
            },
            CapProducer {
                base: Some(right),
                limit: self.limit,
                real_drop: true,
            },
        )
    }
}

impl<'l, I> std::ops::Drop for CapProducer<'l, I> {
    fn drop(&mut self) {
        if self.real_drop {
            self.limit.fetch_add(1, Ordering::SeqCst);
        }
    }
}

impl<'l, I> Producer for CapProducer<'l, I>
where
    I: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.as_ref().map(|b| b.sizes()).unwrap()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.as_ref().map(|b| b.preview(index)).unwrap()
    }
    fn scheduler<'s, P: 's, R: 's>(&self) -> Box<dyn Scheduler<P, R> + 's>
    where
        P: Producer,
        P::Item: Send,
        R: Reducer<P::Item>,
    {
        self.base.as_ref().map(|b| b.scheduler()).unwrap()
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        match self.base.as_mut() {
            Some(inner) => inner.partial_fold(init, fold_op, limit),
            None => init,
        }
    }
    fn micro_block_sizes(&self) -> (usize, usize) {
        self.base
            .as_ref()
            .map(|inner| inner.micro_block_sizes())
            .unwrap_or((1, usize::MAX))
    }
}

impl<'l, I> PreviewableParallelIterator for Cap<'l, I> where I: PreviewableParallelIterator {}

// consumer

impl<'l, C: Clone> Clone for Cap<'l, C> {
    fn clone(&self) -> Self {
        Cap {
            base: self.base.clone(),
            limit: self.limit,
        }
    }
}

impl<'l, Item, C: Consumer<Item>> Consumer<Item> for Cap<'l, C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let cap_producer = CapProducer {
            base: Some(producer),
            limit: self.limit,
            real_drop: true,
        };
        self.base.consume_producer(cap_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
