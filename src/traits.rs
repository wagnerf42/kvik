use crate::adaptive::Adaptive;
use crate::even_levels::EvenLevels;
use crate::join_policy::JoinPolicy;
use crate::map::Map;
use crate::merge::Merge;
use crate::rayon_policy::Rayon;
use crate::sequential::Sequential;
use crate::wrap::Wrap;
use crate::zip::Zip;

// Iterators have different properties
// which allow for specialisation of some algorithms.
//
// We need to know :
// - can you control around where you cut ?
// - do you exactly know the number of elements yielded ?
// We use marker types to associate each information to each iterator.
pub struct True;
pub struct False;

pub trait Divisible: Sized {
    type Controlled;
    fn should_be_divided(&self) -> bool;
    fn divide(self) -> (Self, Self);
    fn divide_at(self, index: usize) -> (Self, Self);
    /// Cut divisible recursively into smaller pieces forming a ParallelIterator.
    /// # Example:
    /// ```
    /// use rayon_try_fold::prelude::*;
    /// let r = (0u64..10);
    /// //TODO : write sum and all parallel ranges (to get .len)
    /// let length = r.wrap_iter().map(|p| p.end-p.start).reduce(||0, |a,b|a+b);
    /// assert_eq!(length, 10)
    /// ```
    fn wrap_iter(self) -> Wrap<Self> {
        Wrap { content: self }
    }
}

impl<A, B> Divisible for (A, B)
where
    A: Divisible,
    B: Divisible,
{
    type Controlled = A::Controlled; // TODO: take min
    fn should_be_divided(&self) -> bool {
        self.0.should_be_divided() || self.1.should_be_divided()
    }
    fn divide(self) -> (Self, Self) {
        let (left_a, right_a) = self.0.divide();
        let (left_b, right_b) = self.1.divide();
        ((left_a, left_b), (right_a, right_b))
    }
    fn divide_at(self, index: usize) -> (Self, Self) {
        let (left_a, right_a) = self.0.divide_at(index);
        let (left_b, right_b) = self.1.divide_at(index);
        ((left_a, left_b), (right_a, right_b))
    }
}

pub trait ProducerCallback<T> {
    type Output;
    fn call<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>;
}

//TODO: there is a way to not have any method
//here and use .len from ExactSizeIterator
//but it require changing with_producer to propagate
//type constraints. would it be a better option ?
pub trait Producer: Send + Iterator + Divisible {
    fn sizes(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
    //TODO: this should only be called on left hand sides of infinite iterators
    fn length(&self) -> usize {
        let (min, max) = self.sizes();
        if let Some(m) = max {
            assert_eq!(m, min);
            min
        } else {
            panic!("we are not enumerable")
        }
    }
    fn preview(&self, index: usize) -> Self::Item;
}

struct ReduceCallback<'f, OP, ID> {
    op: &'f OP,
    identity: &'f ID,
}

fn schedule_join<'f, P, T, OP, ID>(producer: P, reducer: &ReduceCallback<'f, OP, ID>) -> T
where
    P: Producer<Item = T>,
    T: Send,
    OP: Fn(T, T) -> T + Sync + Send,
    ID: Fn() -> T + Send + Sync,
{
    if producer.should_be_divided() {
        let (left, right) = producer.divide();
        let (left_r, right_r) = rayon::join(
            || schedule_join(left, reducer),
            || schedule_join(right, reducer),
        );
        (reducer.op)(left_r, right_r)
    } else {
        producer.fold((reducer.identity)(), reducer.op)
    }
}

impl<'f, T, OP, ID> ProducerCallback<T> for ReduceCallback<'f, OP, ID>
where
    T: Send,
    OP: Fn(T, T) -> T + Sync + Send,
    ID: Fn() -> T + Send + Sync,
{
    type Output = T;
    fn call<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>,
    {
        schedule_join(producer, &self)
    }
}

pub trait ParallelIterator: Sized {
    type Item: Send;
    type Controlled;
    //TODO: we did not need a power for previewable
    //do we really need them here ?
    //it is only needed for SPECIALIZATION,
    //so is there a method which is implemented for everyone but
    //where implementations differ based on power ?
    type Enumerable;
    /// Use rayon's steals reducing scheduling policy.
    fn rayon(self, limit: usize) -> Rayon<Self> {
        Rayon {
            base: self,
            reset_counter: limit,
        }
    }
    /// Turn back into a sequential iterator.
    /// Must be called just before the final reduction.
    fn sequential(self) -> Sequential<Self> {
        Sequential { base: self }
    }
    /// Turn back an adaptive reducer.
    /// Must be called just before the final reduction.
    fn adaptive(self) -> Adaptive<Self> {
        Adaptive { base: self }
    }
    fn for_each<OP>(self, op: OP)
    where
        OP: Fn(Self::Item) + Sync + Send,
    {
        self.map(op).reduce(|| (), |_, _| ())
    }
    fn even_levels(self) -> EvenLevels<Self> {
        EvenLevels { base: self }
    }
    fn join_policy(self, limit: usize) -> JoinPolicy<Self> {
        JoinPolicy { base: self, limit }
    }
    fn map<R, F>(self, op: F) -> Map<Self, F>
    where
        F: Fn(Self::Item) -> R + Send + Sync,
    {
        Map { base: self, op }
    }

    fn reduce_with<OP>(self, op: OP) -> Option<Self::Item>
    where
        OP: Fn(Self::Item, Self::Item) -> Self::Item + Sync + Send,
    {
        self.map(|i| Some(i)).reduce(
            || None,
            |o1, o2| {
                if let Some(r1) = o1 {
                    if let Some(r2) = o2 {
                        Some(op(r1, r2))
                    } else {
                        Some(r1)
                    }
                } else {
                    o2
                }
            },
        )
    }

    fn reduce<OP, ID>(self, identity: ID, op: OP) -> Self::Item
    where
        OP: Fn(Self::Item, Self::Item) -> Self::Item + Sync + Send,
        ID: Fn() -> Self::Item + Send + Sync,
    {
        let reduce_cb = ReduceCallback {
            op: &op,
            identity: &identity,
        };
        self.with_producer(reduce_cb)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>;
}

pub trait EnumerableParallelIterator: ParallelIterator {
    fn zip<I>(self, other: I) -> Zip<Self, I>
    where
        I: ParallelIterator<Controlled = True, Enumerable = True>,
    {
        Zip { a: self, b: other }
    }
}

pub trait PreviewableParallelIterator: ParallelIterator {
    fn merge<I>(self, other: I) -> Merge<Self, I>
    where
        I: PreviewableParallelIterator<Item = Self::Item>,
        Self::Item: Ord,
    {
        unimplemented!()
    }
}

impl<I> EnumerableParallelIterator for I where I: ParallelIterator<Enumerable = True> {}

pub trait IntoParallelIterator {
    type Item: Send;
    type Iter: ParallelIterator<Item = Self::Item>;
    fn into_par_iter(self) -> Self::Iter;
}

pub trait IntoParallelRefIterator<'data> {
    /// The type of the parallel iterator that will be returned.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    /// This will typically be an `&'data T` reference type.
    type Item: Send + 'data;

    /// Converts `self` into a parallel iterator.
    fn par_iter(&'data self) -> Self::Iter;
}

impl<'data, I: 'data + ?Sized> IntoParallelRefIterator<'data> for I
where
    &'data I: IntoParallelIterator,
{
    type Iter = <&'data I as IntoParallelIterator>::Iter;
    type Item = <&'data I as IntoParallelIterator>::Item;

    fn par_iter(&'data self) -> Self::Iter {
        self.into_par_iter()
    }
}
