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
                    base,
                    limit: self.limit,
                };
                self.callback.call(producer)
            }
        }
    }
}

struct CapProducer<'l, I> {
    base: I,
    limit: &'l AtomicIsize,
}

impl<'l, I> Iterator for CapProducer<'l, I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
    // TODO: we should also do try_fold
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let r = self.base.fold(init, f);
        self.limit.fetch_add(1, Ordering::SeqCst);
        r
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
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided() && {
            let l = self.limit.fetch_sub(1, Ordering::SeqCst);
            if l >= 0 {
                true
            } else {
                self.limit.fetch_add(1, Ordering::SeqCst);
                false
            }
        }
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            CapProducer {
                base: left,
                limit: self.limit,
            },
            CapProducer {
                base: right,
                limit: self.limit,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            CapProducer {
                base: left,
                limit: self.limit,
            },
            CapProducer {
                base: right,
                limit: self.limit,
            },
        )
    }
}

impl<'l, I> Producer for CapProducer<'l, I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}

impl<'l, I> PreviewableParallelIterator for Cap<'l, I> where I: PreviewableParallelIterator {}
