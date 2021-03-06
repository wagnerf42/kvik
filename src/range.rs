use crate::prelude::*;
use crate::try_fold::try_fold;
use crate::Try;

pub struct Iter<T> {
    range: std::ops::Range<T>,
}

macro_rules! implement_traits {
    ($x: ty) => {
        impl IntoParallelIterator for std::ops::Range<$x> {
            type Item = $x;
            type Iter = Iter<$x>;
            fn into_par_iter(self) -> Self::Iter {
                Iter { range: self }
            }
        }

        impl ParallelIterator for Iter<$x> {
            type Item = $x;
            type Controlled = True;
            type Enumerable = True;
            fn with_producer<CB>(self, callback: CB) -> CB::Output
            where
                CB: ProducerCallback<Self::Item>,
            {
                callback.call(self.range)
            }
        }

        impl Divisible for std::ops::Range<$x> {
            type Controlled = True;
            fn should_be_divided(&self) -> bool {
                (self.end - self.start) >= 2
            }
            fn divide(self) -> (Self, Self) {
                // We need to divide with the biggest half on the left, so instead of just doing
                // (end - start) / 2, which in cases of an odd length will put the biggest
                // half on the right, we need to make sure we take the ceiling of the value:
                // this is done using (end - start) - ((end - start) / 2).
                let index = (self.end - self.start) - ((self.end - self.start) / 2);
                self.divide_at(index as usize)
            }
            fn divide_at(self, index: usize) -> (Self, Self) {
                let mut mid = self.start + (index as $x);
                if mid > self.end {
                    mid = self.end
                }
                (self.start..mid, mid..self.end)
            }
        }

        impl Producer for std::ops::Range<$x> {
            fn sizes(&self) -> (usize, Option<usize>) {
                self.size_hint()
            }
            fn preview(&self, index: usize) -> Self::Item {
                self.start + index as $x
            }
            fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
            where
                B: Send,
                F: Fn(B, Self::Item) -> B,
            {
                let next_limit = (self.start + limit as $x).min(self.end);
                let output = (self.start..next_limit).fold(init, fold_op);
                self.start = next_limit;
                output
            }
            fn partial_try_fold<B, F, R>(&mut self, init: B, f: F, limit: usize) -> R
            where
                F: FnMut(B, Self::Item) -> R,
                R: Try<Ok = B>,
            {
                let next_limit = (self.start + limit as $x).min(self.end);
                let output = try_fold(&mut (self.start..next_limit), init, f);
                self.start = next_limit;
                output
            }
        }

        impl PreviewableParallelIterator for Iter<$x> {}
    };
}

implement_traits!(i16);
implement_traits!(u16);
implement_traits!(i32);
implement_traits!(isize);
implement_traits!(u8);
implement_traits!(usize);
implement_traits!(i8);
implement_traits!(u32);
implement_traits!(u64);
