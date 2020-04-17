use crate::map::Map;
pub trait ProducerCallback<T> {
    /// The type of value returned by this callback. Analogous to
    /// [`Output` from the `FnOnce` trait][Output].
    ///
    /// [Output]: https://doc.rust-lang.org/std/ops/trait.FnOnce.html#associatedtype.Output
    type Output;

    /// Invokes the callback with the given producer as argument. The
    /// key point of this trait is that this method is generic over
    /// `P`, and hence implementors must be defined for any producer.
    fn call<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>;
}

pub trait Producer: Send + Sized + Iterator {
    // TODO: think about the index
    fn divide(self) -> (Self, Self);
}

struct ReduceCallback<'f, OP, ID> {
    op: &'f OP,
    identity: &'f ID,
}

impl<'f, T, OP, ID> ProducerCallback<T> for ReduceCallback<'f, OP, ID>
where
    OP: Fn(T, T) -> T + Sync + Send,
    ID: Fn() -> T + Send + Sync,
{
    type Output = T;
    fn call<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>,
    {
        producer.fold((self.identity)(), self.op)
    }
}

//TODO: power ?
pub trait ParallelIterator: Sized {
    type Item;
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
    type Item;
    type Iter: ParallelIterator<Item = Self::Item>;
    fn into_par_iter(self) -> Self::Iter;
}
