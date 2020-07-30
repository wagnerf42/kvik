//! iterator in reverse order.
use crate::prelude::*;

pub struct Rev<I> {
    pub(crate) base: I,
}

impl<I: DoubleEndedIterator> Iterator for Rev<I> {
    type Item = I::Item;
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next_back()
    }
}
impl<I: DoubleEndedIterator> DoubleEndedIterator for Rev<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
}

impl<P> Divisible for Rev<P>
where
    P: Producer,
{
    type Controlled = <P as Divisible>::Controlled;
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (Rev { base: right }, Rev { base: left })
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let index = self.base.length() - index;
        let (left, right) = self.base.divide_at(index);
        (Rev { base: right }, Rev { base: left })
    }
}

impl<P> Producer for Rev<P>
where
    P: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.sizes()
    }
    fn partial_fold<B, F>(&mut self, _init: B, _fold_op: F, _limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        unimplemented!("we need a reversed partial fold!")
    }
    fn length(&self) -> usize {
        self.base.length()
    }
    fn preview(&self, index: usize) -> Self::Item {
        let index = self.length() - index;
        self.base.preview(index)
    }
    fn scheduler<'s, Q: 's, R: 's>(&self) -> Box<dyn Scheduler<Q, R> + 's>
    where
        Q: Producer,
        Q::Item: Send,
        R: Reducer<Q::Item>,
    {
        self.base.scheduler()
    }
    fn micro_block_sizes(&self) -> (usize, usize) {
        self.base.micro_block_sizes()
    }
}

impl<C: Clone> Clone for Rev<C> {
    fn clone(&self) -> Self {
        Rev {
            base: self.base.clone(),
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for Rev<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let rev_producer = Rev { base: producer };
        self.base.consume_producer(rev_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

impl<I: ParallelIterator> ParallelIterator for Rev<I> {
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let rev_consumer = Rev { base: consumer };
        self.base.drive(rev_consumer)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        unimplemented!("TODO")
    }
}
