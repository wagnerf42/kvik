//! Parallel iterators on pieces of a divisible.

use crate::prelude::*;

pub struct Wrap<D> {
    pub(crate) content: D,
}

struct WrapProducer<D> {
    content: Option<D>,
}

impl<D> ParallelIterator for Wrap<D>
where
    D: Send + Divisible,
{
    type Item = D;
    type Controlled = D::Controlled;
    type Enumerable = False;
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        let producer = WrapProducer {
            content: Some(self.content),
        };
        callback.call(producer)
    }
}

impl<D> Iterator for WrapProducer<D> {
    type Item = D;
    fn next(&mut self) -> Option<Self::Item> {
        self.content.take()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.content.is_some() {
            (1, Some(1))
        } else {
            (0, Some(0))
        }
    }
}

impl<D> DoubleEndedIterator for WrapProducer<D> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.content.take()
    }
}

impl<D> Divisible for WrapProducer<D>
where
    D: Divisible,
{
    type Controlled = D::Controlled;
    fn should_be_divided(&self) -> bool {
        self.content
            .as_ref()
            .map(|c| c.should_be_divided())
            .unwrap_or(false)
    }
    fn divide(self) -> (Self, Self) {
        if let Some(c) = self.content {
            let (left, right) = c.divide();
            (
                WrapProducer {
                    content: Some(left),
                },
                WrapProducer {
                    content: Some(right),
                },
            )
        } else {
            (
                WrapProducer { content: None },
                WrapProducer { content: None },
            )
        }
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        if let Some(c) = self.content {
            let (left, right) = c.divide_at(index);
            (
                WrapProducer {
                    content: Some(left),
                },
                WrapProducer {
                    content: Some(right),
                },
            )
        } else {
            (
                WrapProducer { content: None },
                WrapProducer { content: None },
            )
        }
    }
}

impl<D> Producer for WrapProducer<D>
where
    D: Divisible + Send,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        let size = if self.content.is_some() { 1 } else { 0 };
        (size, Some(size))
    }
    fn preview(&self, _index: usize) -> Self::Item {
        panic!("you cannot preview a WrapProducer")
    }
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        if let Some(content) = self.content.take() {
            let (left, right) = content.divide_at(limit);
            self.content = Some(right);
            fold_op(init, left)
        } else {
            init
        }
    }
}
