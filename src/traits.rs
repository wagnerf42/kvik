use crate::adaptors::{
    bound_depth::BoundDepth,
    cap::Cap,
    composition::Composed,
    composition::ComposedCounter,
    composition::ComposedSize,
    composition::ComposedTask,
    even_levels::EvenLevels,
    filter::Filter,
    flat_map::FlatMap,
    fold::Fold,
    force_depth::ForceDepth,
    join_context_policy::JoinContextPolicy,
    map::Map,
    merge::Merge,
    rayon_policy::Rayon,
    scheduler_adaptors::{Adaptive, DepJoin, Sequential},
    size_limit::SizeLimit,
    zip::Zip,
};
use crate::schedulers::schedule_join;
use crate::try_fold::try_fold;
use crate::wrap::Wrap;
use crate::Try;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

#[cfg(feature = "logs")]
use crate::adaptors::log::Log;
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
    fn sizes(&self) -> (usize, Option<usize>);
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
    fn scheduler<'r, P, R>(&self) -> &'r dyn Fn(P, &'r R) -> P::Item
    where
        P: Producer,
        P::Item: Send,
        R: Reducer<P::Item>,
    {
        &schedule_join
    }
}

// impl<I, T, OP, ID> ProducerCallback<I> for TryReduceCallback<OP, ID>
// where
//     OP: Fn(T, T) -> I + Sync + Send,
//     ID: Fn() -> T + Sync + Send,
//     I: Try<Ok = T> + Send,
// {
//     type Output = I;
//     fn call<P>(self, producer: P) -> Self::Output
//     where
//         P: Producer<Item = I>,
//     {
//         let stop = AtomicBool::new(false);
//         let sizes = std::iter::successors(Some(rayon::current_num_threads()), |s| {
//             Some(s.saturating_mul(2))
//         });
//         // let's get a sequential iterator of producers of increasing sizes
//         let producers = sizes.scan(Some(producer), |p, s| {
//             let remaining_producer = p.take().unwrap();
//             let (_, upper_bound) = remaining_producer.size_hint();
//             let capped_size = if let Some(bound) = upper_bound {
//                 if bound == 0 {
//                     return None;
//                 } else {
//                     s.min(bound)
//                 }
//             } else {
//                 s
//             };
//             let (left, right) = remaining_producer.divide_at(capped_size);
//             *p = Some(right);
//             Some(left)
//         });
//         unimplemented!()
//         //        try_fold(
//         //            &mut producers.map(|p| schedule_join_try_reduce(p, &self, &stop)),
//         //            (self.identity)(),
//         //            |previous_ok, current_result| match current_result.into_result() {
//         //                Ok(r) => (self.op)(previous_ok, r),
//         //                Err(e) => Try::from_error(e),
//         //            },
//         //        )
//     }
// }

