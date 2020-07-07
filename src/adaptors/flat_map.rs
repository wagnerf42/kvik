//! finally, the freaking monad.
use super::map::MapProducer;
use crate::prelude::*;
use crate::traits::schedule_join;

pub struct FlatMap<I, F> {
    pub(crate) base: I,
    pub(crate) map_op: F,
}

impl<PI, I, F> ParallelIterator for FlatMap<I, F>
where
    I: ParallelIterator,
    F: Fn(I::Item) -> PI + Sync + Send,
    PI: IntoParallelIterator,
{
    type Item = PI::Item;
    type Controlled = I::Controlled;
    type Enumerable = False;

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let flatmap_consumer = FlatMapConsumer {
            base: consumer,
            map_op: &self.map_op,
        };
        self.base.drive(flatmap_consumer)
    }
    fn with_producer<CB>(self, _callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        panic!("I don't think we can call that")
    }
}

pub struct FlatMapConsumer<'f, C, F> {
    base: C,
    map_op: &'f F,
}

impl<'f, C: Clone, F> Clone for FlatMapConsumer<'f, C, F> {
    fn clone(&self) -> Self {
        FlatMapConsumer {
            base: self.base.clone(),
            map_op: self.map_op,
        }
    }
}

impl<'f, Item, C, F, PI> Consumer<Item> for FlatMapConsumer<'f, C, F>
where
    F: Fn(Item) -> PI + Send + Sync,
    PI: IntoParallelIterator,
    C: Consumer<PI::Item>,
{
    type Result = C::Result;
    type Reducer = C::Reducer;

    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let inner = |i| (self.map_op)(i).into_par_iter().drive(self.base.clone());
        let map_producer = MapProducer {
            base: producer,
            op: &inner,
        };
        schedule_join(map_producer, &self.clone().to_reducer())
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
