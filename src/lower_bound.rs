use crate::prelude::*;
//As of now this won't really work with the adaptive scheduler. Need to think what it even means in
//that context?
struct LowerBoundProducer<I> {
    base: I,
    limit: u32,
}

impl<I> Iterator for LowerBoundProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
}

impl<I> Divisible for LowerBoundProducer<I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            LowerBoundProducer {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            LowerBoundProducer {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            LowerBoundProducer {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            LowerBoundProducer {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.limit > 0 || (self.limit == 0 && self.base.should_be_divided())
    }
}

impl<I> Producer for LowerBoundProducer<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}

pub struct LowerBound<I> {
    pub base: I,
    pub limit: u32,
}

impl<I: ParallelIterator> ParallelIterator for LowerBound<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<CB> {
            callback: CB,
            limit: u32,
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
                self.callback.call(LowerBoundProducer {
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
