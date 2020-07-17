use crate::prelude::*;

// Note: all type constraints on A and B are done in the `zip` method.
pub struct Zip<A, B> {
    pub(crate) a: A,
    pub(crate) b: B,
}

impl<A, B> ParallelIterator for Zip<A, B>
where
    A: ParallelIterator,
    B: ParallelIterator,
{
    type Controlled = A::Controlled;
    type Enumerable = True;
    type Item = (A::Item, B::Item);
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.a.with_producer(CallbackA {
            callback,
            b: self.b,
        });

        struct CallbackA<CB, B> {
            callback: CB,
            b: B,
        }

        impl<CB, ITEM, B> ProducerCallback<ITEM> for CallbackA<CB, B>
        where
            B: ParallelIterator,
            CB: ProducerCallback<(ITEM, B::Item)>,
        {
            type Output = CB::Output;

            fn call<A>(self, a_producer: A) -> Self::Output
            where
                A: Producer<Item = ITEM>,
            {
                self.b.with_producer(CallbackB {
                    a_producer,
                    callback: self.callback,
                })
            }
        }

        struct CallbackB<CB, A> {
            a_producer: A,
            callback: CB,
        }

        impl<CB, A, ITEM> ProducerCallback<ITEM> for CallbackB<CB, A>
        where
            A: Producer,
            CB: ProducerCallback<(A::Item, ITEM)>,
        {
            type Output = CB::Output;

            fn call<B>(self, b_producer: B) -> Self::Output
            where
                B: Producer<Item = ITEM>,
            {
                self.callback.call(ZipProducer {
                    a: self.a_producer,
                    b: b_producer,
                })
            }
        }
    }
}

struct ZipProducer<A, B> {
    a: A,
    b: B,
}

impl<A, B> Iterator for ZipProducer<A, B>
where
    A: Iterator,
    B: Iterator,
{
    type Item = (A::Item, B::Item);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next_a) = self.a.next() {
            self.b.next().map(|next_b| (next_a, next_b))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower_bound, a_upper_bound) = (
            self.a.size_hint().0,
            self.a.size_hint().1.unwrap_or(std::usize::MAX),
        );
        let (b_lower_bound, b_upper_bound) = (
            self.b.size_hint().0,
            self.b.size_hint().1.unwrap_or(std::usize::MAX),
        );
        (
            std::cmp::min(a_lower_bound, b_lower_bound),
            Some(std::cmp::min(a_upper_bound, b_upper_bound)),
        )
    }
}

impl<A, B> Divisible for ZipProducer<A, B>
where
    A: Producer,
    B: Divisible,
{
    type Controlled = A::Controlled;
    fn should_be_divided(&self) -> bool {
        self.a.should_be_divided() && self.b.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left_a, right_a) = self.a.divide();
        let (left_b, right_b) = self.b.divide_at(left_a.length());
        (
            ZipProducer {
                a: left_a,
                b: left_b,
            },
            ZipProducer {
                a: right_a,
                b: right_b,
            },
        )
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left_a, right_a) = self.a.divide_at(index);
        let (left_b, right_b) = self.b.divide_at(index);
        (
            ZipProducer {
                a: left_a,
                b: left_b,
            },
            ZipProducer {
                a: right_a,
                b: right_b,
            },
        )
    }
}

impl<A, B> Producer for ZipProducer<A, B>
where
    A: Producer,
    B: Producer,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        let (min_a, max_a) = self.a.sizes();
        let (min_b, max_b) = self.b.sizes();
        let min = min_a.max(min_b);
        let max = if let Some(ma) = max_a {
            if let Some(mb) = max_b {
                Some(ma.min(mb))
            } else {
                Some(ma)
            }
        } else {
            max_b
        };
        (min, max)
    }
    fn partial_fold<BI, F>(&mut self, mut init: BI, fold_op: F, mut limit: usize) -> BI
    where
        BI: Send,
        F: Fn(BI, Self::Item) -> BI,
    {
        while limit > 0 {
            if let Some(sorted_elem) = self.next() {
                init = fold_op(init, sorted_elem);
                limit -= 1;
            } else {
                break;
            }
        }
        init
    }
    fn preview(&self, index: usize) -> Self::Item {
        (self.a.preview(index), self.b.preview(index))
    }
}
