use crate::prelude::*;

struct JoinContextPolicyProducer<I> {
    base: I,
    limit: u32,
    is_right: bool,
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
                limit: self.limit.saturating_sub(1),
                is_right: false,
                my_creator: me,
            },
            JoinContextPolicyProducer {
                base: right,
                limit: self.limit.saturating_sub(1),
                is_right: true,
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
                limit: self.limit.saturating_sub(1),
                is_right: false,
                my_creator: me,
            },
            JoinContextPolicyProducer {
                base: right,
                limit: self.limit.saturating_sub(1),
                is_right: true,
                my_creator: me,
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        //There is an upper limit to the depth of the division tree.
        //If this limit has been reached, you don't divide.
        //Else:
        //  You don't divide if and only if you are on the right side and not stolen.
        //In all cases, it is going to ask the base about division, so basically it just has veto
        //powers
        let me = rayon::current_thread_index().unwrap_or(0);
        self.limit > 0
            && (!self.is_right || (self.is_right && me != self.my_creator))
            && self.base.should_be_divided()
    }
}

impl<I> Producer for JoinContextPolicyProducer<I>
where
    I: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.sizes()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}

pub struct JoinContextPolicy<I> {
    pub base: I,
    pub limit: u32,
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
            limit: u32,
            callback: CB,
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
                    limit: self.limit,
                    base: producer,
                    is_right: false,
                    my_creator: rayon::current_thread_index().unwrap_or(0),
                })
            }
        }
        self.base.with_producer(Callback {
            callback,
            limit: self.limit,
        })
    }
}
