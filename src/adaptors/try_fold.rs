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
    type Item = R;
    fn next(&mut self) -> Option<Self::Item> {
        self.base.take().map(|mut b| {
            let res = try_fold(&mut b, self.init.take().unwrap(), self.fold_op);
            let real_res = res.into_result();
            match real_res {
                Ok(o) => R::from_ok(o),
                Err(e) => {
                    self.mark_done();
                    R::from_error(e)
                }
            }
        })
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
        if let Some(res) = self.next() {
            f(init, res)
        } else {
            init
        }
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
                    return fold_op(init, R::from_error(e));
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
