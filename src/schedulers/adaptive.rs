use crate::prelude::*;
use crate::small_channel::small_channel;

//TODO: add block sizes ??
pub(crate) struct AdaptiveScheduler;

pub(crate) fn block_sizes() -> impl Iterator<Item = usize> {
    // TODO: cap
    std::iter::successors(Some(1), |old: &usize| {
        old.checked_shl(1).or(Some(std::usize::MAX))
    })
}

impl<P, R> Scheduler<P, R> for AdaptiveScheduler
where
    P: Producer,
    P::Item: Send,
    R: Reducer<P::Item>,
{
    fn schedule(&self, producer: P, reducer: &R) -> P::Item {
        let initial_output = reducer.identity();
        adaptive_scheduler(reducer, producer, initial_output)
    }
}

//TODO: should we really pass the reduce refs by refs ?
pub(crate) fn adaptive_scheduler<'f, T, P, R>(reducer: &R, producer: P, output: T) -> T
where
    T: Send,
    P: Producer<Item = T>,
    R: Reducer<T>,
{
    let (sender, receiver) = small_channel();
    let (left_result, maybe_right_result): (T, Option<T>) = rayon::join_context(
        |_| match block_sizes()
            .take_while(|_| !sender.receiver_is_waiting())
            .try_fold((producer, output), |(mut producer, output), s| {
                //TODO: is this the right way to test for the end ?
                if producer.sizes().1 == Some(0) {
                    Err(producer.fold(output, |a, b| reducer.reduce(a, b)))
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
