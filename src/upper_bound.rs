use crate::{adaptive::AdaptiveProducer, prelude::*};
//TODO: As of now this won't really work with the adaptive scheduler. Need to think what it even means in
//that context?
pub(crate) struct JoinPolicyProducer<I> {
    pub(crate) base: I,
    pub(crate) limit: u32,
}

impl<I> Iterator for JoinPolicyProducer<I>
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

impl<I> Divisible for JoinPolicyProducer<I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            JoinPolicyProducer {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            JoinPolicyProducer {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            JoinPolicyProducer {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            JoinPolicyProducer {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.limit > 0 && self.base.should_be_divided()
    }
}
impl<I> AdaptiveProducer for JoinPolicyProducer<I>
where
    I: AdaptiveProducer,
{
    fn completed(&self) -> bool {
        self.base.completed()
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        self.base.partial_fold(init, fold_op, limit)
    }
}

impl<I> Producer for JoinPolicyProducer<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}

pub struct UpperBound<I> {
    pub base: I,
    pub limit: u32,
}

impl<I: ParallelIterator> ParallelIterator for UpperBound<I> {
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
                self.callback.call(JoinPolicyProducer {
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
