//! Sequential iterator you can eat by blocks if you try_fold.
use crate::adaptive::AdaptiveProducer;
use crate::prelude::*;

/// Sequential iterator you can eat in several bites.
/// It's like slow food but for programmers.
pub struct Blocked<I> {
    base: I,
    limit: usize,
}

impl<I> Iterator for Blocked<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.limit == 0 {
            None
        } else {
            self.limit -= 1;
            self.base.next()
        }
    }
    fn fold<B, F>(self, init: B, fold_op: F) -> B
    where
        F: FnMut(B, I::Item) -> B,
    {
        self.base.fold(init, fold_op)
    }
}

impl<I> Producer for Blocked<I>
where
    I: Producer,
{
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (Blocked::new(left), Blocked::new(right))
    }
}

impl<I> Blocked<I>
where
    I: Iterator,
{
    /// Return an iterator you can eat by blocks.
    pub fn new(iterator: I) -> Self {
        Blocked {
            base: iterator,
            limit: 0,
        }
    }
}

impl<I: Producer> AdaptiveProducer for Blocked<I> {
    fn completed(&self) -> bool {
        self.base.size_hint().1 == Some(0)
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        F: Fn(B, I::Item) -> B,
    {
        self.limit = limit;
        self.try_fold(init, |old: B, e: I::Item| -> Result<B, ()> {
            Ok(fold_op(old, e))
        })
        .unwrap()
    }
}
