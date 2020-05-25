use crate::prelude::*;

pub struct Filter<I, F> {
    pub(crate) base: I,
    pub(crate) filter: F,
}

impl<I, F> ParallelIterator for Filter<I, F>
where
    I: ParallelIterator,
    F: Fn(&I::Item) -> bool + Sync,
{
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.base.with_producer(Callback {
            callback,
            filter: self.filter,
        });
        struct Callback<CB, F> {
            callback: CB,
            filter: F,
        }
        impl<T, F, CB> ProducerCallback<T> for Callback<CB, F>
        where
            CB: ProducerCallback<T>,
            F: Fn(&T) -> bool + Sync,
        {
            type Output = CB::Output;
            fn call<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = T>,
            {
                let producer = FilterProducer {
                    base,
                    filter: &self.filter,
                };
                self.callback.call(producer)
            }
        }
    }
}

struct FilterProducer<'f, I, F> {
    base: I,
    filter: &'f F,
}

impl<'f, I, F> Iterator for FilterProducer<'f, I, F>
where
    I: Iterator,
    F: Fn(&I::Item) -> bool,
{
    type Item = I::Item;
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next().filter(self.filter)
    }
}

impl<'f, I, F> Divisible for FilterProducer<'f, I, F>
where
    I: Producer,
    F: Fn(&I::Item) -> bool,
{
    type Controlled = I::Controlled;
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }

    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            FilterProducer {
                base: left,
                filter: self.filter,
            },
            FilterProducer {
                base: right,
                filter: self.filter,
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            FilterProducer {
                base: left,
                filter: self.filter,
            },
            FilterProducer {
                base: right,
                filter: self.filter,
            },
        )
    }
}

impl<'f, I, F> Producer for FilterProducer<'f, I, F>
where
    I: Producer,
    F: Fn(&I::Item) -> bool + Sync,
{
    fn preview(&self, _: usize) -> Self::Item {
        panic!("FilterProducer is not previewable")
    }
}
