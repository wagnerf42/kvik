use crate::adaptive::Adaptive;
use crate::adaptors::{
    even_levels::EvenLevels, filter::Filter, flat_map::FlatMap, map::Map, rayon_policy::Rayon,
    size_limit::SizeLimit,
};
use crate::cap::Cap;
use crate::composed::Composed;
use crate::composed_counter::ComposedCounter;
use crate::composed_size::ComposedSize;
use crate::composed_task::ComposedTask;
use crate::fold::Fold;
use crate::join_context_policy::JoinContextPolicy;
use crate::lower_bound::LowerBound;
use crate::merge::Merge;
use crate::sequential::Sequential;
use crate::small_channel::small_channel;
use crate::upper_bound::UpperBound;
use crate::wrap::Wrap;
use crate::zip::Zip;
use crate::Try;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicIsize};

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

struct ReduceCallback<OP, ID> {
    op: OP,
    identity: ID,
}

struct TryReduceCallback<OP, ID> {
    op: OP,
    identity: ID,
}

fn schedule_join_try_reduce<P, T, OP, ID>(
    mut producer: P,
    reducer: &TryReduceCallback<OP, ID>,
    stop: &AtomicBool,
) -> P::Item
where
    P: Producer,
    OP: Fn(T, T) -> P::Item + Sync + Send,
    ID: Fn() -> T + Sync + Send,
    P::Item: Try<Ok = T> + Send,
{
    if stop.load(Ordering::Relaxed) {
        P::Item::from_ok((reducer.identity)())
    } else {
        if producer.should_be_divided() {
            let (left, right) = producer.divide();
            let (left_result, right_result) = rayon::join(
                || schedule_join_try_reduce(left, reducer, stop),
                || schedule_join_try_reduce(right, reducer, stop),
            );
            match left_result.into_result() {
                Ok(left_ok) => match right_result.into_result() {
                    Ok(right_ok) => {
                        // TODO: should we also check the boolean here ?
                        let final_result = (reducer.op)(left_ok, right_ok);
                        match final_result.into_result() {
                            Ok(f) => P::Item::from_ok(f),
                            Err(e) => {
                                stop.store(true, Ordering::Relaxed);
                                P::Item::from_error(e)
                            }
                        }
                    }
                    Err(e) => {
                        stop.store(true, Ordering::Relaxed);
                        P::Item::from_error(e)
                    }
                },
                Err(e) => {
                    stop.store(true, Ordering::Relaxed);
                    P::Item::from_error(e)
                }
            }
        } else {
            let new_id = (reducer.identity)();
            try_fold(&mut producer, new_id, |t, i| match i.into_result() {
                Ok(t2) => (reducer.op)(t, t2),
                Err(e) => {
                    stop.store(true, Ordering::Relaxed);
                    P::Item::from_error(e)
                }
            })
        }
    }
}

fn schedule_depjoin<'f, P, T, OP, ID>(producer: P, reducer: &ReduceCallback<OP, ID>) -> T
where
    P: Producer<Item = T>,
    T: Send,
    OP: Fn(T, T) -> T + Sync + Send,
    ID: Fn() -> T + Send + Sync,
{
    if producer.should_be_divided() {
        let cleanup = AtomicBool::new(false);
        let (sender, receiver) = small_channel();
        let (sender1, receiver1) = small_channel();
        let (left, right) = producer.divide();
        let (left_r, right_r) = rayon::join(
            || {
                let my_result = schedule_depjoin(left, reducer);
                let last = cleanup.swap(true, Ordering::SeqCst);
                if last {
                    let his_result = receiver.recv().expect("receiving depjoin failed");
                    Some((reducer.op)(my_result, his_result))
                } else {
                    sender1.send(my_result);
                    None
                }
            },
            || {
                let my_result = schedule_depjoin(right, reducer);
                let last = cleanup.swap(true, Ordering::SeqCst);
                if last {
                    let his_result = receiver1.recv().expect("receiving1 depjoin failed");
                    Some((reducer.op)(his_result, my_result))
                } else {
                    sender.send(my_result);
                    None
                }
            },
        );
        left_r.or(right_r).unwrap()
    } else {
        producer.fold((reducer.identity)(), &reducer.op)
    }
}

