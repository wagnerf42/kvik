use crate::prelude::*;

pub struct Merge<A, B> {
    pub(crate) a: A,
    pub(crate) b: B,
}

impl<A, B> ParallelIterator for Merge<A, B>
where
    A::Item: Ord,
    A: ParallelIterator,
    B: ParallelIterator<Item = A::Item>,
{
    type Controlled = False;
    type Enumerable = True;
    type Item = A::Item;
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

        impl<CB, B> ProducerCallback<B::Item> for CallbackA<CB, B>
        where
            B::Item: Ord,
            B: ParallelIterator,
            CB: ProducerCallback<B::Item>,
        {
            type Output = CB::Output;

            fn call<A>(self, a_producer: A) -> Self::Output
            where
                A: Producer<Item = B::Item>,
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

        impl<CB, A> ProducerCallback<A::Item> for CallbackB<CB, A>
        where
            A: Producer,
            A::Item: Ord,
            CB: ProducerCallback<A::Item>,
        {
            type Output = CB::Output;

            fn call<B>(self, b_producer: B) -> Self::Output
            where
                B: Producer<Item = A::Item>,
            {
                self.callback.call(MergeProducer {
                    a: self.a_producer,
                    b: b_producer,
                })
            }
        }
    }
}

struct MergeProducer<A, B> {
    a: A,
    b: B,
}

