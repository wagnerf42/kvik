//! Sequential iterator you can eat by blocks if you try_fold.
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
    pub fn inner_size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
    /// Set current's block size.
    pub fn set_block_size(&mut self, block_size: usize) {
        self.limit = block_size
    }
    /// Fold one block.
    /// You can still use the iterator afterwards.
    ///
    /// # Example
    /// ```
    /// use rayon_try_fold::Blocked;
    /// let mut i = Blocked::new(0..10);
    /// i.set_block_size(4);
    /// assert_eq!(0+1+2+3, i.partial_fold(0, |a, b| a+b));
    /// i.set_block_size(3);
    /// assert_eq!(4+5+6, i.partial_fold(0, |a, b| a+b));
    /// i.set_block_size(8);
    /// assert_eq!(7+8+9, i.partial_fold(0, |a, b| a+b));
    /// ```
    pub fn partial_fold<B, F>(&mut self, init: B, fold_op: F) -> B
    where
        F: Fn(B, I::Item) -> B,
    {
        self.try_fold(init, |old: B, e: I::Item| -> Result<B, ()> {
            Ok(fold_op(old, e))
        })
        .unwrap()
    }
}
