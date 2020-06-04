use crate::prelude::*;

#[cfg(feature = "logs")]
pub struct Log<I> {
    pub base: I,
    pub name: &'static str,
}

impl<'a, I: ParallelIterator> ParallelIterator for Log<I> {
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;
    type Item = I::Item;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<CB> {
            callback: CB,
            name: &'static str,
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
                self.callback.call(LogProducer {
                    base: producer,
                    name: self.name,
                })
            }
        }

        self.base.with_producer(Callback {
            callback,
            name: self.name,
        })
    }
}

#[cfg(feature = "logs")]
struct LogProducer<I> {
    base: I,
    name: &'static str,
}

impl<I> Iterator for LogProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        rayon_logs::subgraph(self.name, self.size_hint().0, || self.base.fold(init, f))
    }
}

impl<I> Divisible for LogProducer<I>
where
    I: Producer,
{
    type Controlled = <I as Divisible>::Controlled;

    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            LogProducer {
                base: left,
                name: self.name,
            },
            LogProducer {
                base: right,
                name: self.name,
            },
        )
    }

    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            LogProducer {
                base: left,
                name: self.name,
            },
            LogProducer {
                base: right,
                name: self.name,
            },
        )
    }

    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
}

impl<I> Producer for LogProducer<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}
