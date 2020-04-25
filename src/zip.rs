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
        let (left_a, right_a) = (
            self.a.size_hint().0,
            self.a
                .size_hint()
                .1
                .expect("Left side of the zip is not enumerable"),
        );
        let (left_b, right_b) = (
            self.b.size_hint().0,
            self.b
                .size_hint()
                .1
                .expect("Right side of the zip is not enumerable"),
        );
        assert_eq!(left_a, right_a);
        assert_eq!(left_b, right_b);
        assert_eq!(left_a, left_b);
        (left_a, Some(right_a))
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
    fn preview(&self, index: usize) -> Self::Item {
        (self.a.preview(index), self.b.preview(index))
    }
}
