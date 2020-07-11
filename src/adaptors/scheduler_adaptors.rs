use crate::prelude::*;
use crate::schedulers::{AdaptiveScheduler, DepJoinScheduler, SequentialScheduler};

macro_rules! scheduler_adaptor {
    ($type: ident, $function: ident) => {
        pub struct $type<I> {
            pub(crate) base: I,
        }

        // producer
        impl<I: Iterator> Iterator for $type<I> {
            type Item = I::Item;
            fn next(&mut self) -> Option<Self::Item> {
                self.base.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.base.size_hint()
            }
        }

        impl<P: Producer> Divisible for $type<P> {
            type Controlled = P::Controlled;
            fn should_be_divided(&self) -> bool {
                false
            }
            fn divide(self) -> (Self, Self) {
                let (left, right) = self.base.divide();
                ($type { base: left }, $type { base: right })
            }
            fn divide_at(self, index: usize) -> (Self, Self) {
                let (left, right) = self.base.divide_at(index);
                ($type { base: left }, $type { base: right })
            }
        }

        impl<Q: Producer> Producer for $type<Q> {
            fn sizes(&self) -> (usize, Option<usize>) {
                self.base.sizes()
            }
            fn preview(&self, index: usize) -> Self::Item {
                self.base.preview(index)
            }
            fn scheduler<P, R>(&self) -> Box<dyn Scheduler<P, R>>
            where
                P: Producer,
                P::Item: Send,
                R: Reducer<P::Item>,
            {
                Box::new($function)
            }
        }

        // consumer
        impl<C: Clone> Clone for $type<C> {
            fn clone(&self) -> Self {
                $type {
                    base: self.base.clone(),
                }
            }
        }

        impl<Item, C: Consumer<Item>> Consumer<Item> for $type<C> {
            type Result = C::Result;
            type Reducer = C::Reducer;
            fn consume_producer<P>(self, producer: P) -> Self::Result
            where
                P: Producer<Item = Item>,
            {
                let producer = $type { base: producer };
                self.base.consume_producer(producer)
            }
            fn to_reducer(self) -> Self::Reducer {
                self.base.to_reducer()
            }
        }

        // iterator

        impl<I> ParallelIterator for $type<I>
        where
            I: ParallelIterator,
        {
            type Item = I::Item;
            type Controlled = False;
            type Enumerable = False;
            fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
                let consumer = $type { base: consumer };
                self.base.drive(consumer)
            }
            fn with_producer<CB>(self, _callback: CB) -> CB::Output
            where
                CB: ProducerCallback<Self::Item>,
            {
                panic!("scheduling policies must be called as a consumer")
            }
        }
    };
}

scheduler_adaptor!(DepJoin, DepJoinScheduler);
scheduler_adaptor!(Sequential, SequentialScheduler);
scheduler_adaptor!(Adaptive, AdaptiveScheduler);
