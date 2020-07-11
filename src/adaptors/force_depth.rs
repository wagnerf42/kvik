use crate::prelude::*;
//As of now this won't really work with the adaptive scheduler. Need to think what it even means in
//that context?

pub struct ForceDepth<I> {
    pub base: I,
    pub limit: u32,
}

// producer
impl<I> Iterator for ForceDepth<I>
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

impl<P> Divisible for ForceDepth<P>
where
    P: Producer,
{
    type Controlled = <P as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            ForceDepth {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            ForceDepth {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            ForceDepth {
                base: left,
                limit: self.limit.saturating_sub(1),
            },
            ForceDepth {
                base: right,
                limit: self.limit.saturating_sub(1),
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.limit > 0 || (self.limit == 0 && self.base.should_be_divided())
    }
}

impl<Q> Producer for ForceDepth<Q>
where
    Q: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.base.sizes()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
    fn scheduler<'s, P: 's, R: 's>(&self) -> Box<dyn Scheduler<P, R> + 's>
    where
        P: Producer,
        P::Item: Send,
        R: Reducer<P::Item>,
    {
        self.base.scheduler()
    }
}

// consumer
impl<C: Clone> Clone for ForceDepth<C> {
    fn clone(&self) -> Self {
        ForceDepth {
            base: self.base.clone(),
            limit: self.limit,
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for ForceDepth<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let force_depth_producer = ForceDepth {
            base: producer,
            limit: self.limit,
        };
        self.base.consume_producer(force_depth_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

// iterator
impl<I: ParallelIterator> ParallelIterator for ForceDepth<I> {
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
                self.callback.call(ForceDepth {
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
