use crate::prelude::*;

pub struct Fold<I, ID, F> {
    pub(crate) base: I,
    pub(crate) id: ID,
    pub(crate) fold: F,
}

impl<T, I, ID, F> ParallelIterator for Fold<I, ID, F>
where
    I: ParallelIterator,
    F: Fn(T, I::Item) -> T + Sync + Send,
    ID: Fn() -> T + Sync + Send,
    T: Send,
{
    type Item = T;
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.base.with_producer(Callback {
            callback,
            id: self.id,
            fold: self.fold,
        });

        struct Callback<CB, ID, F> {
            callback: CB,
            id: ID,
            fold: F,
        }

        impl<CB, T, R, ID, F> ProducerCallback<T> for Callback<CB, ID, F>
        where
            CB: ProducerCallback<R>,
            F: Fn(R, T) -> R + Sync + Send,
            ID: Fn() -> R + Sync + Send,
            T: Send,
        {
            type Output = CB::Output;

            fn call<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = T>,
            {
                let producer = FoldProducer {
                    base: Some(base),
                    id: &self.id,
                    fold: &self.fold,
                };
                self.callback.call(producer)
            }
        }
    }
}

struct FoldProducer<'f, I, ID, F> {
    base: Option<I>,
    id: &'f ID,
    fold: &'f F,
}

impl<'f, T, I, ID, F> Iterator for FoldProducer<'f, I, ID, F>
where
    I: Iterator,
    F: Fn(T, I::Item) -> T,
    ID: Fn() -> T,
{
    type Item = T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base
            .as_ref()
            .map(|b| b.size_hint())
            .unwrap_or((0, Some(0)))
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.base
            .take()
            .and_then(|b| Some(b.fold((self.id)(), self.fold)))
    }
}

impl<'f, T, I, ID, F> Divisible for FoldProducer<'f, I, ID, F>
where
    I: Producer,
    F: Fn(T, I::Item) -> T,
    ID: Fn() -> T,
{
    type Controlled = I::Controlled;

    fn should_be_divided(&self) -> bool {
        self.base
            .as_ref()
            .map(|b| b.should_be_divided())
            .unwrap_or(false)
    }

    fn divide(self) -> (Self, Self) {
        let (left, right) = self
            .base
            .map(|b| {
                let (l, r) = b.divide();
                (Some(l), Some(r))
            })
            .unwrap_or((None, None));
        (
            FoldProducer {
                base: left,
                id: self.id,
                fold: self.fold,
            },
            FoldProducer {
                base: right,
                id: self.id,
                fold: self.fold,
            },
        )
    }

    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self
            .base
            .map(|b| {
                let (l, r) = b.divide_at(index);
                (Some(l), Some(r))
            })
            .unwrap_or((None, None));
        (
            FoldProducer {
                base: left,
                id: self.id,
                fold: self.fold,
            },
            FoldProducer {
                base: right,
                id: self.id,
                fold: self.fold,
            },
        )
    }
}

impl<'f, T, I, ID, F> Producer for FoldProducer<'f, I, ID, F>
where
    I: Producer,
    F: Fn(T, I::Item) -> T + Sync,
    ID: Fn() -> T + Sync,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        unimplemented!("i need to check this file")
    }
    fn preview(&self, _: usize) -> Self::Item {
        panic!("FoldProducer is not previewable")
    }
}
