use crate::prelude::*;

struct EvenLevelsProducer<I> {
    base: I,
    counter: bool,
}

impl<I> Iterator for EvenLevelsProducer<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
}

impl<I> Divisible for EvenLevelsProducer<I>
where
    I: Divisible,
{
    type Controlled = <I as Divisible>::Controlled;
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            EvenLevelsProducer {
                base: left,
                counter: !self.counter,
            },
            EvenLevelsProducer {
                base: right,
                counter: !self.counter,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            EvenLevelsProducer {
                base: left,
                counter: !self.counter,
            },
            EvenLevelsProducer {
                base: right,
                counter: !self.counter,
            },
        )
    }
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided() || !self.counter
    }
}

impl<I> Producer for EvenLevelsProducer<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}

pub struct EvenLevels<I> {
    pub base: I,
}

impl<I: ParallelIterator> ParallelIterator for EvenLevels<I> {
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let c = EvenLevels { base: consumer };
        self.base.drive(c)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        struct Callback<CB> {
            callback: CB,
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
                self.callback.call(EvenLevelsProducer {
                    base: producer,
                    counter: true,
                })
            }
        }
        self.base.with_producer(Callback { callback })
    }
}

impl<C: Clone> Clone for EvenLevels<C> {
    fn clone(&self) -> Self {
        EvenLevels {
            base: self.base.clone(),
        }
    }
}

impl<Item, C> Consumer<Item> for EvenLevels<C>
where
    C: Consumer<Item>,
{
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let even_levels_producer = EvenLevelsProducer {
            base: producer,
            counter: true,
        };
        self.base.consume_producer(even_levels_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
