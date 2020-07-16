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

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let fold_consumer = FoldConsumer {
            base: consumer,
            id: &self.id,
            fold: &self.fold,
        };
        self.base.drive(fold_consumer)
    }
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
        if self.base.is_some() {
            (1, Some(1))
        } else {
            (0, Some(0))
        }
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
        self.base
            .as_ref()
            .map(|b| b.sizes())
            .unwrap_or((0, Some(0)))
    }
    fn preview(&self, _: usize) -> Self::Item {
        panic!("FoldProducer is not previewable")
    }
    fn scheduler<'s, P: 's, R: 's>(&self) -> Box<dyn Scheduler<P, R> + 's>
    where
        P: Producer,
        P::Item: Send,
        R: Reducer<P::Item>,
    {
        self.base.as_ref().map(|b| b.scheduler()).unwrap()
    }
    fn partial_fold<B, FO>(&mut self, init: B, yet_another_fold_op: FO, limit: usize) -> B
    where
        B: Send,
        FO: Fn(B, Self::Item) -> B,
    {
        //TODO this is most likely SLOWER. I think the best might be to divide_at and then fold and
        //then fold the result (once).
        let old_folder = self.fold;
        let old_id = self.id;
        match self.base.as_mut() {
            Some(something) => something.partial_fold(
                init,
                |acc, old_item| yet_another_fold_op(acc, (old_folder)((old_id)(), old_item)),
                limit,
            ),
            None => init,
        }
    }
    fn completed(&self) -> bool {
        self.base
            .as_ref()
            .map(|something| something.completed())
            .unwrap_or(false)
    }
}

// consumer

struct FoldConsumer<'f, C, ID, F> {
    base: C,
    id: &'f ID,
    fold: &'f F,
}

impl<'f, C: Clone, ID, F> Clone for FoldConsumer<'f, C, ID, F> {
    fn clone(&self) -> Self {
        FoldConsumer {
            base: self.base.clone(),
            id: self.id,
            fold: self.fold,
        }
    }
}

impl<'f, T, Item, C, ID, F> Consumer<Item> for FoldConsumer<'f, C, ID, F>
where
    C: Consumer<T>,
    F: Fn(T, Item) -> T + Sync,
    ID: Fn() -> T + Sync,
{
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let fold_producer = FoldProducer {
            base: Some(producer),
            id: self.id,
            fold: self.fold,
        };
        self.base.consume_producer(fold_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
