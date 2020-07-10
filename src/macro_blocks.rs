//! macro blocks producers.
//! used for all try_reduce operations.
//! this version is loose : we allow the right-most task to sequentially pass the limit.
use crate::prelude::*;

//TODO: should we add an increase factor, max and initial limit ?
pub(crate) struct MacroBlockProducer<P> {
    base: P,
    division_limit: Option<usize>,
    //TODO: rethink infinite ranges
    initial_size: usize, // we record initial size to avoid updating counts in next
}

impl<P: Producer<Controlled = True>> MacroBlockProducer<P> {
    pub(crate) fn new(base: P) -> Self {
        let (_, max_size) = base.size_hint();
        MacroBlockProducer {
            base,
            division_limit: Some(1),
            initial_size: max_size.unwrap_or(std::usize::MAX),
        }
    }
    // return how much has already been processed
    fn done(&self) -> usize {
        let (_, max_size) = self.base.size_hint();
        self.initial_size - max_size.unwrap_or(std::usize::MAX)
    }
}

impl<P: Producer<Controlled = True>> Iterator for MacroBlockProducer<P> {
    type Item = P::Item;
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
    fn next(&mut self) -> Option<Self::Item> {
        self.base.next()
    }
}

impl<P: Producer<Controlled = True>> Divisible for MacroBlockProducer<P> {
    type Controlled = False; // we can't cut wherever we want anymore
    fn should_be_divided(&self) -> bool {
        self.base.should_be_divided()
    }
    fn divide(mut self) -> (Self, Self) {
        let (left, right) = if let Some(mut limit) = self.division_limit {
            while self.done() > limit {
                // TODO: directly compute value ?
                limit *= 2
            }
            let index = (limit - self.done()) / 2;
            self.division_limit = Some(limit);
            self.base.divide_at(index)
        } else {
            self.base.divide()
        };
        (
            MacroBlockProducer {
                base: left,
                division_limit: None,
                initial_size: 0,
            },
            MacroBlockProducer {
                base: right,
                division_limit: self.division_limit,
                initial_size: self.initial_size,
            },
        )
    }
    fn divide_at(self, _index: usize) -> (Self, Self) {
        panic!("no divide_at for MacroBlockProducer")
    }
}

impl<P: Producer<Controlled = True>> Producer for MacroBlockProducer<P> {
    fn sizes(&self) -> (usize, Option<usize>) {
        unimplemented!()
    }
    fn preview(&self, index: usize) -> Self::Item {
        self.base.preview(index)
    }
}
