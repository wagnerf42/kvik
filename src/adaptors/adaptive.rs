//! turn on adaptive scheduler.
//! it's a good example of all the stuff we need for parallel iterators.
use crate::prelude::*;

pub struct Adaptive<I> {
    pub(crate) base: I,
}

// easy stuff : producer
impl<P: Producer> Iterator for Adaptive<P> {
    type Item = P::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
}

impl<P: Producer> Divisible for Adaptive<P> {
    type Controlled = P::Controlled;
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (Adaptive { base: left }, Adaptive { base: right })
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (Adaptive { base: left }, Adaptive { base: right })
    }
}

impl<P: Producer> Producer for Adaptive<P> {
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.sizes()
    }

    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
    fn scheduler<'r, Q, T, R>(&self) -> &'r dyn Fn(Q, &'r R) -> T
    where
        Q: Producer<Item = T>,
        T: Send,
        R: Reducer<T>,
    {
        &crate::adaptive::schedule_adapt
    }
}

// hard stuff: parallel iterator
impl<I: ParallelIterator> ParallelIterator for Adaptive<I> {
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        self.base.drive(Adaptive { base: consumer })
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        panic!("scheduling policies cannot be called here")
    }
}

// medium stuff : consumer
impl<C: Clone> Clone for Adaptive<C> {
    fn clone(&self) -> Self {
        Adaptive {
            base: self.base.clone(),
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for Adaptive<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let adapt_producer = Adaptive { base: producer };
        self.base.consume_producer(adapt_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