pub trait ParallelIterator: Sized {
    type Item: Send;
    type Controlled;
    //TODO: we did not need a power for previewable
    //do we really need them here ?
    //it is only needed for SPECIALIZATION,
    //so is there a method which is implemented for everyone but
    //where implementations differ based on power ?
    type Enumerable;

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        let c = ConsumerCallback(consumer);
        self.with_producer(c)
    }

    /// Try to cap tasks to a given number.
    /// We cannot ensure it because we cap the divisions
    /// but the user might create tasks we have no control on.
    fn cap(self, limit: &AtomicIsize) -> Cap<Self> {
        Cap { base: self, limit }
    }
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
    /// Turn on depjoin scheduling policy.
    fn depjoin(self) -> DepJoin<Self> {
        DepJoin { base: self }
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
    /// Don't allow any task of size lower than given limit
    /// to go parallel.
    fn size_limit(self, limit: usize) -> SizeLimit<Self> {
        SizeLimit { base: self, limit }
    }
    /// This policy controls the division of the producer inside it.
    /// It will veto the division of a producer iff:
    ///     The depth of that producer in the binary tree of tasks is equal to limit.
    fn bound_depth(self, limit: u32) -> BoundDepth<Self> {
        BoundDepth { base: self, limit }
    }
    /// This policy controls the division of the producer inside it.
    /// It will *force* division of the producer iff:
    ///     The depth of that producer in the binary tree of tasks is less than or equal to the limit.
    fn force_depth(self, limit: u32) -> ForceDepth<Self> {
        ForceDepth { base: self, limit }
    }

    /// This policy controls the division of the producer inside (before) it.
    /// It will veto the division of the base producer iff:
    ///     The right child of any node is not stolen
    fn join_context_policy(self, limit: u32) -> JoinContextPolicy<Self> {
        JoinContextPolicy { base: self, limit }
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
        let consumer = ReduceConsumer {
            op: &op,
            identity: &identity,
        };
        self.drive(consumer)
    }

    fn composed(self) -> Composed<Self> {
        Composed { base: self }
    }

    fn composed_counter(self, threshold: usize) -> ComposedCounter<Self> {
        ComposedCounter {
            base: self,
            counter: std::sync::atomic::AtomicU64::new(0),
            threshold,
        }
    }

    fn composed_task(self) -> ComposedTask<Self> {
        ComposedTask {
            base: self,
            counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn composed_size(self, reset_counter: usize) -> ComposedSize<Self> {
        ComposedSize {
            base: self,
            reset_counter,
        }
    }
    fn flat_map<F, PI>(self, map_op: F) -> FlatMap<Self, F>
    where
        F: Fn(Self::Item) -> PI + Sync + Send,
        PI: IntoParallelIterator,
    {
        FlatMap { base: self, map_op }
    }

    fn filter<F>(self, filter: F) -> Filter<Self, F>
    where
        F: Fn(&Self::Item) -> bool,
    {
        Filter { base: self, filter }
    }

    fn fold<T, ID, F>(self, identity: ID, fold_op: F) -> Fold<Self, ID, F>
    where
        F: Fn(T, Self::Item) -> T + Sync + Send,
        ID: Fn() -> T + Sync + Send,
        T: Send,
    {
        Fold {
            base: self,
            id: identity,
            fold: fold_op,
        }
    }

    fn min_by<F>(self, compare: F) -> Option<Self::Item>
    where
        F: Fn(&Self::Item, &Self::Item) -> std::cmp::Ordering + Sync,
    {
        // Rewritten with fold then reduce to avoid using a map
        self.fold(
            || None,
            |a, b| {
                if let Some(a) = a {
                    match compare(&a, &b) {
                        std::cmp::Ordering::Greater => Some(b),
                        _ => Some(a),
                    }
                } else {
                    Some(b)
                }
            },
        )
        .reduce(
            || None,
            |a, b| {
                if a.is_none() {
                    b
                } else if b.is_none() {
                    a
                } else {
                    let (a, b) = (a.unwrap(), b.unwrap());
                    match compare(&a, &b) {
                        std::cmp::Ordering::Greater => Some(b),
                        _ => Some(a),
                    }
                }
            },
        )
    }

    #[cfg(feature = "logs")]
    fn log(self, name: &'static str) -> Log<Self> {
        Log { base: self, name }
    }

    fn collect<T: FromParallelIterator<Self::Item>>(self) -> T
    where
        <Self as ParallelIterator>::Item: Sync,
    {
        T::from_par_iter(self)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>;
}

// we need a new trait to specialize try_reduce
pub trait TryReducible: ParallelIterator {
    fn try_reduce<T, OP, ID>(self, identity: ID, op: OP) -> Self::Item
    where
        OP: Fn(T, T) -> Self::Item + Sync + Send,
        ID: Fn() -> T + Sync + Send,
        Self::Item: Try<Ok = T>;
    fn all<P>(self, predicate: P) -> bool
    where
        Self: ParallelIterator<Controlled = True>,
        P: Fn(Self::Item) -> bool + Sync + Send,
    {
        match self
            .map(|e| if predicate(e) { Ok(()) } else { Err(()) })
            .try_reduce(|| (), |_, _| Ok(()))
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

impl<I> TryReducible for I
where
    I: ParallelIterator<Controlled = True>,
{
    fn try_reduce<T, OP, ID>(self, identity: ID, op: OP) -> Self::Item
    where
        OP: Fn(T, T) -> Self::Item + Sync + Send,
        ID: Fn() -> T + Sync + Send,
        Self::Item: Try<Ok = T>,
    {
        let stop = AtomicBool::new(false);
        let consumer = TryReduceConsumer {
            op: &op,
            identity: &identity,
            stop: &stop,
        };
        self.drive(consumer)
    }
}

pub trait EnumerableParallelIterator: ParallelIterator {
    /// zip two parallel iterators.
    ///
    /// Example:
    ///
    /// ```
    /// use rayon_try_fold::prelude::*;
    /// let mut v = vec![0; 5];
    /// v.par_iter_mut().zip(0..5).for_each(|(r, i)| *r = i);
    /// assert_eq!(v, vec![0, 1, 2, 3, 4])
    /// ```
    fn zip<I>(self, other: I) -> Zip<Self, I::Iter>
    where
        I: IntoParallelIterator,
        I::Iter: ParallelIterator<Controlled = True, Enumerable = True>,
    {
        Zip {
            a: self,
            b: other.into_par_iter(),
        }
    }
}

pub trait PreviewableParallelIterator: ParallelIterator {
    fn merge<I>(self, other: I) -> Merge<Self, I>
    where
        I: PreviewableParallelIterator<Item = Self::Item>,
        Self::Item: Ord,
    {
        Merge { a: self, b: other }
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

pub trait IntoParallelRefMutIterator<'data> {
    /// The type of iterator that will be created.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that will be produced; this is typically an
    /// `&'data mut T` reference.
    type Item: Send + 'data;

    /// Creates the parallel iterator from `self`.
    fn par_iter_mut(&'data mut self) -> Self::Iter;
}

impl<'data, I: 'data + ?Sized> IntoParallelRefMutIterator<'data> for I
where
    &'data mut I: IntoParallelIterator,
{
    type Iter = <&'data mut I as IntoParallelIterator>::Iter;
    type Item = <&'data mut I as IntoParallelIterator>::Item;

    fn par_iter_mut(&'data mut self) -> Self::Iter {
        self.into_par_iter()
    }
}

pub trait FromParallelIterator<A: Sync + Send> {
    fn from_par_iter<T: ParallelIterator<Item = A>>(iter: T) -> Self;
}

impl<A: Sync + Send> FromParallelIterator<A> for Vec<A> {
    fn from_par_iter<T: ParallelIterator<Item = A>>(iter: T) -> Self {
        use std::collections::LinkedList;

        let l = iter
            .fold(Vec::new, |mut v, e| {
                v.push(e);
                v
            })
            .map(|v| std::iter::once(v).collect::<LinkedList<Vec<T::Item>>>())
            .reduce(LinkedList::new, |mut l1, mut l2| {
                l1.append(&mut l2);
                l1
            });

        let mut iter_list = l.into_iter();
        let first = iter_list.next();

        if let Some(first) = first {
            iter_list.fold(first, |mut v, mut v2| {
                v.append(&mut v2);
                v
            })
        } else {
            Vec::new()
        }
    }
}

//TODO: we could separate folder and reducer to remove one pointer level
pub trait Reducer<Result>: Sync {
    // we need this guy for the adaptive scheduler
    fn identity(&self) -> Result;
    fn fold<P>(&self, producer: P) -> Result
    where
        P: Producer<Item = Result>;
    fn reduce(&self, left: Result, right: Result) -> Result;
}

pub trait Consumer<Item>: Send + Sync + Sized + Clone {
    type Result: Send;
    type Reducer: Reducer<Self::Result>;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>;
    fn to_reducer(self) -> Self::Reducer;
}

pub(crate) struct ReduceConsumer<'f, OP, ID> {
    pub(crate) op: &'f OP,
    pub(crate) identity: &'f ID,
}

impl<'f, OP, ID> Clone for ReduceConsumer<'f, OP, ID> {
    fn clone(&self) -> Self {
        ReduceConsumer {
            op: self.op,
            identity: self.identity,
        }
    }
}

impl<'f, Item, OP, ID> Reducer<Item> for ReduceConsumer<'f, OP, ID>
where
    OP: Fn(Item, Item) -> Item + Send + Sync,
    ID: Fn() -> Item + Send + Sync,
{
    //TODO: we will need a partial fold
    fn identity(&self) -> Item {
        (self.identity)()
    }
    fn fold<P>(&self, producer: P) -> Item
    where
        P: Producer<Item = Item>,
    {
        producer.fold((self.identity)(), self.op)
    }
    fn reduce(&self, left: Item, right: Item) -> Item {
        (self.op)(left, right)
    }
}

impl<'f, Item, OP, ID> Consumer<Item> for ReduceConsumer<'f, OP, ID>
where
    Item: Send,
    OP: Fn(Item, Item) -> Item + Send + Sync,
    ID: Fn() -> Item + Send + Sync,
{
    type Result = Item;
    type Reducer = Self;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let scheduler = producer.scheduler();
        scheduler(producer, &self)
    }
    fn to_reducer(self) -> Self::Reducer {
        self
    }
}

struct ConsumerCallback<C>(C);

impl<T, C> ProducerCallback<T> for ConsumerCallback<C>
where
    C: Consumer<T>,
{
    type Output = C::Result;
    fn call<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>,
    {
        self.0.consume_producer(producer)
    }
}

// try_reduce consumer
pub(crate) struct TryReduceConsumer<'f, OP, ID> {
    pub(crate) op: &'f OP,
    pub(crate) identity: &'f ID,
    pub(crate) stop: &'f AtomicBool,
}

impl<'f, OP, ID> Clone for TryReduceConsumer<'f, OP, ID> {
    fn clone(&self) -> Self {
        TryReduceConsumer {
            op: self.op,
            identity: self.identity,
            stop: self.stop,
        }
    }
}

impl<'f, T, Item, OP, ID> Reducer<Item> for TryReduceConsumer<'f, OP, ID>
where
    OP: Fn(T, T) -> Item + Send + Sync,
    ID: Fn() -> T + Send + Sync,
    Item: Try<Ok = T>,
{
    //TODO: we will need a partial fold
    fn identity(&self) -> Item {
        Item::from_ok((self.identity)())
    }
    fn fold<P>(&self, mut producer: P) -> Item
    where
        P: Producer<Item = Item>,
    {
        if self.stop.load(Ordering::Relaxed) {
            self.identity()
        } else {
            let folded = try_fold(
                &mut producer,
                self.identity().into_result().ok().unwrap(),
                |a, b| match b.into_result() {
                    Err(b_err) => Item::from_error(b_err),
                    Ok(b_ok) => (self.op)(a, b_ok),
                },
            );
            match folded.into_result() {
                Err(folded_err) => {
                    self.stop.store(true, Ordering::Relaxed);
                    Item::from_error(folded_err)
                }
                Ok(folded_ok) => Item::from_ok(folded_ok),
            }
        }
    }
    fn reduce(&self, left: Item, right: Item) -> Item {
        match left.into_result() {
            Err(left_error) => Item::from_error(left_error),
            Ok(left_ok) => match right.into_result() {
                Err(right_error) => Item::from_error(right_error),
                Ok(right_ok) => {
                    let final_result = (self.op)(left_ok, right_ok);
                    match final_result.into_result() {
                        Err(final_err) => {
                            self.stop.store(true, Ordering::Relaxed);
                            Item::from_error(final_err)
                        }
                        Ok(final_ok) => Item::from_ok(final_ok),
                    }
                }
            },
        }
    }
}

impl<'f, T, Item, OP, ID> Consumer<Item> for TryReduceConsumer<'f, OP, ID>
where
    OP: Fn(T, T) -> Item + Send + Sync,
    ID: Fn() -> T + Send + Sync,
    Item: Try<Ok = T> + Send,
{
    type Result = Item;
    type Reducer = Self;
    fn consume_producer<P>(self, producer: P) -> Self::Result
    where
        P: Producer<Item = Item>,
    {
        let scheduler = producer.scheduler();
        scheduler(producer, &self)
    }
    fn to_reducer(self) -> Self::Reducer {
        self
    }
}
