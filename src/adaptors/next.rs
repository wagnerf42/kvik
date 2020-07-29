use crate::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Next<I> {
    pub(crate) base: I,
}

// producer

struct NextProducer<P> {
    base: P,
    stop: Arc<AtomicBool>,      // should i stop ?
    next_stop: Arc<AtomicBool>, // should the next guy stop ?
}

impl<I: Iterator> Iterator for NextProducer<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.stop.load(Ordering::Relaxed) {
            self.next_stop.store(true, Ordering::Relaxed);
            None
        } else {
            let n = self.base.next();
            if n.is_some() {
                self.next_stop.store(true, Ordering::Relaxed)
            }
            n
        }
    }
    //TODO: can we really do fold here ?
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for NextProducer<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.stop.load(Ordering::Relaxed) {
            self.next_stop.store(true, Ordering::Relaxed);
            None
        } else {
            let n = self.base.next_back();
            if n.is_some() {
                self.next_stop.store(true, Ordering::Relaxed)
            }
            n
        }
    }
}

impl<D: Divisible> Divisible for NextProducer<D> {
    type Controlled = D::Controlled;
    fn should_be_divided(&self) -> bool {
        if self.stop.load(Ordering::Relaxed) {
            self.next_stop.store(true, Ordering::Relaxed);
            false
        } else {
            self.base.should_be_divided()
        }
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        let mid_stop = Arc::new(AtomicBool::new(false));
        (
            NextProducer {
                base: left,
                stop: self.stop,
                next_stop: mid_stop.clone(),
            },
            NextProducer {
                base: right,
                stop: mid_stop,
                next_stop: self.next_stop,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        let mid_stop = Arc::new(AtomicBool::new(false));
        (
            NextProducer {
                base: left,
                stop: self.stop,
                next_stop: mid_stop.clone(),
            },
            NextProducer {
                base: right,
                stop: mid_stop,
                next_stop: self.next_stop,
            },
        )
    }
}

impl<P: Producer> Producer for NextProducer<P> {
    fn sizes(&self) -> (usize, Option<usize>) {
        if self.stop.load(Ordering::Relaxed) {
            self.next_stop.store(true, Ordering::Relaxed)
            (0, Some(0))
        } else {
            self.base.sizes()
        }
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        panic!("no partial fold on Next for now")
    }
}

// consumer

impl<C: Clone> Clone for Next<C> {
    fn clone(&self) -> Self {
        Next {
            base: self.base.clone(),
        }
    }
}

impl<Item, C: Consumer<Item>> Consumer<Item> for Next<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let next_producer = NextProducer {
            base: producer,
            stop: Arc::new(AtomicBool::new(false)),
            next_stop: Arc::new(AtomicBool::new(false)),
        };
        self.base.consume_producer(next_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

// iterator

impl<I: ParallelIterator> ParallelIterator for Next<I> {
    type Item = I::Item;
    type Controlled = I::Controlled;
    type Enumerable = False; // TODO: True ?
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let next_consumer = Next { base: consumer };
        self.base.drive(next_consumer)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        panic!("you cannot call with_producer on Next")
    }
}