impl<A, B> Iterator for MergeProducer<A, B>
where
    A: Producer,
    A::Item: Ord,
    B: Producer<Item = A::Item>,
{
    type Item = B::Item;
    fn next(&mut self) -> Option<Self::Item> {
        let a_is_empty = self.a.length() == 0;
        let b_is_empty = self.b.length() == 0;
        if !a_is_empty && !b_is_empty {
            if self.a.preview(0) <= self.b.preview(0) {
                self.a.next()
            } else {
                self.b.next()
            }
        } else if a_is_empty {
            self.b.next()
        } else {
            self.a.next()
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        debug_assert!(self.a.size_hint().0 == self.a.size_hint().1.unwrap());
        debug_assert!(self.b.size_hint().0 == self.b.size_hint().1.unwrap());
        (
            self.a.size_hint().0 + self.b.size_hint().0,
            Some(self.a.size_hint().1.unwrap() + self.b.size_hint().1.unwrap()),
        )
    }
}

fn cut_around_middle<T, P>(sorted_producer: P) -> (P, P)
where
    T: Ord,
    P: Producer<Item = T>,
{
    let iter_len = sorted_producer.length();
    if sorted_producer.preview(iter_len / 2) != sorted_producer.preview(iter_len / 2 + 1) {
        sorted_producer.divide_at(iter_len / 2 + 1)
    } else if sorted_producer.preview(iter_len / 2) != sorted_producer.preview(iter_len / 2 - 1) {
        sorted_producer.divide_at(iter_len / 2)
    } else {
        let middle_value = &sorted_producer.preview(iter_len / 2);
        //This is not good for the cache, but will make the search direction agnostic
        let last_equal_position = (0..)
            .map(|power: u32| 2_usize.pow(power))
            .take_while(|&power_of_two| power_of_two <= (iter_len - 1) / 2)
            .take_while(|power_of_two| {
                middle_value == &sorted_producer.preview(iter_len / 2 + power_of_two)
                    && middle_value == &sorted_producer.preview(iter_len / 2 - power_of_two)
            })
            .last()
            .unwrap();
        if middle_value
            != &sorted_producer.preview(
                (iter_len as u64 / 2).saturating_sub(2 * last_equal_position as u64) as usize,
            )
        {
            let mut start =
                (iter_len as u64 / 2).saturating_sub(2 * last_equal_position as u64) as usize;
            let mut end = iter_len / 2 - last_equal_position; //search only till the previous power of two
            while start < end - 1 {
                let mid = (start + end) / 2;
                if &sorted_producer.preview(mid) < middle_value {
                    //LOOP INVARIANT sorted_producer[start] < middle_value
                    start = mid;
                } else {
                    end = mid;
                }
            }
            debug_assert!(
                start == end - 1 && sorted_producer.preview(start) != sorted_producer.preview(end)
            );
            sorted_producer.divide_at(end)
        } else if middle_value
            != &sorted_producer.preview(std::cmp::min(
                iter_len - 1,
                iter_len / 2 + 2 * last_equal_position,
            ))
        {
            let mut start = iter_len / 2 + last_equal_position;
            let mut end = std::cmp::min(iter_len - 1, iter_len / 2 + 2 * last_equal_position);
            while start < end - 1 {
                let mid = (start + end) / 2;
                if &sorted_producer.preview(mid) <= middle_value {
                    //LOOP INVARIANT sorted_producer[end] > middle_value
                    start = mid;
                } else {
                    end = mid;
                }
            }
            debug_assert!(
                start == end - 1 && sorted_producer.preview(start) != sorted_producer.preview(end)
            );
            sorted_producer.divide_at(end)
        } else {
            panic!("This can not be printed");
        }
    }
}

fn search_and_cut<T, P>(sorted_iter: P, value: &T) -> (P, P)
where
    P: Producer<Item = T>,
    T: Ord,
{
    let iter_len = sorted_iter.length();
    if value < &sorted_iter.preview(0) {
        sorted_iter.divide_at(0)
    } else if value >= &sorted_iter.preview(iter_len - 1) {
        sorted_iter.divide_at(iter_len)
    } else {
        //INVARIANT l[end]>value
        let mut start = 0;
        let mut end = iter_len - 1;
        while start < end - 1 {
            let mid = (start + end) / 2;
            if &sorted_iter.preview(mid) <= value {
                start = mid;
            } else {
                end = mid;
            }
        }
        debug_assert!(start == end - 1);
        debug_assert!(sorted_iter.preview(start) != sorted_iter.preview(end));
        debug_assert!(&sorted_iter.preview(end) > value);
        sorted_iter.divide_at(end)
    }
}

impl<A, B> Divisible for MergeProducer<A, B>
where
    A: Producer,
    A::Item: Ord,
    B: Producer<Item = A::Item>,
{
    type Controlled = False;
    fn should_be_divided(&self) -> bool {
        !(self.a.length() < 6
            || self.b.length() < 6
            || self.a.preview(self.a.length() - 1) <= self.b.preview(0)
            || self.b.preview(self.b.length() - 1) <= self.a.preview(0)
            || self.a.preview(0) == self.a.preview(self.a.length() - 1)
            || self.b.preview(0) == self.b.preview(self.b.length() - 1))
    }

    fn divide(self) -> (Self, Self) {
        if self.a.length() >= self.b.length() {
            let (left_a, right_a) = cut_around_middle(self.a);
            let pivot_value = left_a.preview(left_a.length() - 1);
            let (left_b, right_b) = search_and_cut(self.b, &pivot_value);
            //debug_assert!(left_a.preview(left_a.length() - 1) <= right_a.preview(0));
            //debug_assert!(left_b.preview(left_b.length() - 1) <= right_b.preview(0));
            //debug_assert!(
            //    std::cmp::max(
            //        left_a.preview(left_a.length() - 1),
            //        left_b.preview(left_b.length() - 1)
            //    ) <= std::cmp::min(right_a.preview(0), right_b.preview(0))
            //);
            (
                MergeProducer {
                    a: left_a,
                    b: left_b,
                },
                MergeProducer {
                    a: right_a,
                    b: right_b,
                },
            )
        } else {
            let (left_b, right_b) = cut_around_middle(self.b);
            let pivot_value = left_b.preview(left_b.length() - 1);
            let (left_a, right_a) = search_and_cut(self.a, &pivot_value);
            //debug_assert!(left_a.preview(left_a.length() - 1) <= right_a.preview(0));
            //debug_assert!(left_b.preview(left_b.length() - 1) <= right_b.preview(0));
            //debug_assert!(
            //    std::cmp::max(
            //        left_a.preview(left_a.length() - 1),
            //        left_b.preview(left_b.length() - 1)
            //    ) <= std::cmp::min(right_a.preview(0), right_b.preview(0))
            //);
            (
                MergeProducer {
                    a: left_a,
                    b: left_b,
                },
                MergeProducer {
                    a: right_a,
                    b: right_b,
                },
            )
        }
    }

    fn divide_at(self, _index: usize) -> (Self, Self) {
        panic!("you cannot divide_at a merge")
    }
}

impl<A, B> Producer for MergeProducer<A, B>
where
    A: Producer,
    A::Item: Ord,
    B: Producer<Item = A::Item>,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
    fn preview(&self, _index: usize) -> Self::Item {
        panic!("you cannot preview a merge")
    }
    fn scheduler<P, R>(&self) -> Box<dyn Scheduler<P, R>>
    where
        P: Producer,
        P::Item: Send,
        R: Reducer<P::Item>,
    {
        Box::new(crate::schedulers::AdaptiveScheduler)
    }
}
