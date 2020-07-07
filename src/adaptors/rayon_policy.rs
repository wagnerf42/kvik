//! rayon scheduling policy.
use crate::prelude::*;

pub struct Rayon<I> {
    pub(crate) base: I,
    pub(crate) reset_counter: usize,
}

struct RayonProducer<I> {
    base: I,
    created_by: usize,
    reset_counter: usize,
    counter: usize,
}

impl<I: ParallelIterator> ParallelIterator for Rayon<I> {
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = I::Enumerable;

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let c = Rayon {
            base: consumer,
            reset_counter: self.reset_counter,
        };
        self.base.drive(c)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.base.with_producer(Callback {
            callback,
            reset_counter: self.reset_counter,
        });
        struct Callback<CB> {
            callback: CB,
            reset_counter: usize,
        }
        impl<T, CB> ProducerCallback<T> for Callback<CB>
        where
            CB: ProducerCallback<T>,
        {
            type Output = CB::Output;
            fn call<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = T>,
            {
                let producer = RayonProducer {
                    base,
                    reset_counter: self.reset_counter,
                    counter: self.reset_counter,
                    created_by: rayon::current_thread_index().unwrap(),
                };
                self.callback.call(producer)
                //TODO: panic once the switch to consumers is done
            }
        }
    }
}

impl<I: Iterator> Iterator for RayonProducer<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
    // TODO: should we also do try_fold ?
}

impl<I: Divisible> Divisible for RayonProducer<I> {
    type Controlled = I::Controlled;
    fn should_be_divided(&self) -> bool {
        (self.counter != 0
            || self.created_by != rayon::current_thread_index().unwrap_or(std::usize::MAX))
            && self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        let current_thread = rayon::current_thread_index().unwrap_or(std::usize::MAX);
        let new_counter = if current_thread == self.created_by {
            if self.counter == 0 {
                0
            } else {
                self.counter - 1
            }
        } else {
            self.reset_counter - 1
        };
        (
            RayonProducer {
                base: left,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
            RayonProducer {
                base: right,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        let current_thread = rayon::current_thread_index().unwrap_or(std::usize::MAX);
        let new_counter = if current_thread == self.created_by {
            if self.counter == 0 {
                0
            } else {
                self.counter - 1
            }
        } else {
            self.reset_counter - 1
        };
        (
            RayonProducer {
                base: left,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
            RayonProducer {
                base: right,
                reset_counter: self.reset_counter,
                created_by: current_thread,
                counter: new_counter,
            },
        )
    }
}

impl<I: Producer> Producer for RayonProducer<I> {
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}

impl<C: Clone> Clone for Rayon<C> {
    fn clone(&self) -> Self {
        Rayon {
            base: self.base.clone(),
            reset_counter: self.reset_counter,
        }
    }
}

impl<Item, C> Consumer<Item> for Rayon<C>
where
    C: Consumer<Item>,
{
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let rayon_producer = RayonProducer {
            base: producer,
            created_by: rayon::current_num_threads(),
            reset_counter: self.reset_counter,
            counter: self.reset_counter,
        };
        self.base.consume_producer(rayon_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
