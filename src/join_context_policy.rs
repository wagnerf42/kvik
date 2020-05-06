use crate::prelude::*;

struct JoinContextPolicyProducer<I> {
    base: I,
    lower_limit: u32,
    my_creator: usize,
}

impl<I> Iterator for JoinContextPolicyProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
}

impl<I> Divisible for JoinContextPolicyProducer<I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        let me = rayon::current_thread_index().unwrap_or(0);
        (
            JoinContextPolicyProducer {
                base: left,
                lower_limit: self.lower_limit.saturating_sub(1),
                my_creator: me,
            },
            JoinContextPolicyProducer {
                base: right,
                lower_limit: 0,
                my_creator: me,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        let me = rayon::current_thread_index().unwrap_or(0);
        (
            JoinContextPolicyProducer {
                base: left,
                lower_limit: self.lower_limit.saturating_sub(1),
                my_creator: me,
            },
            JoinContextPolicyProducer {
                base: right,
                lower_limit: 0,
                my_creator: me,
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        let me = rayon::current_thread_index().unwrap_or(0);
        (self.lower_limit > 0 || me != self.my_creator) && self.base.should_be_divided()
    }
}

impl<I> Producer for JoinContextPolicyProducer<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}

pub struct JoinContextPolicy<I> {
    pub base: I,
    pub lower_limit: u32,
}

impl<I: ParallelIterator> ParallelIterator for JoinContextPolicy<I> {
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
                self.callback.call(JoinContextPolicyProducer {
                    base: producer,
                    lower_limit: self.limit,
                    my_creator: rayon::current_thread_index().unwrap_or(0),
                })
            }
        }
        self.base.with_producer(Callback {
            callback,
            limit: self.lower_limit,
        })
    }
}
