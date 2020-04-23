use crate::prelude::*;

pub struct Merge<A, B> {
    pub(crate) a: A,
    pub(crate) b: B,
}

impl<A, B> ParallelIterator for Merge<A, B>
where
    A::Item: Ord,
    A: ParallelIterator,
    B: ParallelIterator<Item = A::Item>,
{
    type Controlled = False;
    type Enumerable = True;
    type Item = A::Item;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.a.with_producer(CallbackA {
            callback,
            b: self.b,
        });

        struct CallbackA<CB, B> {
            callback: CB,
            b: B,
        }

        impl<CB, B> ProducerCallback<B::Item> for CallbackA<CB, B>
        where
            B::Item: Ord,
            B: ParallelIterator,
            CB: ProducerCallback<B::Item>,
        {
            type Output = CB::Output;

            fn call<A>(self, a_producer: A) -> Self::Output
            where
                A: Producer<Item = B::Item>,
            {
                self.b.with_producer(CallbackB {
                    a_producer,
                    callback: self.callback,
                })
            }
        }

        struct CallbackB<CB, A> {
            a_producer: A,
            callback: CB,
        }

        impl<CB, A> ProducerCallback<A::Item> for CallbackB<CB, A>
        where
            A: Producer,
            A::Item: Ord,
            CB: ProducerCallback<A::Item>,
        {
            type Output = CB::Output;

            fn call<B>(self, b_producer: B) -> Self::Output
            where
                B: Producer<Item = A::Item>,
            {
                self.callback.call(MergeProducer {
                    a: self.a_producer,
                    b: b_producer,
                })
            }
        }
    }
}

struct MergeProducer<A, B> {
    a: A,
    b: B,
}

impl<A, B> Iterator for MergeProducer<A, B>
where
    A: Iterator,
    A::Item: Ord,
    B: Iterator<Item = A::Item>,
{
    type Item = B::Item;
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

impl<A, B> Divisible for MergeProducer<A, B>
where
    A: Producer,
    A::Item: Ord,
    B: Producer<Item = A::Item>,
{
    type Controlled = False;
    fn should_be_divided(&self) -> bool {
        unimplemented!()
    }
    fn divide(self) -> (Self, Self) {
        unimplemented!()
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        panic!("you cannot divide_at a merge")
    }
}

impl<A, B> Producer for MergeProducer<A, B>
where
    A: Producer,
    A::Item: Ord,
    B: Producer<Item = A::Item>,
{
    fn preview(&self, index: usize) -> Self::Item {
        panic!("you cannot preview a merge")
    }
}

//TODO: I want the reduction to be adaptive here.
//However if someone calls map after the merge
//there is currently no way to forward this information
//to the reduce op
