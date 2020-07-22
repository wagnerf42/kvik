use crate::prelude::*;

pub struct Iter<'a, T: 'a> {
    slice: &'a [T],
}

impl<'a, T: 'a + Sync> IntoParallelIterator for &'a [T] {
    type Item = &'a T;
    type Iter = Iter<'a, T>;
    fn into_par_iter(self) -> Self::Iter {
        Iter { slice: self }
    }
}

impl<'a, T: 'a + Sync> ParallelIterator for Iter<'a, T> {
    type Item = &'a T;
    type Controlled = True;
    type Enumerable = True;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.call(self.slice.iter())
    }
}

impl<'a, T: 'a + Sync> Divisible for std::slice::Iter<'a, T> {
    type Controlled = True;
    fn should_be_divided(&self) -> bool {
        self.len() >= 2
    }
    fn divide(self) -> (Self, Self) {
        let mid = self.len() / 2;
        self.divide_at(mid)
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let slice = self.as_slice();
        let index = index.min(slice.len());
        let (left, right) = slice.split_at(index);
        (left.iter(), right.iter())
    }
}

impl<'a, T: 'a + Sync> Producer for std::slice::Iter<'a, T> {
    fn sizes(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
    fn preview(&self, index: usize) -> Self::Item {
        &self.as_slice()[index]
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        let slice = self.as_slice();
        let limit = limit.min(slice.len());
        let (left, right) = slice.split_at(limit);
        *self = right.iter();
        left.iter().fold(init, fold_op)
    }
}

impl<'a, T: 'a + Sync> PreviewableParallelIterator for Iter<'a, T> {}

// mutable slices //

pub struct IterMut<'a, T: 'a> {
    slice: &'a mut [T],
}

impl<'a, T: 'a + Sync + Send> IntoParallelIterator for &'a mut [T] {
    type Item = &'a mut T;
    type Iter = IterMut<'a, T>;
    fn into_par_iter(self) -> Self::Iter {
        IterMut { slice: self }
    }
}

impl<'a, T: 'a + Send + Sync> ParallelIterator for IterMut<'a, T> {
    type Item = &'a mut T;
    type Controlled = True;
    type Enumerable = True;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.call(self.slice.iter_mut())
    }
}

impl<'a, T: 'a + Sync> Divisible for std::slice::IterMut<'a, T> {
    type Controlled = True;
    fn should_be_divided(&self) -> bool {
        self.len() >= 2
    }
    fn divide(self) -> (Self, Self) {
        let mid = self.len() / 2;
        self.divide_at(mid)
    }
    fn divide_at(self, mut index: usize) -> (Self, Self) {
        let slice = self.into_slice();
        let len = slice.len();
        if index > len {
            index = len
        }
        let (left, right) = slice.split_at_mut(index);
        (left.iter_mut(), right.iter_mut())
    }
}

impl<'a, T: 'a + Send + Sync> Producer for std::slice::IterMut<'a, T> {
    fn sizes(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
    fn preview(&self, _index: usize) -> Self::Item {
        panic!("mutable slices are not peekable");
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        replace_with::replace_with_or_abort_and_return(self, |s| {
            let slice = s.into_slice();
            let (left, right) = slice.split_at_mut(limit);
            (left.iter_mut().fold(init, fold_op), right.iter_mut())
        })
    }
}
