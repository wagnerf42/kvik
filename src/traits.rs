use crate::adaptive::Adaptive;
use crate::map::Map;
use crate::sequential::Sequential;
use std::iter::Sum;

pub trait ProducerCallback<T> {
    type Output;

    // TODO: what if we don't borrow self ?
    fn call<P>(&self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>;
}

pub trait Producer: Send + Sized + Iterator {
    // TODO: think about the index
    fn divide(self) -> (Self, Self);
    fn should_be_divided(&self) -> bool;
}

struct ReduceCallback<'f, OP, ID> {
    op: &'f OP,
    identity: &'f ID,
}

impl<'f, T, OP, ID> ProducerCallback<T> for ReduceCallback<'f, OP, ID>
where
    T: Send,
    OP: Fn(T, T) -> T + Sync + Send,
    ID: Fn() -> T + Send + Sync,
{
    type Output = T;
    fn call<P>(&self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>,
    {
        if producer.should_be_divided() {
            let (left, right) = producer.divide();
            let (left_r, right_r) = rayon::join(|| self.call(left), || self.call(right));
            (self.op)(left_r, right_r)
        } else {
            producer.fold((self.identity)(), self.op)
        }
    }
}

//TODO: power ?
pub trait ParallelIterator: Sized {
    type Item: Send;
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
    fn map<R, F>(self, op: F) -> Map<Self, F>
    where
        F: Fn(Self::Item) -> R + Send + Sync,
    {
        Map { base: self, op }
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
