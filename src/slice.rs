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
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.call(IterProducer {
            slice: self.slice,
            index: 0,
        })
    }
}

struct IterProducer<'a, T: 'a> {
    slice: &'a [T],
    index: usize,
}

impl<'a, T: 'a> Iterator for IterProducer<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.len() <= self.index {
            None
        } else {
            let index = self.index;
            self.index += 1;
            //TODO: how badly to we really need that ?
            Some(unsafe { self.slice.get_unchecked(index) })
        }
    }
}

impl<'a, T: 'a + Sync> Producer for IterProducer<'a, T> {
    fn should_be_divided(&self) -> bool {
        self.slice.len() - self.index >= 2
    }
    fn divide(self) -> (Self, Self) {
        let mid = (self.slice.len() - self.index) / 2;
        let (left, right) = self.slice[self.index..].split_at(mid);
        (
            IterProducer {
                slice: left,
                index: 0,
            },
            IterProducer {
                slice: right,
                index: 0,
            },
        )
    }
}
