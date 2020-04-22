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
    type Power = <I as Divisible>::Power;
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

pub struct EvenLevels<I> {
    pub base: I,
}

impl<I: ParallelIterator> ParallelIterator for EvenLevels<I> {
    type Item = I::Item;
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
            fn call<P>(&self, producer: P) -> Self::Output
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
