use crate::prelude::*;

struct JoinContextPolicyProducer<I> {
    base: I,
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
                is_right: false,
                my_creator: me,
            },
            JoinContextPolicyProducer {
                base: right,
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
                is_right: false,
                my_creator: me,
            },
            JoinContextPolicyProducer {
                base: right,
                is_right: true,
                my_creator: me,
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        // Left children can always divide (ie. defer this to the base)
        // If right child is not stolen then he won't divide
        // Else he will divide (ie. defer this to the base)
        let me = rayon::current_thread_index().unwrap_or(0);
        (!self.is_right || (self.is_right && me != self.my_creator))
            && self.base.should_be_divided()
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
                    is_right: false,
                    my_creator: rayon::current_thread_index().unwrap_or(0),
                })
            }
        }
        self.base.with_producer(Callback { callback })
    }
}
