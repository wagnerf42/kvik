use crate::prelude::*;

pub struct Iter {
    range: std::ops::Range<u64>,
}

impl IntoParallelIterator for std::ops::Range<u64> {
    type Item = u64;
    type Iter = Iter;
    fn into_par_iter(self) -> Self::Iter {
        Iter { range: self }
    }
}

impl ParallelIterator for Iter {
    type Item = u64;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.call(self.range)
    }
}

impl Producer for std::ops::Range<u64> {
    fn should_be_divided(&self) -> bool {
        (self.end - self.start) >= 2
    }
    fn divide(self) -> (Self, Self) {
        let mid = (self.start + self.end) / 2;
        (self.start..mid, mid..self.end)
    }
}
