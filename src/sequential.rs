use crate::prelude::*;

pub struct Sequential<I> {
    pub(crate) base: I,
}

struct ReduceCallback<'f, OP, ID> {
    op: &'f OP,
    identity: &'f ID,
}

impl<'f, T, OP, ID> ProducerCallback<T> for ReduceCallback<'f, OP, ID>
where
    T: Send,
    OP: Fn(T, T) -> T + Sync + Send,
    ID: Fn() -> T + Send + Sync,
{
    type Output = T;
    fn call<P>(&self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>,
    {
        producer.fold((self.identity)(), self.op)
    }
}

impl<I> ParallelIterator for Sequential<I>
where
    I: ParallelIterator,
{
    type Item = I::Item;
    fn reduce<OP, ID>(self, identity: ID, op: OP) -> Self::Item
    where
        OP: Fn(Self::Item, Self::Item) -> Self::Item + Sync + Send,
        ID: Fn() -> Self::Item + Sync + Send,
    {
        let reduce_cb = ReduceCallback {
            op: &op,
            identity: &identity,
        };
        self.with_producer(reduce_cb)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        self.base.with_producer(callback)
    }
}
