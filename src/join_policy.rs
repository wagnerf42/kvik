use crate::prelude::*;
//As of now this won't really work with the adaptive scheduler. Need to think what it even means in
//that context?
struct JoinPolicyProducer<I> {
    base: I,
    limit: usize,
}

impl<I> Iterator for JoinPolicyProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
}

impl<I> Divisible for JoinPolicyProducer<I>
where
    I: Divisible + Iterator,
{
    type Power = <I as Divisible>::Power;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            JoinPolicyProducer {
                base: left,
                limit: self.limit,
            },
            JoinPolicyProducer {
                base: right,
                limit: self.limit,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            JoinPolicyProducer {
                base: left,
                limit: self.limit,
            },
            JoinPolicyProducer {
                base: right,
                limit: self.limit,
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided() && self.base.size_hint().0 > self.limit
    }
}

pub struct JoinPolicy<I> {
    pub base: I,
    pub limit: usize,
}

impl<I: ParallelIterator> ParallelIterator for JoinPolicy<I> {
    type Item = I::Item;
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
            fn call<P>(&self, producer: P) -> Self::Output
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
