use crate::prelude::*;
pub struct MicroBlockSizes<I> {
    pub inner: I,
    pub lower: usize,
    pub upper: usize,
}
// producer
impl<I> Iterator for MicroBlockSizes<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<P> Divisible for MicroBlockSizes<P>
where
    P: Producer,
{
    type Controlled = <P as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.inner.divide();
        (
            MicroBlockSizes {
                inner: left,
                lower: self.lower,
                upper: self.upper,
            },
            MicroBlockSizes {
                inner: right,
                lower: self.lower,
                upper: self.upper,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.inner.divide_at(index);
        (
            MicroBlockSizes {
                inner: left,
                lower: self.lower,
                upper: self.upper,
            },
            MicroBlockSizes {
                inner: right,
                lower: self.lower,
                upper: self.upper,
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.inner.should_be_divided()
    }
}

impl<P> Producer for MicroBlockSizes<P>
where
    P: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.inner.sizes()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.inner.preview(index)
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        self.inner.partial_fold(init, fold_op, limit)
    }
    fn scheduler<'s, Q: 's, R: 's>(&self) -> Box<dyn Scheduler<Q, R> + 's>
    where
        Q: Producer,
        Q::Item: Send,
        R: Reducer<Q::Item>,
    {
        self.inner.scheduler()
    }
    fn micro_block_sizes(&self) -> (usize, usize) {
        (self.lower, self.upper)
    }
}

// consumer
impl<C: Clone> Clone for MicroBlockSizes<C> {
    fn clone(&self) -> Self {
        MicroBlockSizes {
            inner: self.inner.clone(),
            lower: self.lower,
            upper: self.upper,
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for MicroBlockSizes<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let bound_depth_producer = MicroBlockSizes {
            inner: producer,
            lower: self.lower,
            upper: self.upper,
        };
        self.inner.consume_producer(bound_depth_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.inner.to_reducer()
    }
}

// iterator
impl<I: ParallelIterator> ParallelIterator for MicroBlockSizes<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let bound_depth_consumer = MicroBlockSizes {
            inner: consumer,
            lower: self.lower,
            upper: self.upper,
        };
        self.inner.drive(bound_depth_consumer)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<CB> {
            callback: CB,
            lower: usize,
            upper: usize,
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
                self.callback.call(MicroBlockSizes {
                    inner: producer,
                    lower: self.lower,
                    upper: self.upper,
                })
            }
        }
        self.inner.with_producer(Callback {
            callback,
            lower: self.lower,
            upper: self.upper,
        })
    }
}
