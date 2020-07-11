use crate::adaptive::AdaptiveProducer;
use crate::prelude::*;
use crate::small_channel::small_channel;
use crate::Blocked;

pub(crate) fn block_sizes() -> impl Iterator<Item = usize> {
    // TODO: cap
    std::iter::successors(Some(1), |old: &usize| {
        old.checked_shl(1).or(Some(std::usize::MAX))
    })
}

pub(crate) fn schedule_adaptive<P, T, R>(producer: P, reducer: &R) -> T
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
pub(crate) fn adaptive_scheduler<'f, T, P, R>(reducer: &R, producer: P, output: T) -> T
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