impl<T, OP, ID> ProducerCallback<T> for ReduceCallback<OP, ID>
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
        schedule_depjoin(producer, &self)
    }
}

impl<I, T, OP, ID> ProducerCallback<I> for TryReduceCallback<OP, ID>
where
    OP: Fn(T, T) -> I + Sync + Send,
    ID: Fn() -> T + Sync + Send,
    I: Try<Ok = T> + Send,
{
    type Output = I;
    fn call<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = I>,
    {
        let stop = AtomicBool::new(false);
        let sizes = std::iter::successors(Some(rayon::current_num_threads()), |s| {
            Some(s.saturating_mul(2))
        });
        // let's get a sequential iterator of producers of increasing sizes
        let producers = sizes.scan(Some(producer), |p, s| {
            let remaining_producer = p.take().unwrap();
            let (_, upper_bound) = remaining_producer.size_hint();
            let capped_size = if let Some(bound) = upper_bound {
                if bound == 0 {
                    return None;
                } else {
                    s.min(bound)
                }
            } else {
                s
            };
            let (left, right) = remaining_producer.divide_at(capped_size);
            *p = Some(right);
            Some(left)
        });
        try_fold(
            &mut producers.map(|p| schedule_join_try_reduce(p, &self, &stop)),
            (self.identity)(),
            |previous_ok, current_result| match current_result.into_result() {
                Ok(r) => (self.op)(previous_ok, r),
                Err(e) => Try::from_error(e),
            },
        )
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
    fn upper_bound(self, limit: u32) -> UpperBound<Self> {
        UpperBound { base: self, limit }
    }
    /// This policy controls the division of the producer inside it.
    /// It will *force* division of the producer iff:
    ///     The depth of that producer in the binary tree of tasks is less than or equal to the limit.
    fn lower_bound(self, limit: u32) -> LowerBound<Self> {
        LowerBound { base: self, limit }
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
        let reduce_cb = ReduceCallback { op, identity };
        self.with_producer(reduce_cb)
    }

    fn test_reduce<OP, ID>(self, identity: ID, op: OP) -> Self::Item
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
        let reducer = TryReduceCallback { identity, op };
        self.with_producer(reducer)
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

/// we need to re-implement it because it's not the real trait
pub(crate) fn try_fold<I, B, F, R>(iterator: &mut I, init: B, mut f: F) -> R
where
    F: FnMut(B, I::Item) -> R,
    R: Try<Ok = B>,
    I: Iterator,
{
    let mut accum = init;
    while let Some(x) = iterator.next() {
        let accum_value = f(accum, x);
        match accum_value.into_result() {
            Ok(e) => {
                accum = e;
            }
            Err(e) => return Try::from_error(e),
        }
    }
    Try::from_ok(accum)
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

pub trait Reducer<Result>: Sync {
    fn identity(&self) -> Result;
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

struct ReduceConsumer<'f, OP, ID> {
    op: &'f OP,
    identity: &'f ID,
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
    fn identity(&self) -> Item {
        (self.identity)()
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
        schedule_join(producer, &self)
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

pub(crate) fn schedule_join<P, T, R>(producer: P, reducer: &R) -> T
where
    P: Producer<Item = T>,
    T: Send,
    R: Reducer<T>,
{
    if producer.should_be_divided() {
        let (left, right) = producer.divide();
        let (left_r, right_r) = rayon::join(
            || schedule_join(left, reducer),
            || schedule_join(right, reducer),
        );
        reducer.reduce(left_r, right_r)
    } else {
        producer.fold(reducer.identity(), |a, b| reducer.reduce(a, b))
    }
}
