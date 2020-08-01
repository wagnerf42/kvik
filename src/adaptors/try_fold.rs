//! this file is wrong and needs a conceptual
//! rethinking.
//! rayon has no producers for this type and for good reasons. :-(
use crate::prelude::*;
use crate::try_fold::try_fold;
use crate::Try;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct TryFold<I, U, ID, F> {
    pub(crate) base: I,
    pub(crate) identity: ID,
    pub(crate) fold_op: F,
    pub(crate) marker: PhantomData<U>,
}

// producer

struct TryFoldProducer<'b, I, U, ID, F> {
    base: Option<I>,
    init: Option<U>,
    identity: &'b ID,
    fold_op: &'b F,
    stop: &'b AtomicBool,
}

impl<'b, I, U, ID, F> TryFoldProducer<'b, I, U, ID, F> {
    fn done(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
    fn mark_done(&self) {
        self.stop.store(true, Ordering::Relaxed)
    }
}

impl<'b, I, U, ID, F, R> Iterator for TryFoldProducer<'b, I, U, ID, F>
where
    I: Iterator,
    F: Fn(U, I::Item) -> R + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    R: Try<Ok = U> + Send,
{
    type Item = U;
    fn next(&mut self) -> Option<Self::Item> {
        panic!("no next on TryFoldProducers")
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let iterations = if self.base.is_some() && !self.done() {
            1
        } else {
            0
        };
        (iterations, Some(iterations))
    }
    fn fold<B, G>(mut self, init: B, mut f: G) -> B
    where
        G: FnMut(B, Self::Item) -> B,
    {
        panic!("no fold for TryFoldProducer")
    }
}

impl<'b, I, U, ID, F, R> DoubleEndedIterator for TryFoldProducer<'b, I, U, ID, F>
where
    I: DoubleEndedIterator,
    F: Fn(U, I::Item) -> R + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    R: Try<Ok = U> + Send,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        unimplemented!("we need a try_rfold first")
    }
    //TODO: Iterator trait is nice because we don't need to reimplement everything but having the
    //default methods sucks
}

impl<'b, I, U, ID, F, R> Divisible for TryFoldProducer<'b, I, U, ID, F>
where
    I: Producer,
    F: Fn(U, I::Item) -> R + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    R: Try<Ok = U> + Send,
{
    type Controlled = I::Controlled;
    fn should_be_divided(&self) -> bool {
        !self.done()
            && self
                .base
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
        let right_init = if self.init.is_some() {
            Some((self.identity)())
        } else {
            None
        };
        (
            TryFoldProducer {
                base: left,
                init: self.init,
                identity: self.identity,
                fold_op: self.fold_op,
                stop: self.stop,
            },
            TryFoldProducer {
                base: right,
                init: right_init,
                identity: self.identity,
                fold_op: self.fold_op,
                stop: self.stop,
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
        let right_init = if self.init.is_some() {
            Some((self.identity)())
        } else {
            None
        };
        (
            TryFoldProducer {
                base: left,
                init: self.init,
                identity: self.identity,
                fold_op: self.fold_op,
                stop: self.stop,
            },
            TryFoldProducer {
                base: right,
                init: right_init,
                identity: self.identity,
                fold_op: self.fold_op,
                stop: self.stop,
            },
        )
    }
}

impl<'b, I, U, ID, F, R> Producer for TryFoldProducer<'b, I, U, ID, F>
where
    U: Send,
    I: Producer,
    F: Fn(U, I::Item) -> R + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    R: Try<Ok = U> + Send,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        if self.done() {
            (0, Some(0))
        } else {
            self.base
                .as_ref()
                .map(|b| b.sizes())
                .unwrap_or((0, Some(0)))
        }
    }
    fn partial_fold<B, G>(&mut self, init: B, fold_op: G, limit: usize) -> B
    where
        B: Send,
        G: Fn(B, Self::Item) -> B,
    {
        // ok, so what do we want to do ?
        // well, we want to partial try fold the base
        // if we meet an error then we stop of course and fold_op
        // else we put store back the new init
        let local_init = self.init.take();
        let local_fold_op = self.fold_op;
        let maybe_res = self
            .base
            .as_mut()
            .map(|base| base.partial_try_fold(local_init.unwrap(), local_fold_op, limit));
        if let Some(res) = maybe_res {
            match res.into_result() {
                Ok(o) => {
                    self.init = Some(o);
                    init
                }
                Err(e) => {
                    self.mark_done();
                    panic!("no way to return the error")
                }
            }
        } else {
            init
        }
    }
    fn preview(&self, index: usize) -> Self::Item {
        panic!("you cannot preview TryFold")
    }
}

// consumer

struct TryFoldConsumer<'b, C, ID, F> {
    base: C,
    identity: &'b ID,
    fold_op: &'b F,
    stop: &'b AtomicBool,
}

impl<'b, C: Clone, ID, F> Clone for TryFoldConsumer<'b, C, ID, F> {
    fn clone(&self) -> Self {
        TryFoldConsumer {
            base: self.base.clone(),
            identity: self.identity,
            fold_op: self.fold_op,
            stop: self.stop,
        }
    }
}

impl<'b, Item, C, ID, F, R, U> Consumer<Item> for TryFoldConsumer<'b, C, ID, F>
where
    Item: Send,
    U: Send,
    C: Consumer<U>,
    F: Fn(U, Item) -> R + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    R: Try<Ok = U> + Send,
{
    type Result = C::Result;
    type Reducer = C::Reducer;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let try_fold_producer = TryFoldProducer {
            base: Some(producer),
            init: Some((self.identity)()),
            identity: self.identity,
            fold_op: self.fold_op,
            stop: self.stop,
        };
        self.base.consume_producer(try_fold_producer)
    }
    fn to_reducer(self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

// iterator

impl<I, U, ID, F, R> ParallelIterator for TryFold<I, U, ID, F>
where
    U: Send,
    I: ParallelIterator,
    F: Fn(U, I::Item) -> R + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    R: Try<Ok = U> + Send,
{
    type Controlled = True;
    type Enumerable = False;
    type Item = U;
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let stop = AtomicBool::new(false);
        let try_fold_consumer = TryFoldConsumer {
            base: consumer,
            identity: &self.identity,
            fold_op: &self.fold_op,
            stop: &stop,
        };
        self.base.drive(try_fold_consumer)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        unimplemented!()
    }
}
