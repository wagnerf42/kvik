use crate::prelude::*;
use crate::Try;

struct SizeLimitProducer<I> {
    base: I,
    limit: usize,
}

impl<I> Iterator for SizeLimitProducer<I>
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
}

impl<I> DoubleEndedIterator for SizeLimitProducer<I>
where
    I: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.base.next_back()
    }
}

impl<I> Divisible for SizeLimitProducer<I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            SizeLimitProducer {
                base: left,
                limit: self.limit,
            },
            SizeLimitProducer {
                base: right,
                limit: self.limit,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            SizeLimitProducer {
                base: left,
                limit: self.limit,
            },
            SizeLimitProducer {
                base: right,
                limit: self.limit,
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.size_hint().1.map(|s| s > self.limit).unwrap_or(true) && self.base.should_be_divided()
    }
}

impl<I> Producer for SizeLimitProducer<I>
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
    fn partial_try_fold<B, F, R>(&mut self, init: B, f: F, limit: usize) -> R
    where
        F: FnMut(B, Self::Item) -> R,
        R: Try<Ok = B>,
    {
        self.base.partial_try_fold(init, f, limit)
    }

    fn micro_block_sizes(&self) -> (usize, usize) {
        self.base.micro_block_sizes()
    }
}

pub struct SizeLimit<I> {
    pub base: I,
    pub limit: usize,
}

impl<I: Clone> Clone for SizeLimit<I> {
    fn clone(&self) -> Self {
        SizeLimit {
            base: self.base.clone(),
            limit: self.limit,
        }
    }
}

impl<I: ParallelIterator> ParallelIterator for SizeLimit<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;
    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        let c = SizeLimit {
            base: consumer,
            limit: self.limit,
        };
        self.base.drive(c)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<CB> {
            callback: CB,
            limit: usize,
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
                self.callback.call(SizeLimitProducer {
                    base: producer,
                    limit: self.limit,
                })
            }
        }
        self.base.with_producer(Callback {
            callback,
            limit: self.limit,
        })
    }
}

impl<Item, C> Consumer<Item> for SizeLimit<C>
where
    C: Consumer<Item>,
{
    type Result = C::Result;
    type Reducer = C::Reducer;

    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let limit_producer = SizeLimitProducer {
            base: producer,
            limit: self.limit,
        };
        self.base.consume_producer(limit_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
