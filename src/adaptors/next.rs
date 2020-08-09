use crate::prelude::*;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

pub struct Next<I> {
    pub(crate) base: I,
}

// producer

struct NextProducer<'a, P> {
    base: P,
    fake_range: Range<usize>,
    found_at: &'a AtomicUsize,
}

impl<'a, P> NextProducer<'a, P> {
    fn done(&self) -> bool {
        self.fake_range.start >= self.found_at.load(Ordering::Relaxed)
    }
    fn found(&self) {
        self.found_at
            .fetch_min(self.fake_range.start, Ordering::Relaxed);
    }
}

impl<'a, I: Iterator> Iterator for NextProducer<'a, I> {
    type Item = I::Item;
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done() {
            (0, Some(0))
        } else {
            (0, Some(1))
        }
    }
    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            None
        } else {
            let n = self.base.next();
            if n.is_some() {
                self.found()
            }
            n
        }
    }
    //TODO: can we really do fold here ?
    //i guess we would need a partial_try_fold
}

impl<'a, I: DoubleEndedIterator> DoubleEndedIterator for NextProducer<'a, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done() {
            None
        } else {
            let n = self.base.next_back();
            if n.is_some() {
                self.found()
            }
            n
        }
    }
}

impl<'a, D: Divisible> Divisible for NextProducer<'a, D> {
    type Controlled = D::Controlled;
    fn should_be_divided(&self) -> bool {
        if self.done() {
            false
        } else {
            self.base.should_be_divided()
        }
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        let (left_range, right_range) = self.fake_range.divide();
        (
            NextProducer {
                base: left,
                fake_range: left_range,
                found_at: self.found_at,
            },
            NextProducer {
                base: right,
                fake_range: right_range,
                found_at: self.found_at,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        let (left_range, right_range) = self.fake_range.divide_at(index);
        let mid_stop = Arc::new(AtomicBool::new(false));
        (
            NextProducer {
                base: left,
                fake_range: left_range,
                found_at: self.found_at,
            },
            NextProducer {
                base: right,
                fake_range: right_range,
                found_at: self.found_at,
            },
        )
    }
}

//TODO: for some reasons i don't understand items require Send
impl<'a, I: Send, P: Producer<Item = I>> Producer for NextProducer<'a, P> {
    fn sizes(&self) -> (usize, Option<usize>) {
        if self.done() {
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
        if self.done() {
            init
        } else {
            let r =
                self.base
                    .partial_fold(None, |a, b| if a.is_some() { a } else { Some(b) }, limit);
            if let Some(r) = r {
                self.found();
                fold_op(init, r)
            } else {
                init
            }
            //let r = self.base.partial_try_fold((), |_, e| Err(e), limit);
            //match r {
            //    Ok(_) => init,
            //    Err(e) => {
            //        self.found();
            //        fold_op(init, e)
            //    }
            //}
        }
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

impl<Item: Send, C: Consumer<Item>> Consumer<Item> for Next<C> {
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let found_at = AtomicUsize::new(std::usize::MAX);
        let next_producer = NextProducer {
            base: producer,
            fake_range: 0..std::usize::MAX,
            found_at: &found_at,
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
    fn with_producer<CB>(self, _callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        panic!("you cannot call with_producer on Next")
    }
}
