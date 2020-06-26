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

impl<I> Divisible for Blocked<I>
where
    I: Producer,
{
    type Controlled = I::Controlled;
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = self.base.divide();
        (Blocked::new(left), Blocked::new(right))
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.divide_at(index);
        (Blocked::new(left), Blocked::new(right))
    }
}

impl<I> Producer for Blocked<I>
where
    I: Producer,
{
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
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
        #[cfg(feature = "nightly")]
        {
            #[inline]
            fn ok<B, T>(mut f: impl FnMut(B, T) -> B) -> impl FnMut(B, T) -> Result<B, !> {
                move |acc, x| Ok(f(acc, x))
            }
            self.try_fold(init, ok(fold_op)).unwrap()
        }
        #[cfg(not(feature = "nightly"))]
        {
            #[inline]
            fn ok<B, T>(mut f: impl FnMut(B, T) -> B) -> impl FnMut(B, T) -> Result<B, ()> {
                move |acc, x| Ok(f(acc, x))
            }
            self.try_fold(init, ok(fold_op)).unwrap()
        }
    }
}
