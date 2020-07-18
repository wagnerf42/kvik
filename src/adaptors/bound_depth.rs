use crate::prelude::*;
//TODO: As of now this won't really work with the adaptive scheduler. Need to think what it even means in
//that context?

pub struct BoundDepth<I> {
    pub(crate) base: I,
    pub(crate) limit: u32,
}

// producer
impl<I> Iterator for BoundDepth<I>
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

impl<P> Divisible for BoundDepth<P>
where
    P: Producer,
{
    type Controlled = <P as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            BoundDepth {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            BoundDepth {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            BoundDepth {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            BoundDepth {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.limit > 0 && self.base.should_be_divided()
    }
}

impl<P> Producer for BoundDepth<P>
where
    P: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.sizes()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        self.base.partial_fold(init, fold_op, limit)
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

// consumer
impl<C: Clone> Clone for BoundDepth<C> {
    fn clone(&self) -> Self {
        BoundDepth {
            limit: self.limit,
            base: self.base.clone(),
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for BoundDepth<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let bound_depth_producer = BoundDepth {
            limit: self.limit,
            base: producer,
        };
        self.base.consume_producer(bound_depth_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

// iterator
impl<I: ParallelIterator> ParallelIterator for BoundDepth<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let bound_depth_consumer = BoundDepth {
            limit: self.limit,
            base: consumer,
        };
        self.base.drive(bound_depth_consumer)
    }
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
                self.callback.call(BoundDepth {
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
