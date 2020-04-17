use crate::prelude::*;

pub struct Map<I, F> {
    pub(crate) base: I,
    pub(crate) op: F,
}

impl<R, I, F> ParallelIterator for Map<I, F>
where
    I: ParallelIterator,
    F: Fn(I::Item) -> R + Sync,
{
    type Item = R;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.base.with_producer(Callback {
            callback,
            op: self.op,
        });
        struct Callback<CB, F> {
            callback: CB,
            op: F,
        }
        impl<T, F, R, CB> ProducerCallback<T> for Callback<CB, F>
        where
            CB: ProducerCallback<R>,
            F: Fn(T) -> R + Sync,
        {
            type Output = CB::Output;
            fn call<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = T>,
            {
                let producer = MapProducer { base, op: &self.op };
                self.callback.call(producer)
            }
        }
    }
}

struct MapProducer<'f, I, F> {
    base: I,
    op: &'f F,
}

impl<'f, R, I, F> Iterator for MapProducer<'f, I, F>
where
    I: Iterator,
    F: Fn(I::Item) -> R,
{
    type Item = R;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next().map(self.op)
    }
}

impl<'f, R, I, F> Producer for MapProducer<'f, I, F>
where
    I: Producer,
    F: Fn(I::Item) -> R + Sync,
{
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            MapProducer {
                base: left,
                op: self.op,
            },
            MapProducer {
                base: right,
                op: self.op,
            },
        )
    }
}
