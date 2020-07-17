use crate::prelude::*;

pub struct ByBlocks<I, S> {
    pub(crate) base: I,
    pub(crate) blocks_sizes: Option<S>,
}

// producer
impl<I: Iterator, S> Iterator for ByBlocks<I, S> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}

impl<P: Producer, S: Clone> Divisible for ByBlocks<P, S> {
    type Controlled = P::Controlled;
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            ByBlocks {
                base: left,
                blocks_sizes: self.blocks_sizes.clone(),
            },
            ByBlocks {
                base: right,
                blocks_sizes: self.blocks_sizes,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            ByBlocks {
                base: left,
                blocks_sizes: self.blocks_sizes.clone(),
            },
            ByBlocks {
                base: right,
                blocks_sizes: self.blocks_sizes,
            },
        )
    }
}

impl<Q: Producer, S: Clone + Send> Producer for ByBlocks<Q, S> {
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
impl<C: Clone, S: Clone> Clone for ByBlocks<C, S> {
    fn clone(&self) -> Self {
        ByBlocks {
            base: self.base.clone(),
            blocks_sizes: self.blocks_sizes.clone(),
        }
    }
}

impl<Item, C: Consumer<Item>, S: Iterator<Item = usize> + Clone + Send + Sync> Consumer<Item>
    for ByBlocks<C, S>
{
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(mut self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let sizes = self
            .blocks_sizes
            .take()
            .expect("no sizes left in by_blocks");
        // let's get a sequential iterator of producers of increasing sizes
        let producers = sizes.scan(Some(producer), |p, s| {
            let remaining_producer = p.take().unwrap();
            let (_, upper_bound) = remaining_producer.size_hint();
            let capped_size = if let Some(bound) = upper_bound {
                if bound == 0 {
                    return None;
                } else {
                    s.min(bound)
                }
            } else {
                s
            };
            let (left, right) = remaining_producer.divide_at(capped_size);
            *p = Some(right);
            Some(left)
        });
        self.base
            .clone()
            .to_reducer()
            .fold(&mut producers.map(|p| self.base.clone().consume_producer(p)))
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

// iterator

impl<I, S> ParallelIterator for ByBlocks<I, S>
where
    I: ParallelIterator,
    S: Iterator<Item = usize> + Clone + Send + Sync,
{
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = False;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let consumer = ByBlocks {
            base: consumer,
            blocks_sizes: self.blocks_sizes,
        };
        self.base.drive(consumer)
    }
    fn with_producer<CB>(self, _callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        panic!("scheduling policies must be called as a consumer")
    }
}
