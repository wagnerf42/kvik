use crate::prelude::*;

pub struct Sequential<I> {
    pub(crate) base: I,
}

// producer
impl<I: Iterator> Iterator for Sequential<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}

impl<P: Producer> Divisible for Sequential<P> {
    type Controlled = P::Controlled;
    fn should_be_divided(&self) -> bool {
        false
    }
    fn divide(self) -> (Self, Self) {
        panic!("please do not divide sequential code")
    }
    fn divide_at(self, _index: usize) -> (Self, Self) {
        panic!("please do not divide sequential code")
    }
}

impl<Q: Producer> Producer for Sequential<Q> {
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
        &sequential_scheduler
    }
}

// consumer
impl<C: Clone> Clone for Sequential<C> {
    fn clone(&self) -> Self {
        Sequential {
            base: self.base.clone(),
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for Sequential<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let sequential_producer = Sequential { base: producer };
        self.base.consume_producer(sequential_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

// iterator

impl<I> ParallelIterator for Sequential<I>
where
    I: ParallelIterator,
{
    type Item = I::Item;
    type Controlled = False;
    type Enumerable = False;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let sequential_consumer = Sequential { base: consumer };
        self.base.drive(sequential_consumer)
    }
    fn with_producer<CB>(self, _callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        panic!("scheduling policies must be called as a consumer")
    }
}

pub(crate) fn sequential_scheduler<P, T, R>(producer: P, reducer: &R) -> T
where
    P: Producer<Item = T>,
    T: Send,
    R: Reducer<T>,
{
    producer.fold(reducer.identity(), |a, b| reducer.reduce(a, b))
}
