//! Adaptive reductions

use crate::prelude::*;
use crate::small_channel::small_channel;
use crate::Blocked;

pub(crate) trait AdaptiveProducer: Producer {
    fn completed(&self) -> bool;
    fn partial_fold<B, F>(&mut self, init: B, fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B;
}

fn block_sizes() -> impl Iterator<Item = usize> {
    // TODO: cap
    std::iter::successors(Some(1), |old| Some(2 * old))
}

pub struct Adaptive<I> {
    pub(crate) base: I,
}

//TODO: is this always the same ?
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
        let blocked_producer = Blocked::new(producer);
        let output = (self.identity)();
        adaptive_scheduler(self, blocked_producer, output)
    }
}

fn adaptive_scheduler<'f, T, OP, ID, P>(
    reducer: &ReduceCallback<'f, OP, ID>,
    producer: P,
    output: T,
) -> T
where
    T: Send,
    OP: Fn(T, T) -> T + Sync + Send,
    ID: Fn() -> T + Send + Sync,
    P: AdaptiveProducer<Item = T>,
{
    let (sender, receiver) = small_channel();
    let (left_result, maybe_right_result): (T, Option<T>) = rayon::join_context(
        |_| match block_sizes()
            .take_while(|_| !sender.receiver_is_waiting())
            .try_fold((producer, output), |(mut producer, output), s| {
                //TODO: is this the right way to test for the end ?
                if producer.completed() {
                    Err(output)
                } else {
                    let new_output = producer.partial_fold(output, reducer.op, s);
                    Ok((producer, new_output))
                }
            }) {
            Ok((remaining_producer, output)) => {
                // we are being stolen. Let's give something if what is left is big enough.
                if remaining_producer.should_be_divided() {
                    let (my_half, his_half) = remaining_producer.divide();
                    sender.send(Some(his_half));
                    adaptive_scheduler(reducer, my_half, output)
                } else {
                    sender.send(None);
                    remaining_producer.fold(output, reducer.op)
                }
            }
            Err(output) => {
                // all is completed, cancel stealer's task.
                sender.send(None);
                output
            }
        },
        |c| {
            if c.migrated() {
                receiver
                    .recv()
                    .expect("receiving adaptive producer failed")
                    .map(|producer| adaptive_scheduler(reducer, producer, (reducer.identity)()))
            } else {
                None
            }
        },
    );

    if let Some(right_result) = maybe_right_result {
        (reducer.op)(left_result, right_result)
    } else {
        left_result
    }
}

impl<I> ParallelIterator for Adaptive<I>
where
    I: ParallelIterator,
{
    type Item = I::Item;
    //TODO: why isnt this the default function ?
    fn reduce<OP, ID>(self, identity: ID, op: OP) -> Self::Item
    where
        OP: Fn(Self::Item, Self::Item) -> Self::Item + Sync + Send,
        ID: Fn() -> Self::Item + Sync + Send,
    {
        let reduce_cb = ReduceCallback {
            op: &op,
            identity: &identity,
        };
        self.with_producer(reduce_cb)
    }
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        self.base.with_producer(callback)
    }
}
