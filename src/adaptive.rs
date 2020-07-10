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

pub(crate) fn block_sizes() -> impl Iterator<Item = usize> {
    // TODO: cap
    std::iter::successors(Some(1), |old: &usize| {
        old.checked_shl(1).or(Some(std::usize::MAX))
    })
}

pub(crate) fn schedule_adapt<P, T, R>(producer: P, reducer: &R) -> T
where
    P: Producer<Item = T>,
    T: Send,
    R: Reducer<T>,
{
    let initial_output = reducer.identity();
    let blocked_producer = Blocked::new(producer);
    adaptive_scheduler(reducer, blocked_producer, initial_output)
}

//TODO: should we really pass the reduce refs by refs ?
fn adaptive_scheduler<'f, T, P, R>(reducer: &R, producer: P, output: T) -> T
where
    T: Send,
    P: AdaptiveProducer<Item = T>,
    R: Reducer<T>,
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
                    // TODO: remove closure ?
                    let new_output = producer.partial_fold(output, |a, b| reducer.reduce(a, b), s);
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
                    //TODO: remove closure ?
                    remaining_producer.fold(output, |a, b| reducer.reduce(a, b))
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
                let stolen_task = {
                    #[cfg(feature = "logs")]
                    {
                        use rayon_logs::subgraph;
                        subgraph("En attendant", 0, || {
                            receiver.recv().expect("receiving adaptive producer failed")
                        })
                    }
                    #[cfg(not(feature = "logs"))]
                    {
                        receiver.recv().expect("receiving adaptive producer failed")
                    }
                };
                stolen_task
                    .map(|producer| adaptive_scheduler(reducer, producer, reducer.identity()))
            } else {
                None
            }
        },
    );

    if let Some(right_result) = maybe_right_result {
        reducer.reduce(left_result, right_result)
    } else {
        left_result
    }
}

struct Worker<'f, S, C, D, W, SD> {
    state: S,
    completed: &'f C,
    divide: &'f D,
    should_divide: &'f SD,
    work: &'f W,
}

impl<'f, S, C, D, W, SD> Iterator for Worker<'f, S, C, D, W, SD>
where
    W: Fn(&mut S, usize) + Sync,
    C: Fn(&S) -> bool + Sync,
{
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
    fn fold<B, F>(mut self, init: B, _f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        (self.work)(&mut self.state, std::usize::MAX);
        init
    }
}

impl<'f, S, C, D, W, SD> Divisible for Worker<'f, S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(&mut S, usize) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    type Controlled = False;
    fn should_be_divided(&self) -> bool {
        (self.should_divide)(&self.state)
    }
    fn divide(self) -> (Self, Self) {
        let (left, right) = (self.divide)(self.state);
        (
            Worker {
                state: left,
                completed: self.completed,
                divide: self.divide,
                should_divide: self.should_divide,
                work: self.work,
            },
            Worker {
                state: right,
                completed: self.completed,
                should_divide: self.should_divide,
                divide: self.divide,
                work: self.work,
            },
        )
    }
    fn divide_at(self, _index: usize) -> (Self, Self) {
        panic!("should never be called")
    }
}

impl<'f, S, C, D, W, SD> Producer for Worker<'f, S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(&mut S, usize) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    fn sizes(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
    fn preview(&self, _index: usize) -> Self::Item {
        panic!("you cannot preview a Worker")
    }
}

impl<'f, S, C, D, W, SD> AdaptiveProducer for Worker<'f, S, C, D, W, SD>
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(&mut S, usize) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    fn completed(&self) -> bool {
        (self.completed)(&self.state)
    }
    fn partial_fold<B, F>(&mut self, init: B, _fold_op: F, limit: usize) -> B
    where
        B: Send,
        F: Fn(B, Self::Item) -> B,
    {
        (self.work)(&mut self.state, limit);
        init
    }
}

pub fn work<S, C, D, W, SD>(init: S, completed: C, divide: D, work: W, should_be_divided: SD)
where
    S: Send,
    C: Fn(&S) -> bool + Sync,
    D: Fn(S) -> (S, S) + Sync,
    W: Fn(&mut S, usize) + Sync,
    SD: Fn(&S) -> bool + Sync,
{
    let worker = Worker {
        state: init,
        completed: &completed,
        divide: &divide,
        work: &work,
        should_divide: &should_be_divided,
    };
    let identity = || ();
    let op = |_, _| ();
    let reducer = crate::traits::ReduceConsumer {
        op: &op,
        identity: &identity,
    };
    adaptive_scheduler(&reducer, worker, ());
}
