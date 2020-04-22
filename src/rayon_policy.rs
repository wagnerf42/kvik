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
            fn call<P>(&self, base: P) -> CB::Output
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
    type Power = I::Power;
    fn should_be_divided(&self) -> bool {
        self.counter != 0 && self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        let current_thread = rayon::current_thread_index().unwrap();
        let new_counter = if current_thread == self.created_by {
            if self.counter == 0 {
                0
            } else {
                self.counter - 1
            }
        } else {
            self.reset_counter
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
        let current_thread = rayon::current_thread_index().unwrap();
        let new_counter = if current_thread == self.created_by {
            if self.counter == 0 {
                0
            } else {
                self.counter - 1
            }
        } else {
            self.reset_counter
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
