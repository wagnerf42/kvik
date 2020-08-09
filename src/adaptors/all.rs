use crate::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct All<'b, I, P> {
    pub(crate) base: I,
    pub(crate) predicate: &'b P,
    pub(crate) stop: &'b AtomicBool,
}

impl<'b, I, P> All<'b, I, P> {
    fn done(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
    fn mark_done(&self) {
        self.stop.store(true, Ordering::Relaxed)
    }
}

impl<'b, I: Iterator, P: Fn(I::Item) -> bool> Iterator for All<'b, I, P> {
    type Item = Result<(), ()>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            None
        } else {
            self.base.next().map(|n| {
                if (self.predicate)(n) {
                    Ok(())
                } else {
                    self.mark_done();
                    Err(())
                }
            })
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done() {
            (0, Some(0))
        } else {
            (0, self.base.size_hint().1)
        }
    }
}

impl<'b, I: DoubleEndedIterator, P: Fn(I::Item) -> bool> DoubleEndedIterator for All<'b, I, P> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done() {
            None
        } else {
            self.base.next_back().map(|n| {
                if (self.predicate)(n) {
                    Ok(())
                } else {
                    self.mark_done();
                    Err(())
                }
            })
        }
    }
}

impl<'b, I: Producer, P: Fn(I::Item) -> bool> Divisible for All<'b, I, P> {
    type Controlled = I::Controlled;
    fn should_be_divided(&self) -> bool {
        !self.done() && self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (
            All {
                base: left,
                predicate: self.predicate,
                stop: self.stop,
            },
            All {
                base: right,
                predicate: self.predicate,
                stop: self.stop,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (
            All {
                base: left,
                predicate: self.predicate,
                stop: self.stop,
            },
            All {
                base: right,
                predicate: self.predicate,
                stop: self.stop,
            },
        )
    }
}

impl<'b, I: Producer, P: Fn(I::Item) -> bool + Send + Sync> Producer for All<'b, I, P> {
    fn sizes(&self) -> (usize, Option<usize>) {
        if self.done() {
            (0, Some(0))
        } else {
            self.base.sizes()
        }
    }
    fn scheduler<'s, Q: 's, R: 's>(&self) -> Box<dyn Scheduler<Q, R> + 's>
    where
        Q: Producer,
        Q::Item: Send,
        R: Reducer<Q::Item>,
    {
        self.base.scheduler()
    }

    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        if self.done() {
            init
        } else {
            let predicate = self.predicate;
            #[inline]
            fn check<T>(f: impl Fn(T) -> bool) -> impl Fn(Option<()>, T) -> Option<()> {
                move |acc, x| {
                    if acc.is_none() {
                        acc
                    } else {
                        if f(x) {
                            Some(())
                        } else {
                            None
                        }
                    }
                }
            }
            let r = self.base.partial_fold(Some(()), check(predicate), limit);
            match r {
                Some(()) => init,
                None => {
                    self.mark_done();
                    fold_op(init, Err(()))
                }
            }
        }
    }
    fn preview(&self, index: usize) -> Self::Item {
        panic!("no preview for All")
    }
}

impl<'b, I: Clone, P> Clone for All<'b, I, P> {
    fn clone(&self) -> Self {
        All {
            base: self.base.clone(),
            predicate: self.predicate,
            stop: self.stop,
        }
    }
}

impl<'b, Item, I: Consumer<Result<(), ()>>, P: Fn(Item) -> bool + Send + Sync> Consumer<Item>
    for All<'b, I, P>
{
    type Result = I::Result;
    type Reducer = I::Reducer;
    fn consume_producer<Q>(self, producer: Q) -> Self::Result
    where
        Q: Producer<Item = Item>,
    {
        let all_producer = All {
            base: producer,
            predicate: self.predicate,
            stop: self.stop,
        };
        self.base.consume_producer(all_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
